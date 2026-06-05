# iter-012 Retrospective

- **What shipped**: 仓库从 C++ 工作树彻底清空（git 历史保留 v0.3.0），重建为 m2a 风格的 cargo workspace。5 个顶层 crate（Dict / Core / Win / Mac / Linux）+ 根级 `.design.md` / `.interface.md` / `AGENTS.md` / `README.md` / `product.json` / `.cargo/config.toml` / `rust-toolchain.toml` + ADR-0004。`cargo build --workspace` 在 ~8.5 s 通过；`cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` 干净。

- **AI 走偏 / 教训**
  1. `cargo nextest` 在 Phase 1 空 lib 状态下默认会 exit 1 with "no tests to run"。已在 AGENTS.md / README 把命令改成 `--no-tests=pass`；iter-013 起每个 crate 都有真实测试后这个 flag 是 no-op。
  2. 我在 `[profile.dev.package.rusqlite]` 提前声明了 sqlite opt-level=2，结果 Phase 1 还没 rusqlite 依赖，cargo 每次跑都报两行 warning。决定保留——文档化了 intent，且 iter-013 引入 rusqlite 后 warning 自动消失。
  3. 顶层目录大写 `Dict/Core/Win/Mac/Linux` 在 Rust 生态里少见，但用户明确要求。crate 内部 name 用 Rust 惯例的 `ee-dict` 等，写在 AGENTS.md §6 防止 AI 在 PR 里"纠正"目录名。

- **下次保留**：m2a 的 `.design.md` 用 mermaid 时序图 + `.interface.md` 用 Public / Private 切分的写法。比纯文字契约清楚很多——任何 AI 拿到 `Dict/.interface.md` 就能开始写 `lookup()` 实现，不需要看代码。
