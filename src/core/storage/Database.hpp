#pragma once

#include <expected>
#include <filesystem>
#include <string_view>

#include "core/storage/Statement.hpp"
#include "core/storage/errors.hpp"

struct sqlite3;

namespace easyenglish::core::storage {

/// RAII handle for an open SQLite connection.
///
/// Construction is via the static factory `open()`. The instance is movable but
/// not copyable; the destructor closes the connection. All operations report
/// failure via `std::expected<…, StorageError>` and never throw on SQLite errors.
class Database {
public:
    /// Special sentinel: pass to `open()` to create an in-memory database.
    static constexpr std::string_view kInMemory = ":memory:";

    Database() = delete;
    Database(const Database&) = delete;
    Database& operator=(const Database&) = delete;
    Database(Database&& other) noexcept;
    Database& operator=(Database&& other) noexcept;
    ~Database();

    /// Open an existing database file (or the in-memory sentinel).
    /// Returns `StorageError::IoError` when the file does not exist or cannot be opened.
    static auto open(const std::filesystem::path& file) -> std::expected<Database, StorageError>;

    /// Execute one or more SQL statements that return no rows.
    auto execute(std::string_view sql) -> std::expected<void, StorageError>;

    /// Prepare a parameterized statement for repeated execution.
    auto prepare(std::string_view sql) -> std::expected<Statement, StorageError>;

    /// Raw connection accessor for advanced callers (e.g. PRAGMA or backup APIs).
    /// Kept package-private by convention — UI/app layers MUST NOT use it.
    [[nodiscard]] sqlite3* handle() const noexcept { return db_; }

private:
    explicit Database(sqlite3* db) noexcept;

    sqlite3* db_{nullptr};
};

}  // namespace easyenglish::core::storage
