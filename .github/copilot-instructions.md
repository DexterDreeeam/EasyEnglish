# GitHub Copilot — Repository Instructions

This is the **EasyEnglish** repository: a modular Rust rewrite of an English →
Chinese desktop translator. Workspace = 5 crates (`Dict`/`Core`/`Win`/`Mac`/
`Linux`), m2a-style documentation discipline.

> Authoritative project constitution lives in [`AGENTS.md`](../AGENTS.md). This
> file is the *Copilot-specific* layer on top of it. When the two conflict,
> `AGENTS.md` wins.

## What to read before changing anything

1. The module's contract: `<Module>/.interface.md`
2. The module's design: `<Module>/.design.md`
3. The module's test spec: `<Module>/tests/.test.md`
4. Any referenced ADR in `docs/adr/`

Not reading these before editing = constitutional violation (see `AGENTS.md` §2).

## Repository shape at a glance

| Dir | Crate | Status |
|---|---|---|
| `Dict/` | `ee-dict`  | ✅ frozen (iter-013) — SQLite-backed EN→CN dictionary |
| `Core/` | `ee-core`  | ✅ frozen (iter-014) — Config, Notes, History, Lookup, AppState |
| `Win/`  | `ee-win`   | 🔨 placeholder — Windows tray + hotkey + overlay + MSI |
| `Mac/`  | `ee-mac`   | 🔨 placeholder — macOS tray + dmg |
| `Linux/` | `ee-linux` | 🔨 placeholder — Linux tray + AppImage |

## Quality gate (run all four locally before claiming done)

```powershell
cargo build --workspace
cargo nextest run --workspace --no-tests=pass
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

If `cargo` is not on PATH:
```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
```

## Hard rules for Copilot

1. **Do not write code in a module without reading its `.interface.md` first.**
2. **Do not add a workspace member** without an ADR in `docs/adr/`.
3. **Do not depend on UI / OS / network from `Dict/` or `Core/`.** Use
   `tools/check_core_no_ui.py` (planned) to verify.
4. **Do not pin the Rust toolchain** via `rust-toolchain.toml`. On dual-host
   machines (msvc + gnu installed), `channel = "stable"` may resolve to gnu
   and break rusqlite. Trust the user's `rustup default`. (See `AGENTS.md` §3.)
5. **Use `m2a-module-scaffold` skill** to lay out a new module — never
   hand-roll `.design.md` / `.interface.md` / `tests/.test.md` from scratch.
6. **Use `rust-quality-gate` skill** to run the four-step gate; do not invent
   shorter variants.
7. **Never create `.md` files for planning / notes / scratch** in the repo
   except as part of an `iter-NNN-*/retro.md` or under `docs/adr/`. Session
   scratch lives in `~/.copilot/session-state/`.
8. **Title-case directory names** (`Dict/`, `Core/`, `Win/`, `Mac/`, `Linux/`)
   are intentional product-level boundaries. Cargo crate names underneath are
   conventional snake-with-dash (`ee-dict`, `ee-core`, ...).
9. **Tests are part of the contract.** Every change to `src/` must update the
   matching `tests/.test.md` and `tests/test_*.rs`.

## Commit conventions

- Commit message format: `<type>(<scope>): <subject>` (conventional commits).
- Include the trailer:
  `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>`
- Reference the iter ID in the subject when applicable (`feat(core): iter-014 ...`).

## Custom agents and skills

This repo ships specialised Copilot agents and skills in this directory:

| Type | Path | Purpose |
|---|---|---|
| agent | [`agents/m2a-module-designer.agent.md`](agents/m2a-module-designer.agent.md) | Design a new module in m2a style: `.design.md` + `.interface.md` + `tests/.test.md`, **no implementation** |
| agent | [`agents/rust-test-guardian.agent.md`](agents/rust-test-guardian.agent.md) | Audit a Rust change: does every src delta have matching test updates and a `tests/.test.md` entry? |
| skill | [`skills/m2a-module-scaffold/SKILL.md`](skills/m2a-module-scaffold/SKILL.md) | Scaffold a new `<Module>/` directory with the standard m2a layout |
| skill | [`skills/rust-quality-gate/SKILL.md`](skills/rust-quality-gate/SKILL.md) | Run the canonical 4-step quality gate |

Use them; do not re-derive their behavior in chat.
