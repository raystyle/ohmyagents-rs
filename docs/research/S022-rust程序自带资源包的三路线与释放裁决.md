# S022：rust 程序自带资源包的三路线与释放裁决

- 日期：2026-09-01
- 关联：`P0023`（kanban 前端资源包化）；用户定调「web\share-src 应该是在源码里，安装释放到用户应用数据目录，研究 rust 编译的程序如何自带资源包解压安装」；前置 `P0022`（现状是 serve 读仓库 `docs\web\kanban`——克隆仓才可用，装出去的产品没有这个目录）

## 一、为什么要研究

oma 作为可安装产品（exe + 自管数据根 `~/.ohmyagents`），kanban 前端资产目前躺在**仓库**里被 serve 运行时读盘——只在本仓开发态可用。产品化要求：单 exe 自带资源，首次运行释放到用户应用数据目录，与 rmux 安装（`%LOCALAPPDATA%\ohmyagents\rmux\<ver>`）、agent 安装（`~/.ohmyagents/agents/<name>/<ver>`）同一哲学。

## 二、三路线对比

| 路线 | 机制 | 优点 | 缺点 |
| --- | --- | --- | --- |
| A. `include_bytes!` 直出 | 每文件嵌常量，handler 内存返回 | 零依赖、零落盘 | 文件多时手写路由爆炸（我们正是这么起步的）；无「释放到数据目录」形态 |
| B. `rust-embed` crate | derive 宏嵌整目录，运行时迭代取文件 | API 优雅、主流（下载量大） | 引新依赖；仍是内存直出，同样不落盘 |
| C. **嵌入归档 + 首启释放** | build.rs 把资产目录打成 `tar.gz` 进 `OUT_DIR`，`include_bytes!` 嵌入；运行时按指纹解压到数据目录 | 单 exe 自带资源、落盘形态可检视可替换、`tar`+`flate2` **已是依赖**（install.rs 同款解压代码可复用）、指纹驱动增量更新 | build.rs 里写打包逻辑（walk + tar 写入，约 40 行） |

## 三、裁决

**路线 C**。[推断: 依需求推导——用户点名「释放到用户应用数据目录」，排除内存直出系（A/B）；零新依赖与 install.rs 解压复用加分]

细节口径：

- build.rs：`docs/web/kanban` → `OUT_DIR/kanban-web.tar.gz`（内容寻址：对 tar.gz 字节算 sha256 取前 8 位做指纹目录名）。
- 运行时：`ensure_web_assets()` 检 `~/.ohmyagents/web/<sha8>/`——存在即跳过；缺失或指纹不符则清旧目录解压新包（同 install.rs 的 zip 解压姿态：建目录、逐条目写、错误带路径）。
- serve 的 `KANBAN_DIR` 从仓库路径改指 `~/.ohmyagents/web/<sha8>/`（`oma_home()` 同源，`OMA_HOME` 可覆盖）。
- 仓库仍保留 `docs/web/kanban`（构建输入，保证无 node 环境也能 `cargo build` 出带资源的产品）；`share-src` 是它的源（npm build 产出），两层都进仓。

## 四、关键结论

1. 「安装释放」语义的正确单位是**内容指纹目录**（sha 前 8 位），不是版本号——前端 rebuild 频率与 oma 版本解耦，指纹对了才免重释放。[推断]
2. build.rs 产物经 `OUT_DIR` 传递：`println!("cargo:rerun-if-changed=docs/web/kanban")` 声明依赖，资产变了自动重打包重编。[经验: cargo 标准机制]
3. 解压复用 install.rs 的 tar_gz 路径（`extract_tar_gz` 已 pub）——资源包与安装包同一套解压纪律（错误带路径、清残留）。[实证: P0012 已验]
