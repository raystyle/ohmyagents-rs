# Changelog

本文件只记录**大版本里程碑**：定位变更、发布、阶段完成、核心能力整体落地。细碎条目由 `docs\diary\YYYY-MM-DD-*.md` 与 git 历史承载。

## [Unreleased]

### 里程碑

> 文档地基已完成。当前目标是各功能部件 POC（方案 0005）。代码尚未发布。

- **项目定位**：Oh My Agents，通用智能体多路复用任务编排器。方案 `docs\history\0004-项目重新定位-通用智能体多路复用任务编排器.md`。上一版措辞见 0002；首期切面见 0001。
- **文档骨架**：对齐 `D:\ohmypwsh` 的四段 AGENTS、三原语、history/research/references。
- **CLI 名**：二进制 `oma`；项目名仍是 ohmyagents；运行时目录 `.ohmyagents`。
- **rmux 安装**：`oma check` 按 `catalog/rmux.toml` 检测版本与 SHA256，缺则全平台安装完整包。
