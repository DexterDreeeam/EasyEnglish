# iter-013 Retrospective

- **What shipped**: Full implementation of the `ee-dict` crate. `Entry` / `DictError` / `DictStore` (`open` / `create_or_seed` / `lookup` / `suggest` / `len`). The seed data `Dict/data/seed_en_cn.json` contains 245 common English words, each with an IPA phonetic + multi-sense Chinese definitions. 19 tests in total (3 unit + 14 integration + 2 seed-file health checks) all pass in 0.236 s.

- **Where the AI went wrong / lessons**
  1. **`rust-toolchain.toml` channel = "stable" picked the wrong toolchain on this machine**. The dev machine had both `stable-x86_64-pc-windows-gnu` and `stable-x86_64-pc-windows-msvc` installed; rustup picked gnu, and rusqlite failed to compile because it could not find `gcc.exe`. Decided to remove `rust-toolchain.toml` entirely and let cargo use the rustup default (msvc); the minimum version is backstopped by `Cargo.toml [workspace.package].rust-version = "1.83"`; AGENTS.md §3 keeps a detailed note.
  2. `DictStore` lacking `Debug` made `Result::expect_err` fail to compile. `rusqlite::Connection` itself is not `Debug`, so a manual `impl Debug` was written that only exposes the entry count, without leaking implementation details (connection / cached statements).
  3. `cargo fmt` wanted to wrap several long lines across multiple lines; my hand-written style in the first commit differed slightly from rustfmt's defaults. Accepted fmt's reflow; future code goes through `cargo fmt` before commit.

- **Data decisions**
  - The seed word list of 245 words covers the most common English verbs / nouns / prepositions; every word's Chinese was checked to have no empty arrays.
  - The `definitions` field is stored as a JSON array in a sqlite TEXT column (no sub-table). At 245 words × ~1.5 senses on average the parse cost is negligible, and the schema-complexity win is bigger.
  - `headword` uses `COLLATE NOCASE` as the PRIMARY KEY, natively supporting case-insensitive lookups; no lowercased side column needed.

- **Keep / change next time**
  - Keep: listing entries in tests/.test.md before writing the tests — makes test coverage visible at a glance, so a relay AI knows which cases are already pinned.
  - Change: from iter-014 on, run `cargo fmt --all` on all new code before showing it to the AI tool, to avoid review noise.
