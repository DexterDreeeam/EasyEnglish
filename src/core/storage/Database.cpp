#include "core/storage/Database.hpp"

#include <utility>

#include <sqlite3.h>

namespace easyenglish::core::storage {

namespace {

/// Map a non-OK SQLite primary result code to our StorageError taxonomy.
StorageError mapResultCode(int rc) noexcept {
    switch (rc & 0xFF) {  // primary code is the low 8 bits
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

Database::Database(sqlite3* db) noexcept : db_(db) {}

Database::Database(Database&& other) noexcept : db_(other.db_) {
    other.db_ = nullptr;
}

Database& Database::operator=(Database&& other) noexcept {
    if (this != &other) {
        if (db_ != nullptr) {
            sqlite3_close(db_);
        }
        db_ = other.db_;
        other.db_ = nullptr;
    }
    return *this;
}

Database::~Database() {
    if (db_ != nullptr) {
        sqlite3_close(db_);
    }
}

auto Database::open(const std::filesystem::path& file) -> std::expected<Database, StorageError> {
    const auto path_str = file.string();
    const bool in_memory = (path_str == kInMemory);

    // Disallow implicit file creation: the storage layer is read/write on existing
    // files but does NOT bootstrap new ones — that is the migration layer's job.
    if (!in_memory) {
        std::error_code ec;
        if (!std::filesystem::exists(file, ec) || ec) {
            return std::unexpected(StorageError::IoError);
        }
    }

    sqlite3* raw = nullptr;
    const int flags = SQLITE_OPEN_READWRITE | SQLITE_OPEN_URI |
                      (in_memory ? SQLITE_OPEN_MEMORY | SQLITE_OPEN_CREATE : 0);
    const int rc = sqlite3_open_v2(path_str.c_str(), &raw, flags, nullptr);
    if (rc != SQLITE_OK) {
        if (raw != nullptr) {
            sqlite3_close(raw);
        }
        return std::unexpected(mapResultCode(rc));
    }

    // Sensible defaults: foreign keys ON; busy timeout 250ms for retry-on-Busy callers.
    sqlite3_busy_timeout(raw, 250);
    (void)sqlite3_exec(raw, "PRAGMA foreign_keys = ON;", nullptr, nullptr, nullptr);

    return Database{raw};
}

auto Database::execute(std::string_view sql) -> std::expected<void, StorageError> {
    if (db_ == nullptr) {
        return std::unexpected(StorageError::InvalidQuery);
    }
    // sqlite3_exec requires a null-terminated string; copy into a small std::string.
    const std::string owned(sql);
    char* err = nullptr;
    const int rc = sqlite3_exec(db_, owned.c_str(), nullptr, nullptr, &err);
    if (err != nullptr) {
        sqlite3_free(err);
    }
    if (rc != SQLITE_OK) {
        return std::unexpected(mapResultCode(rc));
    }
    return {};
}

auto Database::prepare(std::string_view sql) -> std::expected<Statement, StorageError> {
    if (db_ == nullptr) {
        return std::unexpected(StorageError::InvalidQuery);
    }
    sqlite3_stmt* stmt = nullptr;
    const int rc =
        sqlite3_prepare_v2(db_, sql.data(), static_cast<int>(sql.size()), &stmt, nullptr);
    if (rc != SQLITE_OK) {
        if (stmt != nullptr) {
            sqlite3_finalize(stmt);
        }
        return std::unexpected(mapResultCode(rc));
    }
    return Statement{stmt};
}

}  // namespace easyenglish::core::storage
