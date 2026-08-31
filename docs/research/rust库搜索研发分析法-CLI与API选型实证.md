# rust库搜索研发分析法-CLI与API选型实证

> 2026-08-31。编码经验研究（`ponytail懒人阶梯与oma编码经验.md`）的第一个配套落地方法：选 Rust 库时怎么搜索、怎么排序、怎么判断稳。素材来自用户提供的可脚本化方法综述（内置 CLI、第三方 CLI、官方 HTTP API、Rust 客户端），本机逐条实证后标六态。网页浏览法（docs.rs / lib.rs / blessed.rs 人工看）不在本文重复。

## 背景

AGENTS 写 Rust 规则要求「先查 crates.io / docs.rs / GitHub 是否已有最流行、最稳定的库」，但没给操作方法。本文是该规则的搜索与分析手册：脚本化拿到候选、按维护度排序、验证稳定性证据。六态齐后可浓缩进规则或 guide 细则。

## 关键结论

1. **三级通道按成本递增**：内置 CLI（零安装）到第三方 CLI（TUI/JSON 增强）到 HTTP API（脚本与 CI）到 Rust 客户端（自研工具）。日常选型内置 CLI 加 API 即覆盖。[经验: 用户综述；内置 CLI 与 API 本机实证可用]
2. **本机镜像环境的关键订正**：cargo 源被 rsproxy 替换后，`cargo search` 与 `cargo info` 直接报错，必须加 `--registry crates-io`。[实证: 2026-08-31 本机，两命令均复现]
3. **判断「稳」的四个信号**：`max_stable_version` 非空（进过 1.x 更稳）、`recent_downloads` 高（还在用，优于总下载量）、`updated_at` 近 6 到 12 个月（还在维护）、`reverse_dependencies` 总数不低（有人在依赖它）。[经验: 用户综述的启发式阈值，非官方标准；四个字段本机可取实证]
4. **排序口径**：搜索 API 的 `sort=recent-downloads` / `recent-updates` 比 `relevance` 与总 `downloads` 更接近「还在用、还在维护」。crates.io 无质量分排序（lib.rs 才有），要「最稳」就用 API 自合成规则。[经验: 用户综述；sort 参数本机生效]
5. **反向依赖的返回结构**：`GET /api/v1/crates/<name>/reverse_dependencies` 的 `.dependencies[].crate_id` 是被查 crate 自己，依赖方名字在 `.versions[].crate`，总数在 `.meta.total`——直接取 crate_id 会得到一列重复的名字。[实证: 2026-08-31 本机 jq 解包 rmux-sdk，meta.total=6]
6. **限流与礼节**：API 限 1 req/s，必须带可识别 User-Agent（建议含联系方式）；大批量走 sparse index（`index.crates.io`）或每日 dump（`static.crates.io/db-dump.tar.gz`），不要扫搜索 API。[经验: 用户综述；本机实测每次请求约 1.2s 且连续请求未被拒]
7. **第三方 CLI（cargo-seek / crates-info / cargo-crates / get-blessed）本机均未装**：日常够用内置 CLI 加 curl 加 jq，出现真实高频需求（交互筛选、按维护者浏览、fzf 预览流）再装，符合阶梯第五档。[实证: 2026-08-31 查 PATH 四者皆无]
8. **Rust 客户端 crates_io_api 未本机编译**：oma 当前无程序化查 crates.io 的需求（check 走 pin 哈希不搜索）；需要时选 crates_io_api（自带 UA 与限流间隔）而非 Cargo 内部的 crates-io 库。[经验: 用户综述；需求未出现故不验证]

## 现状或实测

### 本机实证命令与结果（2026-08-31，Windows + rsproxy 镜像）

