//! observe: raw pane output stream. Oldest replays retained backlog, Now
//! delivers only new bytes. This is the minimal core of the future web
//! mirror; no HTTP is opened here.

use std::time::{Duration, Instant};

use oma::rmuxpoc;
use rmux_sdk::{Pane, PaneOutputChunk, PaneOutputStart};

/// Markers must not contain each other as substrings, or the no-replay
/// assertion hits the live marker itself (see M027).
const BACKLOG_MARKER: &str = "OMA-STREAM-BACKLOG";
const LIVE_MARKER: &str = "OMA-STREAM-LIVE";

#[tokio::main]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("poc-stream: {e}");
            if rmuxpoc::is_job_object_error(&e) {
                eprintln!("poc.skip=job-object");
            }
            std::process::ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let _report = rmuxpoc::gate()?;
    println!("poc.name=stream");
    println!("poc.os={}", std::env::consts::OS);

    let rmux = rmuxpoc::connect("stm").await?;
    rmuxpoc::assert_dedicated(rmux.endpoint())?;
    let name = rmuxpoc::poc_session_name("stm")?;
    println!("poc.session={}", name.as_str());

    let session =
        rmuxpoc::create_only(&rmux, name.clone(), rmuxpoc::interactive_shell_argv()).await?;
    let pane = session.pane(0, 0);
    let result = stream_inner(&pane).await;
    let _ = session.kill().await;
    println!("poc.kill_session=true");
    println!("poc.kill_server=false");
    result?;
    println!("poc.ok=true");
    Ok(())
}

async fn stream_inner(pane: &Pane) -> Result<(), String> {
    if !pane.exists().await.map_err(|e| e.to_string())? {
        return Err("pane 0.0 missing".into());
    }

    // Produce output first so the daemon retains it as backlog.
    echo(pane, BACKLOG_MARKER).await?;
    pane.expect_visible_text()
        .to_contain(BACKLOG_MARKER)
        .timeout(Duration::from_secs(15))
        .await
        .map_err(|e| format!("visible text missing {BACKLOG_MARKER}: {e}"))?;

    // Oldest: the retained backlog must replay through the stream.
    let mut stream = pane
        .output_stream_starting_at(PaneOutputStart::Oldest)
        .await
        .map_err(|e| format!("open Oldest: {e}"))?;
    let seen = collect_until(&mut stream, BACKLOG_MARKER, Duration::from_secs(15)).await?;
    println!("poc.stream.oldest.replay=true");
    println!(
        "poc.stream.oldest.chunks={} bytes={}",
        seen.chunks, seen.bytes
    );
    println!(
        "poc.stream.oldest.first_seq={:?}",
        seen.first_sequence
    );
    drop(stream);

    // Now: anchored after the newest retained output, so only new bytes.
    let mut live = pane
        .output_stream_starting_at(PaneOutputStart::Now)
        .await
        .map_err(|e| format!("open Now: {e}"))?;
    let echo_start = Instant::now();
    echo(pane, LIVE_MARKER).await?;
    let seen = collect_until(&mut live, LIVE_MARKER, Duration::from_secs(15)).await?;
    println!("poc.stream.now.live=true");
    println!("poc.stream.now.chunks={} bytes={}", seen.chunks, seen.bytes);
    if seen.text.contains(BACKLOG_MARKER) {
        return Err("Now stream replayed the old backlog marker".into());
    }
    println!("poc.stream.now.no_replay=true");
    println!(
        "poc.stream.now.echo_to_see_ms={}",
        echo_start.elapsed().as_millis()
    );
    drop(live);

    println!("poc.stream=raw-bytes");
    Ok(())
}

async fn echo(pane: &Pane, marker: &str) -> Result<(), String> {
    pane.send_text(format!("echo {marker}"))
        .await
        .map_err(|e| format!("send_text: {e}"))?;
    pane.send_key("Enter")
        .await
        .map_err(|e| format!("send_key Enter: {e}"))?;
    Ok(())
}

struct Collected {
    chunks: usize,
    bytes: usize,
    first_sequence: Option<u64>,
    text: String,
}

/// Read chunks until the marker shows up in the accumulated raw bytes or the
/// deadline passes. Bytes stay raw; the text copy is only for matching.
async fn collect_until(
    stream: &mut rmux_sdk::PaneOutputStream,
    marker: &str,
    deadline: Duration,
) -> Result<Collected, String> {
    let mut collected = Collected {
        chunks: 0,
        bytes: 0,
        first_sequence: None,
        text: String::new(),
    };
    let mut buf: Vec<u8> = Vec::new();
    let end = tokio::time::Instant::from_std(Instant::now() + deadline);
    while tokio::time::Instant::now() < end {
        let remain = end - tokio::time::Instant::now();
        let chunk = tokio::time::timeout(remain, stream.next())
            .await
            .map_err(|_| format!("stream read timed out before {marker}"))?
            .map_err(|e| format!("stream next: {e}"))?;
        let Some(chunk) = chunk else {
            return Err(format!("stream ended before {marker}"));
        };
        match chunk {
            PaneOutputChunk::Bytes { sequence, bytes } => {
                if collected.first_sequence.is_none() {
                    collected.first_sequence = Some(sequence);
                }
                buf.extend_from_slice(&bytes);
                collected.chunks += 1;
                collected.bytes += bytes.len();
                if find_ascii(&buf, marker.as_bytes()) {
                    collected.text = String::from_utf8_lossy(&buf).into_owned();
                    return Ok(collected);
                }
            }
            _ => {
                // Gap reports do not carry bytes; nothing to accumulate.
            }
        }
    }
    Err(format!(
        "marker {marker} not seen within deadline; got {} bytes",
        buf.len()
    ))
}

/// Substring search on raw bytes; the marker is plain ASCII.
fn find_ascii(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|w| w == needle)
}
