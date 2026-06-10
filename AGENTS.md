# AGENTS.md — EasyEnglish Rust Rewrite Constitution

> Any AI coding assistant (Copilot CLI / Cursor / Claude Code / others) **must** read this file in full before starting work.
> This file is deliberately kept short (≤ 150 lines). Verbose specs will be politely ignored.

## 1. The project in one sentence

A cross-platform (Win/Mac/Linux) English → Chinese instant translator, built with Rust + a cargo workspace.
Code organization and documentation conventions are **learned from `C:\r\m2a`**: every module has a `.design.md` (design) + `.interface.md` (interface).
The development flow is "AI drafts + local automated tests + human review". **There is no CI** — quality gates are run locally by the developer.

## 2. Read these first

- The repo-root `.design.md` (system overview + top-level module table) + `.interface.md` (interface index)
- The `<Module>/.design.md` and `<Module>/.interface.md` of the module you are changing
- Any referenced ADR: `docs/adr/NNNN-*.md`

Starting to change code without reading the three kinds of files above = violating this constitution.

## 3. Build / test / static checks (**must pass** before committing)

```powershell
cargo build --workspace
cargo nextest run --workspace --no-tests=pass
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

Install the one-time tool before the first run: `cargo install cargo-nextest --locked`

> `--no-tests=pass` lets Phase 1 (where each crate is an empty lib before Dict/Core are implemented) also pass the gate.
> From iter-013 on, every crate has tests, so this flag no longer actually triggers.

> **Toolchain choice**: the repo does **not** pin the channel via `rust-toolchain.toml`. Reason: on a dev machine that has both
> `stable-x86_64-pc-windows-gnu` and `stable-x86_64-pc-windows-msvc` installed,
> `channel = "stable"` is resolved by rustup to gnu, which makes crates that need a C compiler (such as rusqlite)
> fail because they cannot find `gcc.exe`. The MSRV stays in `Cargo.toml [workspace.package].rust-version`
> (1.83); a developer simply sets the local default with `rustup default stable-x86_64-pc-windows-msvc`.

## 4. Directory discipline

Top-level directories are the top-level cargo crates:

| Directory | May depend on | Must not depend on |
|---|---|---|
| `Dict/`   | pure libs such as `rusqlite`, `serde`, `serde_json`, `thiserror` | any UI / OS / network crate; `ee-core` (avoid a dependency cycle) |
| `Core/`   | `ee-dict`, `ee-utils`, `serde`, `serde_json`, `thiserror`, `chrono`, `directories` | UI / OS / network crates; `ee-win/mac/linux` |
| `Utils/`  | `std` only (no third-party deps)                                | UI / OS / network crates; other crates in this workspace |
| `App/Win/`    | `ee-core`, `ee-dict`, `ee-utils`, `windows`, UI/packaging crates | `ee-mac`, `ee-linux` |
| `App/Mac/`    | `ee-core`, `ee-dict`, `ee-utils`, mac crates such as `objc2-*`          | `ee-win`, `ee-linux` |
| `App/Linux/`  | `ee-core`, `ee-dict`, `ee-utils`, linux-only crates                | `ee-win`, `ee-mac` |

Dependencies flow strictly downward: **Platforms → Core → Dict**. Any reverse dependency must have an ADR written first.

## 5. Modification constraints

1. **Focus one module per change**. Cross-module changes must have an ADR written first.
2. **No drive-by refactoring** of code outside the current task's scope. Even if it looks ugly.
3. **Public API** (declared in the Public section of `<Module>/.interface.md`) changes must:
   - update the corresponding `.interface.md`;
   - list the changed surface in the commit message;
   - write an ADR for larger changes.
4. **New dependencies** may only be introduced centrally through `Cargo.toml`'s `[workspace.dependencies]`;
   each crate reuses the version via `dep = { workspace = true }`.
5. **Error handling**: library crates define module-specific errors with `thiserror::Error`;
   the bin layer (the future App/Win/Mac/Linux entry points) uses `anyhow::Result`.
6. **Do not invent APIs**. When unsure whether a given rusqlite/serde function exists, first say "not sure", then look it up or ask.

## 6. Code style

- Enforced by `cargo fmt` (rustfmt default config + edition = 2021); no need to discuss formatting in a PR.
- Modules / functions / variables follow Rust conventions: `snake_case`; types / traits: `UpperCamelCase`.
- Top-level directory names keep their leading capital per the user's request (`Dict`/`Core`/`Win`/`Mac`/`Linux`),
  but the crate names are `ee-dict` / `ee-core` / `ee-win` / `ee-mac` / `ee-linux`.
- Public APIs must have doc comments (`///`), otherwise `cargo doc --no-deps` emits a warning.

## 7. Testing requirements

- Every module's `tests/` directory must have a `.test.md` listing the purpose of **each test** (m2a convention).
- Any change to `<Module>/src/**` **must** update `<Module>/tests/` in sync.
- Runtime-only data such as Note / History: tests cover the default state + boundaries (empty / cap / casing).
- Fuzzy-matching output uses **golden files** (`<Module>/tests/fixtures/*.golden.json`);
  golden updates must be highlighted in the commit message, not changed silently.
- Integration tests (`tests/test_*.rs`) run against an in-memory DB or temp files; tests **must not** touch the network.

## 8. Pre-commit self-check (answer all before replying "done" to the user)

1. Are all the files I changed within the module scope declared by the task?
2. Did I add/change `tests/` and `tests/.test.md` in sync? Do the tests pass locally with `cargo nextest run --workspace`?
3. Did I touch any Public-section API in a `<Module>/.interface.md`? If so, is the corresponding doc updated?
4. Did I introduce a new dependency? If so, was it added only in root `Cargo.toml [workspace.dependencies]`?
5. Is the module dependency direction still Platforms → Core → Dict?
6. Do `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass?
7. Do the key pub fns have `///` doc comments?

Answers must be written in the commit message or PR description as "yes / no + evidence (command output or diff reference)".

## 9. Questions and uncertainty

- When unsure, say "I don't know" plainly; do not make things up.
- If the task definition is ambiguous or conflicts with `.interface.md`, **stop first** and write one clarifying question; do not continue on imagination.
- If you find a bug in existing code unrelated to the current task — **record** it in the commit message, but **do not fix** it. Open a new task.

## 10. Out of scope

- Do not write new docs (except `.design.md` / `.interface.md` / ADR / retro).
- Do not modify `AGENTS.md` itself unless the user explicitly requests it.
- Do not introduce GitHub Actions / automated releases (the user explicitly declined).
- Do not delete any existing historical record under `docs/adr/`; change a deprecated ADR's Status to superseded.
