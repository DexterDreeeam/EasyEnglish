# `favorites` Contract

**Source path**: `src/core/favorites/`
**Owner test path**: `tests/unit/favorites/`
**Status**: frozen (since iter-005)

## 1. Public API (FROZEN — change requires ADR)

```cpp
namespace easyenglish::core::favorites {

enum class FavoritesError { InvalidInput, StorageError };

struct FavoriteEntry {
    std::string headword;
    std::int64_t added_at_unix{0};
};

class FavoritesStore {
public:
    static auto open(storage::Database db) -> std::expected<FavoritesStore, FavoritesError>;

    auto add(std::string_view word)                          -> std::expected<void, FavoritesError>;
    auto remove(std::string_view word)                       -> std::expected<void, FavoritesError>;
    auto contains(std::string_view word) const               -> std::expected<bool, FavoritesError>;
    auto list(std::size_t max = 200) const                   -> std::expected<std::vector<FavoriteEntry>, FavoritesError>;
};

}
```

## 2. Invariants

- Schema auto-created on `open()` (parallel to history).
- `add()` is **idempotent** — adding an already-favorited word succeeds with
  no row count change.
- `remove()` is also idempotent — removing a non-favorited word succeeds.
- Case-insensitive (`COLLATE NOCASE` on `headword`).
- `list()` is ordered by `added_at DESC`.

## 3. Error codes

| Code | Meaning |
|---|---|
| `InvalidInput`  | empty or > 128-char word |
| `StorageError`  | wrap of `storage::StorageError` |

## 4. Dependencies

- Allowed: `src/core/storage`, Qt6::Core types only.
- Forbidden: Qt UI, network I/O.

## 5. Test fixtures

- Tests use an in-memory `Database`; schema is auto-created.

## 6. Change log

- 2026-06-04 — iter-005: initial implementation + frozen. UPSERT (DO NOTHING)
  for add(). UI plumbed: star toolbutton + favorites tab in side panel.
