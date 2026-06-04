#include "core/history/HistoryStore.hpp"

#include <chrono>
#include <utility>

namespace easyenglish::core::history {

namespace {

constexpr const char* kCreateSchema =
    "CREATE TABLE IF NOT EXISTS history ("
    "    headword TEXT PRIMARY KEY COLLATE NOCASE,"
    "    last_at  INTEGER NOT NULL,"
    "    count    INTEGER NOT NULL DEFAULT 1"
    ");"
    "CREATE INDEX IF NOT EXISTS idx_history_last_at "
    "    ON history(last_at DESC);";

constexpr const char* kUpsertSql =
    "INSERT INTO history(headword, last_at, count) VALUES(?1, ?2, 1) "
    "ON CONFLICT(headword) DO UPDATE SET "
    "    last_at = excluded.last_at, "
    "    count = history.count + 1;";

constexpr const char* kRecentSql =
    "SELECT headword, last_at, count FROM history "
    "ORDER BY last_at DESC LIMIT ?1;";

constexpr const char* kClearSql = "DELETE FROM history;";

HistoryError mapStorageError(storage::StorageError /*e*/) noexcept {
    return HistoryError::StorageError;
}

std::int64_t nowUnix() {
    return std::chrono::duration_cast<std::chrono::seconds>(
               std::chrono::system_clock::now().time_since_epoch())
        .count();
}

}  // namespace

HistoryStore::HistoryStore(storage::Database db, storage::Statement insert_stmt,
                           storage::Statement recent_stmt, storage::Statement clear_stmt) noexcept
    : db_(std::move(db)),
      insert_stmt_(std::move(insert_stmt)),
      recent_stmt_(std::move(recent_stmt)),
      clear_stmt_(std::move(clear_stmt)) {}

HistoryStore::HistoryStore(HistoryStore&& other) noexcept = default;
HistoryStore& HistoryStore::operator=(HistoryStore&& other) noexcept = default;
HistoryStore::~HistoryStore() = default;

auto HistoryStore::open(storage::Database db) -> std::expected<HistoryStore, HistoryError> {
    if (auto r = db.execute(kCreateSchema); !r) {
        return std::unexpected(mapStorageError(r.error()));
    }
    auto insert_or = db.prepare(kUpsertSql);
    if (!insert_or) {
        return std::unexpected(mapStorageError(insert_or.error()));
    }
    auto recent_or = db.prepare(kRecentSql);
    if (!recent_or) {
        return std::unexpected(mapStorageError(recent_or.error()));
    }
    auto clear_or = db.prepare(kClearSql);
    if (!clear_or) {
        return std::unexpected(mapStorageError(clear_or.error()));
    }
    return HistoryStore(std::move(db), std::move(insert_or.value()), std::move(recent_or.value()),
                        std::move(clear_or.value()));
}

auto HistoryStore::record(std::string_view word) -> std::expected<void, HistoryError> {
    if (word.empty() || word.size() > kMaxWordLen) {
        return std::unexpected(HistoryError::InvalidInput);
    }
    if (auto r = insert_stmt_.reset(); !r) {
        return std::unexpected(mapStorageError(r.error()));
    }
    if (auto r = insert_stmt_.clearBindings(); !r) {
        return std::unexpected(mapStorageError(r.error()));
    }
    if (auto r = insert_stmt_.bind(1, word); !r) {
        return std::unexpected(mapStorageError(r.error()));
    }
    if (auto r = insert_stmt_.bind(2, nowUnix()); !r) {
        return std::unexpected(mapStorageError(r.error()));
    }
    auto step = insert_stmt_.step();
    if (!step) {
        return std::unexpected(mapStorageError(step.error()));
    }
    return {};
}

auto HistoryStore::recent(std::size_t max) const
    -> std::expected<std::vector<HistoryEntry>, HistoryError> {
    std::vector<HistoryEntry> out;
    if (max == 0) {
        return out;
    }
    if (auto r = recent_stmt_.reset(); !r) {
        return std::unexpected(mapStorageError(r.error()));
    }
    if (auto r = recent_stmt_.clearBindings(); !r) {
        return std::unexpected(mapStorageError(r.error()));
    }
    if (auto r = recent_stmt_.bind(1, static_cast<std::int64_t>(max)); !r) {
        return std::unexpected(mapStorageError(r.error()));
    }
    while (true) {
        auto step = recent_stmt_.step();
        if (!step) {
            return std::unexpected(mapStorageError(step.error()));
        }
        if (!step.value()) {
            break;
        }
        HistoryEntry e;
        e.headword = recent_stmt_.columnText(0);
        e.last_at_unix = recent_stmt_.columnInt64(1);
        e.count = recent_stmt_.columnInt64(2);
        out.push_back(std::move(e));
    }
    return out;
}

auto HistoryStore::clear() -> std::expected<void, HistoryError> {
    if (auto r = clear_stmt_.reset(); !r) {
        return std::unexpected(mapStorageError(r.error()));
    }
    auto step = clear_stmt_.step();
    if (!step) {
        return std::unexpected(mapStorageError(step.error()));
    }
    return {};
}

}  // namespace easyenglish::core::history
