# AGENTS.md — EasyEnglish Rust 重写仓库宪法

> 任何 AI 编程助手（Copilot CLI / Cursor / Claude Code / 其他）开始工作前 **必须** 读完本文件。
> 本文件刻意保持简短（≤ 150 行）。冗长规范会被礼貌地忽略。

## 1. 项目一句话

跨平台（Win/Mac/Linux）的英→中即时翻译器，Rust + cargo workspace 实现。
代码组织和文档约定**学自 `C:\r\m2a`**：每个模块都有 `.design.md`（设计） + `.interface.md`（接口）。
开发流程是"AI 起草 + 本地自动化测试 + 人工 review"。**没有 CI**——质量门禁靠开发者本地跑。

## 2. 一定要先读

- 仓库根 `.design.md`（系统总览 + 顶层模块表） + `.interface.md`（接口索引）
- 你要改动的模块的 `<Module>/.design.md` 和 `<Module>/.interface.md`
- 任何被引用的 ADR：`docs/adr/NNNN-*.md`

不读上面三类文件就开始改代码 = 违反本宪法。

## 3. 构建 / 测试 / 静态检查（**必须能跑通**才能提交）

```powershell
cargo build --workspace
cargo nextest run --workspace --no-tests=pass
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

第一次跑前装一次性工具：`cargo install cargo-nextest --locked`

> `--no-tests=pass` 让 Phase 1（Dict/Core 实现之前各 crate 是空 lib）也能通过 gate。
> iter-013 起每个 crate 都有测试后这个 flag 实际不会触发。

> **Toolchain 选择**：仓库**不**通过 `rust-toolchain.toml` 钉 channel。原因：在同时
> 装有 `stable-x86_64-pc-windows-gnu` 与 `stable-x86_64-pc-windows-msvc` 的开发机上，
> `channel = "stable"` 会被 rustup 解析到 gnu，导致 rusqlite 等需要 C 编译器的 crate
> 找不到 `gcc.exe` 而 fail。MSRV 保留在 `Cargo.toml [workspace.package].rust-version`
> 里（1.83）；开发者用 `rustup default stable-x86_64-pc-windows-msvc` 设本机默认即可。

## 4. 目录纪律

顶层目录就是顶层 cargo crate：

| 目录 | 可以依赖 | 禁止依赖 |
|---|---|---|
| `Dict/`   | `rusqlite`, `serde`, `serde_json`, `thiserror` 等纯库 | 任何 UI / OS / 网络 crate；`ee-core`（避免环依） |
| `Core/`   | `ee-dict`, `serde`, `serde_json`, `thiserror`, `chrono`, `directories` | UI / OS / 网络 crate；`ee-win/mac/linux` |
| `Win/`    | `ee-core`, `ee-dict`, `windows`, UI/打包相关 crate | `ee-mac`, `ee-linux` |
| `Mac/`    | `ee-core`, `ee-dict`, `objc2-*` 等 mac crate          | `ee-win`, `ee-linux` |
| `Linux/`  | `ee-core`, `ee-dict`, linux-only crate                 | `ee-win`, `ee-mac` |

依赖方向严格向下：**Platforms → Core → Dict**。任何反向依赖必须先写 ADR。

## 5. 修改约束

1. **一次修改专注一个模块**。跨模块变更必须先写 ADR。
2. **禁止顺手重构** 不在当前任务范围内的代码。哪怕看着不顺眼。
3. **公开 API**（在 `<Module>/.interface.md` 里 Public 段声明的）若需变更，必须：
   - 更新对应 `.interface.md`；
   - 在 commit message 里把变更面列出来；
   - 较大变更写 ADR。
4. **新依赖** 只允许通过 `Cargo.toml` 的 `[workspace.dependencies]` 集中引入；
   各 crate 用 `dep = { workspace = true }` 复用版本。
5. **错误处理**：库 crate 用 `thiserror::Error` 定义模块专属错误；
   bin 层（未来的 Win/Mac/Linux 入口）用 `anyhow::Result`。
6. **不要编造 API**。不确定 rusqlite/serde 某个函数是否存在时，先回答"不确定"，再去查或问。

## 6. 代码风格

- 由 `cargo fmt` 强制（rustfmt 默认配置 + edition = 2021）；不需要在 PR 里讨论格式。
- 模块 / 函数 / 变量遵 Rust 惯例：`snake_case`；类型 / trait：`UpperCamelCase`。
- 顶层目录名按用户要求保留首字母大写（`Dict`/`Core`/`Win`/`Mac`/`Linux`），
  但 crate name 是 `ee-dict` / `ee-core` / `ee-win` / `ee-mac` / `ee-linux`。
- 公共 API 必须有文档注释（`///`），否则 `cargo doc --no-deps` 会出 warning。

## 7. 测试要求

- 每个模块 `tests/` 目录下必须有 `.test.md` 列出**每个测试**的目的（m2a 约定）。
- 任何 `<Module>/src/**` 改动 **必须** 同步更新 `<Module>/tests/`。
- Note / History 等 runtime-only 数据：测试覆盖默认状态 + 边界（empty / cap / 大小写）。
- 模糊匹配类输出用 **golden 文件**（`<Module>/tests/fixtures/*.golden.json`），
  golden 更新必须在 commit message 里高亮，不能默默改。
- 集成测试（`tests/test_*.rs`）跑 in-memory DB 或临时文件；**禁止**测试触网。

## 8. 提交前 self-check（每次回复用户"完成"前都要答完）

1. 我改动的文件是否都在任务声明的模块范围内？
2. 我是否同步加 / 改了 `tests/` 与 `tests/.test.md`？测试在本地 `cargo nextest run --workspace` 通过吗？
3. 我是否动了任何 `<Module>/.interface.md` 中 Public 段的 API？若动了，对应文档是否更新？
4. 我引入了新依赖吗？若有，是否只在 root `Cargo.toml [workspace.dependencies]` 增加？
5. 模块的依赖方向是否仍然 Platforms → Core → Dict？
6. `cargo fmt --all --check` 与 `cargo clippy --workspace --all-targets -- -D warnings` 通过吗？
7. 关键 pub fn 是否有 `///` 文档注释？

回答必须以"是 / 否 + 证据（命令输出或 diff 引用）"的形式写在 commit message 或 PR 描述里。

## 9. 提问与不确定性

- 拿不准就明说"I don't know"，不要瞎编。
- 如果任务定义模糊或与 `.interface.md` 冲突，**先停下**写一条澄清问题，不要靠想象继续。
- 如果发现现存代码有 bug 但与本任务无关 —— 在 commit message 里**记录**它，但**不要修**。开一张新 task。

## 10. 范围之外

- 不写新文档（除了 `.design.md` / `.interface.md` / ADR / retro）。
- 不修改 `AGENTS.md` 本身，除非用户明确要求。
- 不引入 GitHub Actions / 自动 release（用户明确拒绝）。
- 不删除 `docs/adr/` 下任何已存在的历史记录；废弃的 ADR 改 Status 为 superseded。
