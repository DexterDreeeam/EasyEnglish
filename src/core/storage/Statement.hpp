#pragma once

#include <cstdint>
#include <expected>
#include <string>
#include <string_view>

#include "core/storage/errors.hpp"

struct sqlite3_stmt;

namespace easyenglish::core::storage {

class Database;

/// Thin RAII wrapper around `sqlite3_stmt*`.
///
/// All bind/step/reset calls report failure via `std::expected<…, StorageError>`;
/// no SQLite error ever escapes as an exception. Movable, non-copyable.
class Statement {
public:
    Statement() = delete;
    Statement(const Statement&) = delete;
    Statement& operator=(const Statement&) = delete;
    Statement(Statement&& other) noexcept;
    Statement& operator=(Statement&& other) noexcept;
    ~Statement();

    // Parameter binding. Indices are 1-based, matching SQLite's native convention.
    auto bind(int index, std::int64_t value) -> std::expected<void, StorageError>;
    auto bind(int index, double value) -> std::expected<void, StorageError>;
    auto bind(int index, std::string_view value) -> std::expected<void, StorageError>;
    auto bindNull(int index) -> std::expected<void, StorageError>;

    /// Advance one step. Returns true if a row is available, false if execution
    /// completed (DONE). StorageError on failure.
    auto step() -> std::expected<bool, StorageError>;

    /// Column accessors are 0-based. Calling these without a successful `step()`
    /// that returned true is undefined behavior (sqlite3 itself returns NULLs).
    [[nodiscard]] auto columnInt64(int index) const -> std::int64_t;
    [[nodiscard]] auto columnDouble(int index) const -> double;
    [[nodiscard]] auto columnText(int index) const -> std::string;
    [[nodiscard]] auto isColumnNull(int index) const -> bool;

    /// Reset for re-execution; bindings are preserved unless `clearBindings()` is called.
    auto reset() -> std::expected<void, StorageError>;
    auto clearBindings() -> std::expected<void, StorageError>;

private:
    friend class Database;
    explicit Statement(sqlite3_stmt* stmt) noexcept;

    sqlite3_stmt* stmt_{nullptr};
};

}  // namespace easyenglish::core::storage
