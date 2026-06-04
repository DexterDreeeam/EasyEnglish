# `storage` Contract

**Source path**: `src/core/storage/`
**Owner test path**: `tests/unit/storage/`
**Status**: frozen  (since iter-001)

## 1. Public API (FROZEN — change requires ADR)

```cpp
namespace easyenglish::core::storage {

enum class StorageError {
    NotFound,
    InvalidQuery,
    ConstraintViolation,
    IoError,
    Busy,
};

class Database {
public:
    // RAII open. Pass ":memory:" for unit tests when no fixture is needed.
    static auto open(const std::filesystem::path& file)
        -> std::expected<Database, StorageError>;

    Database(Database&&) noexcept;
    Database& operator=(Database&&) noexcept;
    ~Database();

    // Run a single statement returning no rows.
    auto execute(std::string_view sql) -> std::expected<void, StorageError>;

    // Prepare a parameterized statement for repeated execution.
    auto prepare(std::string_view sql)
        -> std::expected<class Statement, StorageError>;
};

class Statement {
    // bind() / step() / column() — exact signatures TBD by iter-001 author.
};

}  // namespace easyenglish::core::storage
```

## 2. Invariants

- `Database` is movable but non-copyable.
- `Database::execute` and `Statement::step` must NOT throw — all failure
  modes flow through `std::expected<…, StorageError>`.
- Concurrent **read-only** access from multiple threads is safe; concurrent
  writes are NOT safe and must be serialized by the caller.
- A `Database` opened on a missing file returns `StorageError::IoError`
  (the storage layer does not create new files implicitly; migrations do).

## 3. Error codes

| Code | Meaning | Caller should… |
|---|---|---|
| `NotFound`             | Statement returned zero rows where one was expected | treat as empty result |
| `InvalidQuery`         | SQL preparation failed                              | bug → fail the test |
| `ConstraintViolation`  | UNIQUE / NOT NULL / CHECK failed                    | surface to user |
| `IoError`              | File not found, permission denied, disk full        | surface to user |
| `Busy`                 | SQLite returned SQLITE_BUSY                         | retry with backoff |

## 4. Dependencies

- Allowed: standard library, `sqlite3` (vcpkg), `fmt`
- Forbidden: any `Qt6::*` *except* `Qt6::Core` types like `QString` if needed
  at the boundary; **no** `QtSql` (we want zero Qt at the persistence layer).

## 5. Test fixtures

- `tests/fixtures/mini_dict.sqlite` — 200-entry seed DB produced by
  `tools/seed_db.py`. Committed to git; regenerate via `python tools/seed_db.py`.

## 6. Performance budget

- `prepare()` overhead < 100us per statement on warm cache.
- Connecting to `mini_dict.sqlite` < 5ms.

## 7. Change log

- 2026-06-04 — iter-001: initial implementation + frozen. Database/Statement
  with `std::expected` error returns, ASan-clean move semantics, fixture
  `tests/fixtures/mini_dict.sqlite` (54 entries) committed.
