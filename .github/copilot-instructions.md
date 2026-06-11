# GitHub Copilot — Repository Instructions

This is the **EasyEnglish** repository: a modular Rust rewrite of an English → Chinese desktop translator.

## Language Policy
- **Conversation with the user is in Chinese** — all chat replies, questions, and explanations to the user must be written in Chinese.
- **Repository file content is in English** — source code, comments, identifiers, documentation (`.design.md` / `.interface.md` / `Tests/UnitTest` specs / `Tests/UITest` specs / ADRs / READMEs), and commit messages must be written in English.
- Exception: user-facing content that is inherently bilingual — dictionary data, localized UI strings, and the Chinese portions of web pages — is expected to contain Chinese and is exempt from the English-only rule.

## Repository Shape
- `Dict/` (`ee-dict`): Offline SQLite-backed dictionary.
- `Core/` (`ee-core`): Configuration, Notes, History, and AppState.
- `Utils/` (`ee-utils`): Thread-safe shared state primitives.
- `App/` (`ee-win` / `ee-mac` / `ee-linux`): OS tray, hotkeys, and overlay UI placeholders.
- `Tests/UnitTest/` (`ee-unit-tests`): Centralized Rust unit and integration tests.
- `Tests/UITest/`: Markdown UI automation specifications.

## Quality Gate (Run locally before committing)
```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
cargo build --workspace
cargo nextest run --workspace --no-tests=pass
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

## Automated UI Testing
- Run automated UI tests on Windows after code changes that affect the Windows UI, overlay behavior, hotkeys, focus, keyboard input, IME handling, or end-to-end app behavior.
- Also run automated UI tests whenever the user explicitly asks to test, run, or verify the app.
- Keep UI automation scenarios as markdown files under `Tests/UITest/`; do not mix UI test specifications into Rust unit-test files.
- Use the dedicated Hyper-V VM named `vm-ee-test` for these tests. Do not use the host desktop as the default UI test target.
- If Hyper-V is unavailable or disabled, ask the user to confirm before enabling it because enabling Hyper-V can require a reboot and can affect other virtualization software.
- If `vm-ee-test` does not exist, create it before running UI tests.
- Before downloading a Windows ISO, check the user's Downloads directory and reuse a suitable existing ISO when possible.
- If no suitable ISO exists, proactively download an official Windows ISO into the Downloads directory and use it for the VM setup.
- Do not reboot the host, enable Hyper-V, create or modify VMs, or download large ISO files silently. Make the action visible to the user first.
- Do not bypass Windows licensing or activation requirements.
- Report the UI test result, the VM used, and any setup gap or blocker in the final response.

## Critical Rules
1. **Prioritize `.interface.md`**: Do not write code in any module without reading its interface contract first.
2. **Layering Discipline**: No UI, OS, or Network dependencies in `Dict/` or `Core/`. Dependencies flow: Platforms → Core → Dict.
3. **Tests are Contract**: Any change to `src/` must be paired with matching Rust tests under `Tests/UnitTest/`. UI behavior changes must also update the markdown UI specifications under `Tests/UITest/`.
4. **Centralized Tests**: Do not add new Rust tests under module-local `tests/` directories or inline `#[cfg(test)] mod tests`; keep automated test code in `Tests/UnitTest/`.
