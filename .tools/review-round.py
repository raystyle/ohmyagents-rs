# /// script
# requires-python = ">=3.10"
# ///
"""四 agent 轮询 review 工作流：单轮执行器。

对 claude/codex/grok/kimi 并行委派 `oma task` review（产物走任务目录协议，
DONE 收件），收齐后按产物第一行的 `FINDINGS=N` 契约判定：
- 全家 N=0：本轮全绿，退出码 0（工作流可终止）
- 有任何 N>0：退出码 1（修复后跑下一轮）
- 有 agent 超时/失败：退出码 2

产物归档 `<project>/.ohmyagents/reviews/round<N>/<agent>.md` 供跨轮对照。

用法：uv run --script .tools/review-round.py <round> [--project PATH] [--timeout 1800]
"""
import argparse
import concurrent.futures
import re
import shutil
import subprocess
import sys
from pathlib import Path

AGENTS = ["claude", "codex", "grok", "kimi"]

REVIEW_PROMPT = (
    "review 当前仓库 src/ 与最近提交：找正确性、并发、边界、契约问题。"
    "范围：src/orch.rs、src/api.rs、src/server.rs、src/task.rs、src/main.rs、src/mcp.rs。"
    "产物契约：output.md 第一行必须是 FINDINGS=N（N=发现的问题条数，没发现问题写 0），"
    "之后按严重度列结论；确认没问题的方面也要列出。写完 output.md 最后创建空文件 DONE。"
)


def run_one(agent: str, project: Path, timeout: int) -> tuple[str, str, str]:
    """返回 (agent, status, detail)。status: ok | findings | timeout | error."""
    cmd = [
        "oma", "task", agent, REVIEW_PROMPT,
        "--timeout", str(timeout), "--project", str(project),
    ]
    try:
        r = subprocess.run(
            cmd, capture_output=True, text=True, timeout=timeout + 120,
            encoding="utf-8", errors="replace",
        )
    except subprocess.TimeoutExpired:
        return agent, "timeout", f"{agent}: oma task 超时"
    if r.returncode != 0:
        return agent, "error", (r.stderr or r.stdout or "").strip()[:400]
    # 从 stdout 抓 task.dir= 行定位产物。
    m = re.search(r"^task\.dir=(.+)$", r.stdout, re.M)
    if not m:
        return agent, "error", f"{agent}: stdout 无 task.dir 行"
    out_md = Path(m.group(1).strip()) / "output.md"
    text = out_md.read_text(encoding="utf-8", errors="replace") if out_md.exists() else ""
    fm = re.search(r"^FINDINGS=(\d+)\s*$", text, re.M)
    if not fm:
        return agent, "error", f"{agent}: 产物缺 FINDINGS=N 契约行"
    n = int(fm.group(1))
    return agent, ("ok" if n == 0 else "findings"), f"FINDINGS={n} → {out_md}"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("round", type=int)
    ap.add_argument("--project", default=".")
    ap.add_argument("--timeout", type=int, default=1800)
    args = ap.parse_args()
    project = Path(args.project).resolve()

    print(f"review.round={args.round} agents={','.join(AGENTS)} timeout={args.timeout}s")
    results = {}
    with concurrent.futures.ThreadPoolExecutor(max_workers=len(AGENTS)) as ex:
        futs = {ex.submit(run_one, a, project, args.timeout): a for a in AGENTS}
        for fut in concurrent.futures.as_completed(futs):
            agent, status, detail = fut.result()
            results[agent] = (status, detail)
            print(f"review.round{args.round}.{agent}={status} {detail}")

    # 归档产物。
    arc_dir = project / ".ohmyagents" / "reviews" / f"round{args.round}"
    arc_dir.mkdir(parents=True, exist_ok=True)
    for agent, (_, detail) in results.items():
        m = re.search(r"→ (.+)$", detail)
        if m:
            src = Path(m.group(1))
            if src.exists():
                shutil.copyfile(src, arc_dir / f"{agent}.md")

    statuses = {s for s, _ in results.values()}
    if statuses <= {"ok"}:
        print(f"review.round{args.round}.verdict=all-clear")
        return 0
    if "error" in statuses or "timeout" in statuses:
        print(f"review.round{args.round}.verdict=has-errors")
        return 2
    print(f"review.round{args.round}.verdict=has-findings")
    return 1


if __name__ == "__main__":
    sys.exit(main())
