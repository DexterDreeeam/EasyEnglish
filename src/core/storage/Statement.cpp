#include "core/storage/Statement.hpp"

#include <utility>

#include <sqlite3.h>

namespace easyenglish::core::storage {

namespace {

StorageError mapResultCode(int rc) noexcept {
    switch (rc & 0xFF) {
        case SQLITE_BUSY:
        case SQLITE_LOCKED:
            return StorageError::Busy;
        case SQLITE_CONSTRAINT:
            return StorageError::ConstraintViolation;
        case SQLITE_NOTFOUND:
            return StorageError::NotFound;
        case SQLITE_PERM:
        case SQLITE_CANTOPEN:
        case SQLITE_IOERR:
        case SQLITE_FULL:
        case SQLITE_READONLY:
            return StorageError::IoError;
        default:
            return StorageError::InvalidQuery;
    }
}

}  // namespace

Statement::Statement(sqlite3_stmt* stmt) noexcept : stmt_(stmt) {}

Statement::Statement(Statement&& other) noexcept : stmt_(other.stmt_) {
    other.stmt_ = nullptr;
}

Statement& Statement::operator=(Statement&& other) noexcept {
    if (this != &other) {
        if (stmt_ != nullptr) {
            sqlite3_finalize(stmt_);
        }
        stmt_ = other.stmt_;
        other.stmt_ = nullptr;
    }
    return *this;
}

Statement::~Statement() {
    if (stmt_ != nullptr) {
        sqlite3_finalize(stmt_);
    }
}

auto Statement::bind(int index, std::int64_t value) -> std::expected<void, StorageError> {
    const int rc = sqlite3_bind_int64(stmt_, index, value);
    if (rc != SQLITE_OK) {
        return std::unexpected(mapResultCode(rc));
    }
    return {};
}

auto Statement::bind(int index, double value) -> std::expected<void, StorageError> {
    const int rc = sqlite3_bind_double(stmt_, index, value);
    if (rc != SQLITE_OK) {
        return std::unexpected(mapResultCode(rc));
    }
    return {};
}

auto Statement::bind(int index, std::string_view value) -> std::expected<void, StorageError> {
    // SQLITE_TRANSIENT tells SQLite to copy the buffer; safe because string_view
    // may dangle right after the call returns.
    const int rc = sqlite3_bind_text(stmt_, index, value.data(), static_cast<int>(value.size()),
                                     SQLITE_TRANSIENT);
    if (rc != SQLITE_OK) {
        return std::unexpected(mapResultCode(rc));
    }
    return {};
}

auto Statement::bindNull(int index) -> std::expected<void, StorageError> {
    const int rc = sqlite3_bind_null(stmt_, index);
    if (rc != SQLITE_OK) {
        return std::unexpected(mapResultCode(rc));
    }
    return {};
}

auto Statement::step() -> std::expected<bool, StorageError> {
    const int rc = sqlite3_step(stmt_);
    if (rc == SQLITE_ROW) {
        return true;
    }
    if (rc == SQLITE_DONE) {
        return false;
    }
    return std::unexpected(mapResultCode(rc));
}

auto Statement::columnInt64(int index) const -> std::int64_t {
    return sqlite3_column_int64(stmt_, index);
}

auto Statement::columnDouble(int index) const -> double {
    return sqlite3_column_double(stmt_, index);
}

auto Statement::columnText(int index) const -> std::string {
    const auto* text = sqlite3_column_text(stmt_, index);
    const int size = sqlite3_column_bytes(stmt_, index);
    if (text == nullptr) {
        return {};
    }
    return std::string(reinterpret_cast<const char*>(text), static_cast<std::size_t>(size));
}

auto Statement::isColumnNull(int index) const -> bool {
    return sqlite3_column_type(stmt_, index) == SQLITE_NULL;
}

auto Statement::reset() -> std::expected<void, StorageError> {
    const int rc = sqlite3_reset(stmt_);
    if (rc != SQLITE_OK) {
        return std::unexpected(mapResultCode(rc));
    }
    return {};
}

auto Statement::clearBindings() -> std::expected<void, StorageError> {
    const int rc = sqlite3_clear_bindings(stmt_);
    if (rc != SQLITE_OK) {
        return std::unexpected(mapResultCode(rc));
    }
    return {};
}

}  // namespace easyenglish::core::storage
