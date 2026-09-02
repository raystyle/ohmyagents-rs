#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""mdcharlint.py：Markdown 技术文档禁用字符检查（G005）。

用法：uv run --script .tools/mdcharlint.py 文件1.md [文件2.md ...]
规则：先掩掉豁免区（围栏代码块、行内代码、链接目标、裸 URL），再逐字符
扫四类硬禁令（破折号、箭头、emoji、非法全角）；中文正文标点白名单放行。
源自《中英文 Markdown 技术文档写作规范》v1.0（2026-09-02 转档，G005）。
"""
import re
import sys
from pathlib import Path

RULES = [
    ("DASH", re.compile("[–—―−ー]")),
    ("ARROW", re.compile("[←-⇿➔➜➡⬅-⬇]")),
    ("EMOJI", re.compile(
        "[\U0001F000-\U0001FAFF☀-➿⬀-⬏"
        "‼⁉ℹ︀-️‍]")),
    ("FULLWIDTH", re.compile("[！-｠　‘-”]")),
]
CJK_OK = set("，。：；？！、（）《》「」『』·")
INLINE_CODE = re.compile(r"`[^`]*`")
LINK_TARGET = re.compile(r"\]\([^)]*\)")
BARE_URL = re.compile(r"https?://\S+")


def mask(line):
    line = INLINE_CODE.sub(lambda m: "`" + " " * (len(m.group()) - 2) + "`", line)
    line = LINK_TARGET.sub("]( )", line)
    return BARE_URL.sub(" ", line)


def scan(text):
    in_fence = False
    for no, raw in enumerate(text.splitlines(), 1):
        if raw.lstrip().startswith(("```", "$$")):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        for col, ch in enumerate(mask(raw), 1):
            for name, pattern in RULES:
                if pattern.match(ch) and not (name == "FULLWIDTH" and ch in CJK_OK):
                    yield no, col, name, ch
                    break


def main():
    hits = 0
    for path in sys.argv[1:]:
        for no, col, name, ch in scan(Path(path).read_text(encoding="utf-8")):
            print(f"{path}:{no}:{col}: {name} U+{ord(ch):04X} {ch!r}")
            hits += 1
    print(f"违规 {hits} 处" if hits else "通过：未发现违规字符")
    return 1 if hits else 0


if __name__ == "__main__":
    sys.exit(main())
