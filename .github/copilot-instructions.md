# GitHub Copilot — Repository Instructions

This is the **EasyEnglish** repository: a modular Rust rewrite of an English → Chinese desktop translator.

## Language Policy
- **Conversation with the user is in Chinese** — all chat replies, questions, and explanations to the user must be written in Chinese.
- **Repository file content is in English** — source code, comments, identifiers, documentation (`ee/.design.md` / `ee/.interface.md` / `ee/Tests/UnitTest` specs / `ee/Tests/UITest` specs / ADRs / READMEs), and commit messages must be written in English.
- Exception: user-facing content that is inherently bilingual — dictionary data, localized UI strings, and the Chinese portions of web pages — is expected to contain Chinese and is exempt from the English-only rule.
- **Simplified Chinese and Traditional Chinese are separate supported languages.** Do not generate Traditional Chinese dictionary data by converting Simplified Chinese. Use source data that is explicitly Traditional Chinese, Hong Kong Chinese, Cantonese, or otherwise independently sourced for the Traditional Chinese package.

## Repository Shape
- `ee/`: The Rust workspace and all project implementation files.
- `ee/Dict/` (`ee-dict`): Offline SQLite-backed dictionary.
- `ee/Core/` (`ee-core`): Configuration, Notes, History, and AppState.
- `ee/Utils/` (`ee-utils`): Thread-safe shared state primitives.
- `ee/App/` (`ee-win` / `ee-mac` / `ee-linux`): OS tray, hotkeys, and overlay UI placeholders.
- `ee/Tests/UnitTest/` (`ee-unit-tests`): Centralized Rust unit and integration tests.
- `ee/Tests/UITest/`: Markdown UI automation specifications.

## Quality Gate (Run locally only when explicitly requested)

Use these commands only when the user explicitly asks to test, verify, or run
quality gates.

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
cd ee
cargo build --workspace
cargo nextest run --workspace --no-tests=pass
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

## Local Install / Run Requests
- When the user asks to build and run, run, launch locally, compile and run, or uses equivalent wording such as `编译运行`, `运行`, `本地启动`, `启动`, follow `.github/skills/local-install/SKILL.md`.
- Do not satisfy those requests by launching a debug binary directly unless the user explicitly asks for a debug run.
- The default local workflow is: build the current OS release package into `ee/Release/` and silently install it.
- Launch or UI verification runs on the local machine only when the user explicitly asks to run, launch, test, or verify the app.

## Versioning and Packaging
- The project version file is `ee/version` (no extension) and contains the packaged app version string in the stable three-number format `EasyEnglish-<major>.<minor>.<patch>`, for example `EasyEnglish-1.0.0`.
- Version checks compare only the `version` file content from the installed package and GitHub raw. Do not include or compare package language suffixes in version checks.
- Windows installer filenames add a language-pair suffix after the version. For the Chinese-English build, use `EasyEnglish-<version>-CN.exe`, where `CN` means Chinese-English bidirectional dictionary.

## Automated UI Testing
- Do not automatically run tests after code changes unless the user explicitly asks to test, verify, run quality gates, or run the app.
- When the user explicitly asks to test, run tests on the local machine.
- Run automated UI tests on Windows only when the user explicitly asks to test, run, or verify Windows UI behavior.
- Keep UI automation scenarios as markdown files under `ee/Tests/UITest/`; do not mix UI test specifications into Rust unit-test files.
- UI tests that interact with the desktop run on the local Windows desktop.
- Report the local test result and any setup gap or blocker when tests are requested.

## Critical Rules
1. **Prioritize `.interface.md`**: Do not write code in any module without reading its interface contract first.
2. **Layering Discipline**: No UI, OS, or Network dependencies in `ee/Dict/` or `ee/Core/`. Dependencies flow: Platforms → Core → Dict.
3. **Tests are Contract**: Any change to `ee/**/src/` must be paired with matching Rust tests under `ee/Tests/UnitTest/`. UI behavior changes must also update the markdown UI specifications under `ee/Tests/UITest/`.
4. **Centralized Tests**: Do not add new Rust tests under module-local `tests/` directories or inline `#[cfg(test)] mod tests`; keep automated test code in `ee/Tests/UnitTest/`.
