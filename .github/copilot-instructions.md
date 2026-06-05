# GitHub Copilot — Repository Instructions

This is the **EasyEnglish** repository: a modular Rust rewrite of an English → Chinese desktop translator.

## Repository Shape
- `Dict/` (`ee-dict`): Offline SQLite-backed dictionary.
- `Core/` (`ee-core`): Configuration, Notes, History, and AppState.
- `Utils/` (`ee-utils`): Thread-safe shared state primitives.
- `App/` (`ee-win` / `ee-mac` / `ee-linux`): OS tray, hotkeys, and overlay UI placeholders.

## Quality Gate (Run locally before committing)
```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
cargo build --workspace
cargo nextest run --workspace --no-tests=pass
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

## Critical Rules
1. **Prioritize `.interface.md`**: Do not write code in any module without reading its interface contract first.
2. **Layering Discipline**: No UI, OS, or Network dependencies in `Dict/` or `Core/`. Dependencies flow: Platforms → Core → Dict.
3. **Tests are Contract**: Any change to `src/` must be paired with matching updates in `tests/` and `tests/.test.md`.