| 步骤 | 命令 | 结果 |
| --- | --- | --- |
| 搜索 | `cargo search http client --limit 5 --registry crates-io` | 通；提示还有 12365 个匹配；不带旗标直接报错 |
| 详情 | `cargo info serde --registry crates-io` | 通；输出版本 1.0.229、license、rust-version、features、repository、docs 链接 |
| API 搜索 | `curl -A "<UA>" "…/api/v1/crates?q=rmux&per_page=3&sort=recent-downloads"` | 200；recent_downloads、default_version、num_versions 字段齐 |
| API 详情 | `GET /api/v1/crates/rmux-sdk` | 200；`max_stable_version=0.10.0`、`updated_at`、`repository` 可取 |
| 反向依赖 | `GET /api/v1/crates/rmux-sdk/reverse_dependencies` | 200；`meta.total=6`；依赖方在 `.versions[].crate` |
| jq 稳度筛选 | `select(.max_stable_version != null) \| select((.recent_downloads // 0) > 1000)` 管道 | 通；terminal multiplexer 关键词筛出 clearscreen、alacritty_terminal 等 |
| 第三方 CLI | `command -v cargo-seek cratesinfo cargo-crates get-blessed` | 四者皆未装 |

### 推荐工作流（本机订正版）

```bash
# 1 拿候选（本机必须 --registry crates-io）
cargo search <关键词> --limit 20 --registry crates-io

# 2 按维护度重排 + 稳度筛选（1 req/s，带 UA）
curl -sS -A "<工具名> (联系方式)" \
  "https://crates.io/api/v1/crates?q=<关键词>&per_page=30&sort=recent-downloads" \
| jq -r '.crates[]
    | select(.max_stable_version != null)
    | select((.recent_downloads // 0) > 1000)
    | "\(.recent_downloads)\t\(.max_stable_version)\t\(.name)\t\(.updated_at[0:10])"'

# 3 定点核证（license、repository、features）
cargo info <name> --registry crates-io

# 4 谁在用（依赖方在 versions[].crate，总数在 meta.total）
curl -sS -A "<UA>" "https://crates.io/api/v1/crates/<name>/reverse_dependencies" \
| jq -r '.meta.total, (.versions[].crate)'

# 5 安全过一遍，再装
cargo audit            # 若已装 cargo-audit
cargo add <name> --features <...>
```

阈值是启发式不是门禁：新库（recent_downloads 低但 updated_at 新）与领域窄库（reverse_dependencies 少）会被误杀，人工复核 `cargo info` 的 repository 与文档再定。[经验: 用户综述加本机解读]

### 与本仓规则的关系

- 本文补全 AGENTS 写 Rust 规则「先查最流行最稳」的**操作面**：怎么查（三级通道）、怎么排（recent-downloads 加 stable 加 updated 加 reverse）、怎么定（cargo info 核证后 add）。
- 与 ponytail 阶梯第五档衔接：搜索是「确认没有现成依赖」的证据步骤；搜到了就不写，搜不到再上移档位。
- oma 自身场景例证：`rmux-sdk` 的 `max_stable_version=0.10.0`（未到 1.x）但 `updated_at` 新、仓库活跃，配合 pin `=0.10.0` 使用——窄领域新库的复核路径。[实证: 2026-08-31 API 详情]

## 踩坑沉淀

| 坑 | 现象 | 正确处理 |
| --- | --- | --- |
| 镜像源劫持 cargo search | rsproxy 替换后两命令直接报错 | 一律加 `--registry crates-io` |
| 反向依赖取错字段 | `.dependencies[].crate_id` 全是被查 crate 名 | 依赖方取 `.versions[].crate`，总数取 `.meta.total` |
| 拿总下载量当稳度 | 高总量可能是弃维护的历史积累 | 用 recent_downloads 加 updated_at 合成判断 |
| category: 语法进 q 参数 | 前端搜索框语法与 API q 解析不一致 | 脚本用独立 query 参数（`&category=`、`&keyword=`） |
| 大批量扫搜索 API | 限流 1 req/s 会拖死且不礼貌 | 少量元数据走 index.crates.io；全库分析走每日 dump |

## 待办

1. 六态已齐（CLI 与 API 路径本机实证，第三方与 Rust 客户端按需求标注），可浓缩为 guide 细则或并入写 Rust 规则：「查怎么查」三行口径（`--registry crates-io`、稳度四信号、cargo add 收尾）。
2. 第二个落地方法（用户预告）到达后同法研究，两法齐再统一升规则。
3. cargo-seek 等第三方工具：出现高频交互筛选需求再装并补实证。
