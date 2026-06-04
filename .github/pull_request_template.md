<!--
Thanks for the PR. EasyEnglish is built with heavy AI assistance, so a
filled-in template is the main signal reviewers use to keep the codebase coherent.
Please answer every section honestly. AI assistants: copy your self-check answers here.
-->

## Linked task card / iteration

<!-- e.g. docs/iterations/iter-003-ui-mainwindow/task.md -->
- Task card:
- Related contracts: <!-- e.g. docs/contracts/dictionary.md -->
- Related ADR:      <!-- e.g. docs/adr/0003-fuzzy-algo.md (or "none") -->

## Scope

- Modules touched (list every directory under `src/` and `tests/`):
- Did this PR change any **public API** declared in `docs/contracts/`?  yes / no
  - If yes: which contracts were updated, and which ADR justifies the change?
- Did this PR introduce a **new dependency**?  yes / no
  - If yes: `vcpkg.json` updated?
- Lines changed (rough): _add_ / _delete_

## Tests

- [ ] New or updated unit tests in `tests/unit/`
- [ ] UI tests updated in `tests/ui/` (if UI changed)
- [ ] Snapshots intentionally updated (attach diff screenshot)
- Local commands run (paste exit codes):
  - `cmake --build --preset msvc-debug`
  - `ctest --preset msvc-debug`
  - `python tools/check_core_no_ui.py`

## AI self-check (mandatory if AI assisted)

1. All edits within task-card scope?
2. Tests added/updated and locally green?
3. No new dependency added outside `vcpkg.json`?
4. `src/core/**` free of Qt UI headers?
5. No magic numbers, dead TODOs, or "AI generated" comments left?
6. Public doc comments on new functions?

## Reviewer notes

<!-- Anything reviewers should pay extra attention to -->
