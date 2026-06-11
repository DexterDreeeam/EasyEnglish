# iter-014 Retrospective

- **What shipped**: All five `ee-core` submodules in place (`config` / `notes` / `history` / `lookup` / `state`). 35 Core integration tests + 1 unit test all pass; the workspace totals 54/54 tests pass in ~0.55 s. Core/.interface.md frozen.

- **Key design trade-offs**
  1. **NoteStore is runtime-only** (per the user's request). In `Note { word, content }`, `content` is an arbitrary string — it can be a translation, mnemonic, URL, or sentence pattern; semantically it is a superset of v0.3.0's "favorites". If Phase 2 needs persistence, just add `persist_to(path)` without breaking the interface.
  2. **Lookup order is controlled by Config**: `prefer_notes_over_dict` defaults to true, so an overriding Note takes priority over the dictionary; after a user writes `add_note("apple", "my own translation")` it takes effect immediately. The dict-first path is also implemented and tested, making it easy to switch later.
  3. **HistoryStore tests use a fake_clock**. `with_clock(max, fn)` is a doc-hidden, test-only entry point that avoids wall-clock flakiness; the production path still goes through `with_capacity` → `default_clock()`.
  4. **Config deserialization uses a nested RawConfig + Option**. All JSON fields are `Option<T>`; when absent, `into_config()` injects defaults, and `partial_override_keeps_other_defaults` verifies the whole merge logic by setting just one field.

- **Where the AI went wrong / lessons**
  1. `AppState::recent()` initially wanted to return `&[HistoryEntry]`, but a `VecDeque` cannot give a contiguous slice; changed to returning a `Vec<HistoryEntry>` clone — a single copy of ≤ 50 entries, negligible cost.
  2. `LookupHit` as an enum fits the "Note OR Dict" either/or semantics better than a struct + Source field.
  3. Did not repeat iter-013's toolchain / Debug pitfalls — the repository instruction hint took effect directly.
  4. **Minor process mishap**: `New-Item` and the `create` tool were run in parallel in the same turn, and create finished before mkdir, so retro.md was not written the first time; written and amend-pushed in the next turn. From now on, any `create` that requires a directory to already exist is split into two turns.

- **Numbers**
  - File count: 5 src + 5 tests + 1 .test.md + 1 retro
  - LOC: ~700 (including tests)
  - Incremental build: < 3 s
