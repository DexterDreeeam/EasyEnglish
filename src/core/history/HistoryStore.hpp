#pragma once

#include <cstddef>
#include <cstdint>
#include <expected>
#include <string>
#include <string_view>
#include <vector>

#include "core/storage/Database.hpp"
#include "core/storage/Statement.hpp"

namespace easyenglish::core::history {

enum class HistoryError {
    InvalidInput,
    StorageError,
};

struct HistoryEntry {
    std::string headword;
    std::int64_t last_at_unix{0};
    std::int64_t count{0};

    friend bool operator==(const HistoryEntry&, const HistoryEntry&) = default;
};

/// Persistent query-history store backed by a SQLite table.
///
/// Schema is auto-created on `open()` so callers do not need a separate
/// migration step. `record()` is upsert: new headwords get count=1, existing
/// headwords increment count and update last_at.
class HistoryStore {
public:
    static constexpr std::size_t kDefaultRecent = 20;

    static auto open(storage::Database db) -> std::expected<HistoryStore, HistoryError>;

    HistoryStore(const HistoryStore&) = delete;
    HistoryStore& operator=(const HistoryStore&) = delete;
    HistoryStore(HistoryStore&&) noexcept;
    HistoryStore& operator=(HistoryStore&&) noexcept;
    ~HistoryStore();

    /// Upsert: increments count for an existing headword (case-insensitive),
    /// inserts new ones. Updates `last_at` to "now" in unix seconds.
    auto record(std::string_view word) -> std::expected<void, HistoryError>;

    /// Most-recently-used words, newest first. `max` caps the result length.
    auto recent(std::size_t max = kDefaultRecent) const
        -> std::expected<std::vector<HistoryEntry>, HistoryError>;

    /// Drop every recorded entry.
    auto clear() -> std::expected<void, HistoryError>;

private:
    HistoryStore(storage::Database db, storage::Statement insert_stmt,
                 storage::Statement recent_stmt, storage::Statement clear_stmt) noexcept;

    static constexpr std::size_t kMaxWordLen = 128;

    storage::Database db_;
    mutable storage::Statement insert_stmt_;
    mutable storage::Statement recent_stmt_;
    mutable storage::Statement clear_stmt_;
};

}  // namespace easyenglish::core::history
