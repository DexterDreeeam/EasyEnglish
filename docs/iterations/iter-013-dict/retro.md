# iter-013 Retrospective

- **What shipped**: `ee-dict` crate 完整实现。`Entry` / `DictError` / `DictStore`（`open` / `create_or_seed` / `lookup` / `suggest` / `len`）。种子数据 `Dict/data/seed_en_cn.json` 含 245 个常用英文词，每个带 IPA 音标 + 多义中文释义。整体 19 个测试（3 个 unit + 14 个集成 + 2 个种子文件健康检查）全过，0.236 s。

- **AI 走偏 / 教训**
  1. **`rust-toolchain.toml` channel = "stable" 在本机选错了 toolchain**。开发机同时装了 `stable-x86_64-pc-windows-gnu` 与 `stable-x86_64-pc-windows-msvc`，rustup 选了 gnu，rusqlite 编译时找不到 `gcc.exe` 直接挂。决定彻底去掉 `rust-toolchain.toml`，让 cargo 用 rustup 默认（msvc）；最低版本由 `Cargo.toml [workspace.package].rust-version = "1.83"` 兜底；AGENTS.md §3 留了详细说明。
  2. `DictStore` 缺 `Debug` 导致 `Result::expect_err` 编译失败。`rusqlite::Connection` 自身不是 `Debug`，所以写了手动 `impl Debug` 只暴露 entry count，对应的实现细节（连接 / 缓存语句）不外泄。
  3. `cargo fmt` 想把若干长行折成多行；首次提交我手写的代码风格略与 rustfmt 默认不一致。已经接受 fmt 重排，未来代码直接 `cargo fmt` 后再 commit。

- **数据决策**
  - 种子词表 245 词，覆盖最常用的英文动词 / 名词 / 介词；每词中文都校验过没空数组。
  - `definitions` 字段以 JSON 数组存进 sqlite TEXT 列（不开子表）。245 词 × 平均 1.5 个义项规模下解析成本可忽略，schema 复杂度 win 更多。
  - `headword` 用 `COLLATE NOCASE` 作 PRIMARY KEY，原生支持大小写不敏感查询；无需 lowercased 副列。

- **下次保留 / 改变**
  - 保留：tests/.test.md 在写测试前先列条目 — 让测试覆盖率一眼可见，AI 接力时知道哪些 case 已 pin。
  - 改变：iter-014 起所有新代码先跑 `cargo fmt --all`，再让 AI tool 看，避免出 review noise。
