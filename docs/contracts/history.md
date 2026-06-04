# `history` Contract

**Source path**: `src/core/history/`
**Owner test path**: `tests/unit/history/`
**Status**: frozen (since iter-004)

## 1. Public API (FROZEN — change requires ADR)

```cpp
namespace easyenglish::core::history {

enum class HistoryError {
    InvalidInput,
    StorageError,
};

struct HistoryEntry {
    std::string headword;
    std::int64_t last_at_unix{0};
    std::int64_t count{0};
};

class HistoryStore {
public:
    static constexpr std::size_t kDefaultRecent = 20;

    static auto open(storage::Database db) -> std::expected<HistoryStore, HistoryError>;

    auto record(std::string_view word)                          -> std::expected<void, HistoryError>;
    auto recent(std::size_t max = kDefaultRecent) const         -> std::expected<std::vector<HistoryEntry>, HistoryError>;
    auto clear()                                                -> std::expected<void, HistoryError>;
};

}  // namespace
```

## 2. Invariants

- `open()` is responsible for **schema creation** (`CREATE TABLE IF NOT EXISTS history(...)`).
  The store can therefore be plugged into any `Database`, including a brand-new
  one opened with `Database::createOrOpen`.
- `record()` is **upsert**: re-recording an existing headword increments
  its `count` and updates `last_at` to "now".
- Case-insensitive: "Apple" / "APPLE" / "apple" collapse to one row
  (table uses `COLLATE NOCASE` on `headword`).
- `recent()` is ordered by `last_at DESC`. Caller-supplied `max=0` returns
  an empty vector (no error).
- `record()` with an empty or > 128-char input returns `HistoryError::InvalidInput`.

## 3. Error codes

| Code | Meaning | Caller should… |
|---|---|---|
| `InvalidInput`  | Empty or too-long word | not retry; fix the input |
| `StorageError`  | Wrap of `storage::StorageError` | surface non-fatally; do not block UI |

## 4. Dependencies

- Allowed: `src/core/storage`, standard library, `Qt6::Core` (string/container types only)
- Forbidden: Qt UI, network I/O

## 5. Test fixtures

- Tests use an in-memory `Database` and verify behavior end-to-end;
  no on-disk fixture is required because schema is auto-created.

## 6. Performance budget

- `record()` p99 < 1ms on `:memory:` DB.
- `recent(20)` p99 < 1ms.

## 7. Change log

- 2026-06-04 — iter-004: initial implementation + frozen. UPSERT via SQLite
  `ON CONFLICT(headword) DO UPDATE`. UI wires into MainWindow's right-side
  QListWidget (see `docs/contracts/ui-mainwindow.md`).
