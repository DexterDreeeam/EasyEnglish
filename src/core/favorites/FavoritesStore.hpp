#pragma once

#include <cstddef>
#include <cstdint>
#include <expected>
#include <string>
#include <string_view>
#include <vector>

#include "core/storage/Database.hpp"
#include "core/storage/Statement.hpp"

namespace easyenglish::core::favorites {

enum class FavoritesError {
    InvalidInput,
    StorageError,
};

struct FavoriteEntry {
    std::string headword;
    std::int64_t added_at_unix{0};

    friend bool operator==(const FavoriteEntry&, const FavoriteEntry&) = default;
};

/// Persistent set of bookmarked words. Backed by SQLite; schema is created on
/// `open()` so the store can be plugged into any `Database`.
class FavoritesStore {
public:
    static constexpr std::size_t kDefaultListLimit = 200;

    static auto open(storage::Database db) -> std::expected<FavoritesStore, FavoritesError>;

    FavoritesStore(const FavoritesStore&) = delete;
    FavoritesStore& operator=(const FavoritesStore&) = delete;
    FavoritesStore(FavoritesStore&&) noexcept;
    FavoritesStore& operator=(FavoritesStore&&) noexcept;
    ~FavoritesStore();

    /// Add a word. Already-favorited words succeed silently (idempotent).
    auto add(std::string_view word) -> std::expected<void, FavoritesError>;

    /// Remove a word. Removing a non-favorited word succeeds silently.
    auto remove(std::string_view word) -> std::expected<void, FavoritesError>;

    /// `true` if the word is currently favorited (case-insensitive).
    auto contains(std::string_view word) const -> std::expected<bool, FavoritesError>;

    /// Favorites ordered by added_at descending (newest first), up to `max`.
    auto list(std::size_t max = kDefaultListLimit) const
        -> std::expected<std::vector<FavoriteEntry>, FavoritesError>;

private:
    FavoritesStore(storage::Database db, storage::Statement add_stmt,
                   storage::Statement remove_stmt, storage::Statement contains_stmt,
                   storage::Statement list_stmt) noexcept;

    static constexpr std::size_t kMaxWordLen = 128;

    storage::Database db_;
    mutable storage::Statement add_stmt_;
    mutable storage::Statement remove_stmt_;
    mutable storage::Statement contains_stmt_;
    mutable storage::Statement list_stmt_;
};

}  // namespace easyenglish::core::favorites
