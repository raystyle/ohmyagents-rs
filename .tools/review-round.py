# /// script
# requires-python = ">=3.10"
# ///
"""agent 轮换接力 review 工作流（用户定调 2026-09-01：**不并行**，每次一
家、轮换接力——每家 review 的是上一家修复后的最新状态；某家 FINDINGS=0
即「找不出问题」，工作流终止）。

判定契约：产物 output.md 第一行 `FINDINGS=N`；KNOWN-WONTFIX.md 内事项
不计入、不复报。产物归档 `<project>/.ohmyagents/reviews/relay/<轮>-<agent>.md`。

用法：
  uv run --script .tools/review-round.py relay <轮> <agent> [--project P] [--oma PATH] [--timeout 1800]

退出码：0 = 本家全绿（工作流终止）；1 = 有发现（修复后接力下一家）；
2 = 超时/失败。
"""
import argparse
import re
import shutil
import subprocess
import sys
from pathlib import Path

AGENTS = ["claude", "codex", "grok", "kimi"]

REVIEW_PROMPT = (
    "review 当前仓库 src/ 与最近提交：找正确性、并发、边界、契约问题。"
    "范围：src/orch.rs、src/api.rs、src/server.rs、src/task.rs、src/main.rs、src/mcp.rs。"
    "收敛规则：.ohmyagents/reviews/KNOWN-WONTFIX.md 内是已拍板不修的取舍，"
    "不计入 FINDINGS、不要复报；对该决策有新的实质证据才可重新提出。"
    "此前各轮已修项（git log 可见）不复报。"
    "产物契约：output.md 第一行必须是 FINDINGS=N（N=发现的问题条数，没发现问题写 0），"
    "之后按严重度列结论；确认没问题的方面也要列出。写完 output.md 最后创建空文件 DONE。"
)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("mode", choices=["relay", "round"])
    ap.add_argument("n", type=int, help="接力轮次（归档目录名用）")
    ap.add_argument("agent", help="本轮接力的 agent 名")
    ap.add_argument("--project", default=".")
    ap.add_argument("--oma", default="oma", help="oma 可执行路径（不在 PATH 时传）")
    ap.add_argument("--timeout", type=int, default=1800)
    args = ap.parse_args()
    project = Path(args.project).resolve()
    agent = args.agent
    if agent not in AGENTS:
        print(f"relay.agent={agent} invalid; pick from {AGENTS}")
        return 2

    cmd = [
        args.oma, "task", agent, REVIEW_PROMPT,
        "--timeout", str(args.timeout), "--project", str(project),
    ]
    print(f"relay.{args.n}={agent} timeout={args.timeout}s")
    try:
        r = subprocess.run(
            cmd, capture_output=True, text=True, timeout=args.timeout + 120,
            encoding="utf-8", errors="replace",
        )
    except subprocess.TimeoutExpired:
        print(f"relay.{args.n}.{agent}=timeout")
        return 2
    except OSError as e:
        print(f"relay.{args.n}.{agent}=error 无法启动 oma（{e}；用 --oma 传路径）")
        return 2
    if r.returncode != 0:
        print(f"relay.{args.n}.{agent}=error {(r.stderr or r.stdout or '').strip()[:300]}")
        return 2
    m = re.search(r"^task\.dir=(.+)$", r.stdout, re.M)
    if not m:
        print(f"relay.{args.n}.{agent}=error stdout 无 task.dir 行")
        return 2
    out_md = Path(m.group(1).strip()) / "output.md"
    text = out_md.read_text(encoding="utf-8", errors="replace") if out_md.exists() else ""
    fm = re.search(r"^FINDINGS=(\d+)\s*$", text, re.M)
    if not fm:
        print(f"relay.{args.n}.{agent}=error 产物缺 FINDINGS=N 契约行 → {out_md}")
        return 2
    n = int(fm.group(1))

    arc_dir = project / ".ohmyagents" / "reviews" / args.mode / f"{args.n}-{agent}"
    arc_dir.mkdir(parents=True, exist_ok=True)
    if out_md.exists():
        shutil.copyfile(out_md, arc_dir / "output.md")

    if n == 0:
        print(f"relay.{args.n}.{agent}=all-clear → {arc_dir}")
        return 0
    print(f"relay.{args.n}.{agent}=findings FINDINGS={n} → {arc_dir}")
    return 1


if __name__ == "__main__":
    sys.exit(main())
