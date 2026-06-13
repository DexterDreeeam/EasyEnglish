---
applyTo: "**/*.md"
---

# Documentation Style Rules — EasyEnglish

1. **m2a Layout**: Every module directory under `ee/` must contain `.design.md` (architecture) and `.interface.md` (API contracts). Centralized Rust test code lives under `ee/Tests/UnitTest/`; markdown UI test specifications live under `ee/Tests/UITest/`.
2. **Navigation**: Markdown files must start with parent/child navigation links (e.g., ⬆️ [parent] / ⬇️ [child]).
3. **No TODOs**: Do not leave dangling `TODO` or `FIXME` comments in Markdown files.
