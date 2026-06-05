---
name: m2a-module-scaffold
description: Scaffold a new EasyEnglish module directory in m2a style. Creates the standard layout (.design.md, .interface.md, tests/.test.md, optionally Cargo.toml + src/lib.rs) with placeholder content the user can fill in. Use when starting a brand-new top-level module in this repo.
---

# m2a Module Scaffold

Lay out a new top-level module under the EasyEnglish workspace following the
m2a documentation convention.

## When to use

User says any of:
- "scaffold a new module called X"
- "add module X with the standard layout"
- "create X module skeleton"
- (any phrasing that implies creating a fresh `<Module>/` directory)

## What the user must specify

1. **Module name** — TitleCase, top-level (e.g. `Win`, `Cloud`, `Plugin`).
2. **One-line description** — for the auto-filled module header.
3. **Mode** — one of:
   - `docs-only`: produce only `.design.md` + `.interface.md` + `tests/.test.md`. **No** Cargo.toml, **no** src/, **no** workspace member registration. This is the safe default and matches the "design before implementation" workflow.
   - `with-crate`: ALSO produce `Cargo.toml`, `src/lib.rs` stub, and a single empty `tests/test_smoke.rs`. ALSO append the module to `[workspace] members` in the root `Cargo.toml`.

Default if user doesn't say: **`docs-only`**.

## Files produced (docs-only)

```
<Module>/
├── .design.md
├── .interface.md
└── tests/
    └── .test.md
```

Use the templates from `../../agents/m2a-module-designer.agent.md` —
specifically the three template blocks under "Template: `.design.md`",
"Template: `.interface.md`", and "Template: `tests/.test.md`".

Substitute:
- `<Module>` → user-supplied name
- `<module>` → lowercase form
- `<iter-NNN>` → next free iter number (query SQL `todos` table for highest existing `iter-NNN-*` id)

## Files produced (with-crate, additional)

```
<Module>/
├── Cargo.toml
├── src/
│   └── lib.rs
└── tests/
    └── test_smoke.rs
```

### `<Module>/Cargo.toml` template

```toml
[package]
name        = "ee-<module>"
version     = { workspace = true }
edition     = { workspace = true }
rust-version = { workspace = true }
description = "<one-line description>"

[dependencies]
# Add via workspace deps when needed:
# thiserror = { workspace = true }

[dev-dependencies]
# tempfile = { workspace = true }
```

### `<Module>/src/lib.rs` template

```rust
//! ee-<module> — <one-line description>
//!
//! See `.design.md` and `.interface.md` in this crate's directory.

// Public surface comes in iter-NNN. Until then, this crate is intentionally empty.
```

### `<Module>/tests/test_smoke.rs` template

```rust
//! Smoke test ensures the crate compiles and links. Replaced by real
//! integration tests in iter-NNN.

#[test]
fn crate_compiles() {
    // Intentionally empty — successful compilation IS the assertion.
}
```

### Root `Cargo.toml` patch (with-crate only)

Append to the `members` list, preserving alphabetical order:

```toml
[workspace]
members = [
    "Core",
    "Dict",
    "Linux",
    "Mac",
    "<Module>",   # <-- inserted in alphabetical position
    "Win",
]
```

## After scaffolding, ALWAYS

1. Run `cargo build --workspace` (with `$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"` on Windows). Report success or failure.
2. Print a 5-line summary:
   - module name
   - files created (full relative paths)
   - mode used (docs-only / with-crate)
   - build status
   - "next: invoke `m2a-module-designer` agent to fill the design"

## Hard rules

- **Never** mix `docs-only` and `with-crate` in one call. Pick one.
- **Never** create `target/` — Cargo does this when needed.
- **Never** add new workspace dependencies in this step. The user adds them
  when actually implementing.
- **Never** modify other modules' files.
- If the `<Module>/` directory already exists, **stop and ask** before
  overwriting anything.
