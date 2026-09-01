# /// script
# requires-python = ">=3.10"
# dependencies = ["websockets>=12"]
# ///
"""连本地 rmux web-share 网关，抓 spectator 视角的 session view 数据。

用法：uv run --script .tools/share-view-probe.py <token> [ws-url]
默认 ws://127.0.0.1:9777/share。打印 ready 与首个 session 视图消息里
每个 pane 的坐标尺寸，用于排查前端布局数据源。
"""
import asyncio
import json
import sys


async def main() -> None:
    if len(sys.argv) < 2:
        print("usage: share-view-probe.py <token> [ws-url]")
        return
    token = sys.argv[1]
    uri = sys.argv[2] if len(sys.argv) > 2 else "ws://127.0.0.1:9777/share"
    import websockets

    async with websockets.connect(uri) as ws:
        await ws.send(json.dumps({"type": "connect", "token": token}))
        seen_view = False
        while not seen_view:
            raw = await asyncio.wait_for(ws.recv(), timeout=6)
            msg = json.loads(raw)
            t = msg.get("type")
            print(f"MSG type={t}")
            if t == "ready":
                keep = {k: msg.get(k) for k in ("role", "session", "spectator_access")}
                print("  ready:", json.dumps(keep))
            panes = msg.get("panes") or (msg.get("view") or {}).get("panes")
            if panes:
                for p in panes:
                    keys = ("id", "x", "y", "w", "h", "width", "height", "cols", "rows", "left", "top")
                    print("  pane:", json.dumps({k: p.get(k) for k in keys if k in p}))
                seen_view = True


if __name__ == "__main__":
    asyncio.run(main())
