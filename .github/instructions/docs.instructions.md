---
applyTo: "**/*.md"
---

# Markdown / m2a docs style — EasyEnglish

These rules apply to every Markdown file in the workspace. They reinforce the
m2a convention this project copied from `C:\r\m2a`.

## Navigation links

- Every `.design.md` and `.interface.md` **must** start with parent / child
  navigation:
  ```
  ⬆️ [parent name](relative/path/.design.md)
  ⬇️ [child](path/.design.md) · [child2](other/path/.design.md)
  ```
  - Root `.design.md` has only `⬇️` (no parent).
  - Leaf `tests/.test.md` has only `⬆️` (no children).

## File layout per module

Every `<Module>/` directory **must** contain:

```
<Module>/
├── .design.md          # architecture, sequences, decisions, open questions
├── .interface.md       # Public + Private API contract, frozen after first iter
├── tests/
│   └── .test.md        # one entry per test, mirrors tests/test_*.rs
└── ... (Cargo.toml, src/, etc.)
```

## `.interface.md` structure

```markdown
⬆️ [parent](../.interface.md)

# <Module> Module — Interface

> **Status:** **frozen** (since iter-NNN). Any breaking change requires an ADR.

## Public

### `TypeOrFn`

```rust
/// signature here
```

## Private

- internal notes (not re-exported)
```

## `tests/.test.md` structure

- Table: `| File | Covers |` listing every `tests/test_*.rs`.
- One section per test file with bullet-pointed test names in `name — what it does` form.
- Final "Quality gate" section with the 4-command block.

## Mermaid diagrams

Allowed (and encouraged) in `.design.md`:

```mermaid
flowchart LR
    A --> B
```

Avoid in `.interface.md` (interface should be code signatures + prose).

## Forbidden

- New top-level Markdown files in the repo root (other than `README.md`,
  `AGENTS.md`, `LICENSE.md`).
- "TODO" / "FIXME" comments in *.md without an iter card or ADR to fix it.
- HTML inside Markdown (mermaid is fine; raw `<div>` `<table>` is not).
- Emoji in module names (`Dict`, `Core`, ...) — but `✅` / `🔨` / `📐`
  status badges in tables are fine.
