#include "core/favorites/FavoritesStore.hpp"

#include <chrono>
#include <utility>

namespace easyenglish::core::favorites {

namespace {

constexpr const char* kCreateSchema =
    "CREATE TABLE IF NOT EXISTS favorites ("
    "    headword TEXT PRIMARY KEY COLLATE NOCASE,"
    "    added_at INTEGER NOT NULL"
    ");"
    "CREATE INDEX IF NOT EXISTS idx_favorites_added_at "
    "    ON favorites(added_at DESC);";

constexpr const char* kAddSql =
    "INSERT INTO favorites(headword, added_at) VALUES(?1, ?2) "
    "ON CONFLICT(headword) DO NOTHING;";

constexpr const char* kRemoveSql = "DELETE FROM favorites WHERE headword = ?1 COLLATE NOCASE;";

constexpr const char* kContainsSql =
    "SELECT 1 FROM favorites WHERE headword = ?1 COLLATE NOCASE LIMIT 1;";

constexpr const char* kListSql =
    "SELECT headword, added_at FROM favorites ORDER BY added_at DESC LIMIT ?1;";

FavoritesError mapStorageError(storage::StorageError /*e*/) noexcept {
    return FavoritesError::StorageError;
}

std::int64_t nowUnix() {
    return std::chrono::duration_cast<std::chrono::seconds>(
               std::chrono::system_clock::now().time_since_epoch())
        .count();
}

}  // namespace

FavoritesStore::FavoritesStore(storage::Database db, storage::Statement add_stmt,
                               storage::Statement remove_stmt, storage::Statement contains_stmt,
                               storage::Statement list_stmt) noexcept
    : db_(std::move(db)),
      add_stmt_(std::move(add_stmt)),
      remove_stmt_(std::move(remove_stmt)),
      contains_stmt_(std::move(contains_stmt)),
      list_stmt_(std::move(list_stmt)) {}

FavoritesStore::FavoritesStore(FavoritesStore&&) noexcept = default;
FavoritesStore& FavoritesStore::operator=(FavoritesStore&&) noexcept = default;
FavoritesStore::~FavoritesStore() = default;

auto FavoritesStore::open(storage::Database db) -> std::expected<FavoritesStore, FavoritesError> {
    if (auto r = db.execute(kCreateSchema); !r) {
        return std::unexpected(mapStorageError(r.error()));
    }
    auto add_or = db.prepare(kAddSql);
    if (!add_or) {
        return std::unexpected(mapStorageError(add_or.error()));
    }
    auto remove_or = db.prepare(kRemoveSql);
    if (!remove_or) {
        return std::unexpected(mapStorageError(remove_or.error()));
    }
    auto contains_or = db.prepare(kContainsSql);
    if (!contains_or) {
        return std::unexpected(mapStorageError(contains_or.error()));
    }
    auto list_or = db.prepare(kListSql);
    if (!list_or) {
        return std::unexpected(mapStorageError(list_or.error()));
    }
    return FavoritesStore(std::move(db), std::move(add_or.value()), std::move(remove_or.value()),
                          std::move(contains_or.value()), std::move(list_or.value()));
}

auto FavoritesStore::add(std::string_view word) -> std::expected<void, FavoritesError> {
    if (word.empty() || word.size() > kMaxWordLen) {
        return std::unexpected(FavoritesError::InvalidInput);
    }
    if (auto r = add_stmt_.reset(); !r)
        return std::unexpected(mapStorageError(r.error()));
    if (auto r = add_stmt_.clearBindings(); !r)
        return std::unexpected(mapStorageError(r.error()));
    if (auto r = add_stmt_.bind(1, word); !r)
        return std::unexpected(mapStorageError(r.error()));
    if (auto r = add_stmt_.bind(2, nowUnix()); !r)
        return std::unexpected(mapStorageError(r.error()));
    auto step = add_stmt_.step();
    if (!step)
        return std::unexpected(mapStorageError(step.error()));
    return {};
}

auto FavoritesStore::remove(std::string_view word) -> std::expected<void, FavoritesError> {
    if (word.empty() || word.size() > kMaxWordLen) {
        return std::unexpected(FavoritesError::InvalidInput);
    }
    if (auto r = remove_stmt_.reset(); !r)
        return std::unexpected(mapStorageError(r.error()));
    if (auto r = remove_stmt_.clearBindings(); !r)
        return std::unexpected(mapStorageError(r.error()));
    if (auto r = remove_stmt_.bind(1, word); !r)
        return std::unexpected(mapStorageError(r.error()));
    auto step = remove_stmt_.step();
    if (!step)
        return std::unexpected(mapStorageError(step.error()));
    return {};
}

auto FavoritesStore::contains(std::string_view word) const -> std::expected<bool, FavoritesError> {
    if (word.empty() || word.size() > kMaxWordLen) {
        return std::unexpected(FavoritesError::InvalidInput);
    }
    if (auto r = contains_stmt_.reset(); !r)
        return std::unexpected(mapStorageError(r.error()));
    if (auto r = contains_stmt_.clearBindings(); !r)
        return std::unexpected(mapStorageError(r.error()));
    if (auto r = contains_stmt_.bind(1, word); !r)
        return std::unexpected(mapStorageError(r.error()));
    auto step = contains_stmt_.step();
    if (!step)
        return std::unexpected(mapStorageError(step.error()));
    return step.value();
}

auto FavoritesStore::list(std::size_t max) const
    -> std::expected<std::vector<FavoriteEntry>, FavoritesError> {
    std::vector<FavoriteEntry> out;
    if (max == 0) {
        return out;
    }
    if (auto r = list_stmt_.reset(); !r)
        return std::unexpected(mapStorageError(r.error()));
    if (auto r = list_stmt_.clearBindings(); !r)
        return std::unexpected(mapStorageError(r.error()));
    if (auto r = list_stmt_.bind(1, static_cast<std::int64_t>(max)); !r)
        return std::unexpected(mapStorageError(r.error()));
    while (true) {
        auto step = list_stmt_.step();
        if (!step)
            return std::unexpected(mapStorageError(step.error()));
        if (!step.value())
            break;
        FavoriteEntry e;
        e.headword = list_stmt_.columnText(0);
        e.added_at_unix = list_stmt_.columnInt64(1);
        out.push_back(std::move(e));
    }
    return out;
}

}  // namespace easyenglish::core::favorites
