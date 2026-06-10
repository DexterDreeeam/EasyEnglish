# iter-012 Retrospective

- **What shipped**: The repo was completely cleared of the C++ working tree (git history keeps v0.3.0) and rebuilt as an m2a-style cargo workspace. 5 top-level crates (Dict / Core / Win / Mac / Linux) + root-level `.design.md` / `.interface.md` / `AGENTS.md` / `README.md` / `product.json` / `.cargo/config.toml` / `rust-toolchain.toml` + ADR-0004. `cargo build --workspace` passes in ~8.5 s; `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` are clean.

- **Where the AI went wrong / lessons**
  1. `cargo nextest` exits 1 with "no tests to run" by default in the Phase 1 empty-lib state. Already changed the command in AGENTS.md / README to `--no-tests=pass`; once every crate has real tests from iter-013 on, this flag is a no-op.
  2. I declared sqlite `opt-level=2` in `[profile.dev.package.rusqlite]` ahead of time, but Phase 1 has no rusqlite dependency yet, so cargo prints two warning lines on every run. Decided to keep it — it documents the intent, and the warning disappears automatically once iter-013 introduces rusqlite.
  3. Capitalized top-level directories `Dict/Core/Win/Mac/Linux` are uncommon in the Rust ecosystem, but the user explicitly required them. The crate names inside use the Rust-idiomatic `ee-dict` etc., written in AGENTS.md §6 to stop the AI from "correcting" the directory names in a PR.

- **Keep next time**: m2a's style of writing `.design.md` with mermaid sequence diagrams + `.interface.md` split into Public / Private. Much clearer than a plain-text contract — any AI handed `Dict/.interface.md` can start writing the `lookup()` implementation without reading the code.
