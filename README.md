# EasyEnglish

> 跨平台英→中即时翻译器，Rust + cargo workspace 实现。

EasyEnglish 是一个 ambient 工作流工具：后台常驻 + 全局快捷键唤起的悬浮翻译框。
输入英文单词，立刻得到中文释义（来自打包的离线词典），并可以为每个词附加任意的
个人 *Note*（英文 → 任意内容）。

> 本仓库是 Rust 重写版本。早期 Qt → ImGui 的 C++ 实现在 git 历史的 `v0.3.0` tag，
> 可用 `git checkout v0.3.0` 拿到。重写动机详见 `docs/adr/0004-rewrite-in-rust-m2a-style.md`。

## 状态

**Phase 1**（当前）：完成 `Dict` 与 `Core` 两个核心模块。
**Phase 2**（后续）：填充 `Win` / `Mac` / `Linux` 平台层（tray + 全局快捷键 + 悬浮窗 + 安装包）。

## 仓库结构

仓库布局学自 [m2a](https://github.com/DexterDreeeam/M2A)：每个模块都有 `.design.md`（架构 + 时序图）
和 `.interface.md`（Public/Private API 接口），文档之间用 ⬆️/⬇️ 互链。

```
EasyEnglish/
├── Cargo.toml           # cargo workspace
├── product.json         # 运行时配置（被 Core 模块读取）
├── .design.md           # 项目根 design（链向各模块）
├── .interface.md        # 项目根 interface（链向各模块）
├── AGENTS.md            # AI 协作宪法
├── Dict/                # 离线词典数据 + SQLite 访问层
│   ├── src/  data/  tests/
│   ├── .design.md  .interface.md
├── Core/                # 配置 / 历史 / Note / Lookup / AppState
│   ├── src/  tests/
│   ├── .design.md  .interface.md
├── Win/  Mac/  Linux/   # Phase 2 占位，每个都有 .design.md / .interface.md
└── docs/adr/            # 架构决策记录
```

## 本地开发

### 一次性安装

需要 Rust stable（≥ 1.83）：
```powershell
# https://rustup.rs/
rustup default stable
cargo install cargo-nextest --locked
```

### 日常命令

```powershell
cargo build --workspace
cargo nextest run --workspace --no-tests=pass
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

> 仓库根的 `.cargo/config.toml` 已经默认开启 `rust-lld` 链接器 + `split-debuginfo`，
> 全量编译预计 ~90 s，增量改动 < 3 s。

### 没有 CI

按设计，本仓库**不使用 GitHub Actions**——所有质量门禁靠开发者本地跑上述命令。
未来若决定加 CI，写 30 行 `.github/workflows/ci.yml` 即可。

## 贡献

任何修改前**先**读：
1. 根目录 `.design.md` 与 `.interface.md`
2. 你要改的模块的 `<Module>/.design.md` 与 `<Module>/.interface.md`
3. `AGENTS.md`（修改约束）

测试约定见 `<Module>/tests/.test.md`。

## 许可证

MIT — 见 `LICENSE`。
