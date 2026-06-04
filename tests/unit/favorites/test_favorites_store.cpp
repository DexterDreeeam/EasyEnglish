#include <gtest/gtest.h>

#include "core/favorites/FavoritesStore.hpp"
#include "core/storage/Database.hpp"

using easyenglish::core::favorites::FavoritesError;
using easyenglish::core::favorites::FavoritesStore;
using easyenglish::core::storage::Database;

namespace {

FavoritesStore openEmpty() {
    auto db = Database::open(std::string(Database::kInMemory));
    EXPECT_TRUE(db.has_value());
    auto fav = FavoritesStore::open(std::move(db.value()));
    EXPECT_TRUE(fav.has_value());
    return std::move(fav.value());
}

}  // namespace

TEST(FavoritesStore, EmptyByDefault) {
    auto f = openEmpty();
    auto list = f.list();
    ASSERT_TRUE(list.has_value());
    EXPECT_TRUE(list->empty());
}

TEST(FavoritesStore, AddMakesContainsTrue) {
    auto f = openEmpty();
    ASSERT_TRUE(f.add("apple").has_value());
    auto contains = f.contains("apple");
    ASSERT_TRUE(contains.has_value());
    EXPECT_TRUE(contains.value());
}

TEST(FavoritesStore, AddIsIdempotent) {
    auto f = openEmpty();
    ASSERT_TRUE(f.add("apple").has_value());
    ASSERT_TRUE(f.add("apple").has_value());
    auto list = f.list();
    ASSERT_TRUE(list.has_value());
    EXPECT_EQ(list->size(), 1u);
}

TEST(FavoritesStore, ContainsIsCaseInsensitive) {
    auto f = openEmpty();
    ASSERT_TRUE(f.add("Apple").has_value());
    auto contains = f.contains("APPLE");
    ASSERT_TRUE(contains.has_value());
    EXPECT_TRUE(contains.value());
}

TEST(FavoritesStore, RemoveTakesEffect) {
    auto f = openEmpty();
    ASSERT_TRUE(f.add("apple").has_value());
    ASSERT_TRUE(f.remove("apple").has_value());

    auto contains = f.contains("apple");
    ASSERT_TRUE(contains.has_value());
    EXPECT_FALSE(contains.value());
}

TEST(FavoritesStore, RemoveOfUnknownWordSucceeds) {
    auto f = openEmpty();
    auto result = f.remove("nosuch");
    EXPECT_TRUE(result.has_value());
}

TEST(FavoritesStore, ListLimitedByMax) {
    auto f = openEmpty();
    for (int i = 0; i < 5; ++i) {
        ASSERT_TRUE(f.add("word" + std::to_string(i)).has_value());
    }
    auto list = f.list(3);
    ASSERT_TRUE(list.has_value());
    EXPECT_EQ(list->size(), 3u);
}

TEST(FavoritesStore, RejectsEmptyInput) {
    auto f = openEmpty();
    EXPECT_EQ(f.add("").error(), FavoritesError::InvalidInput);
    EXPECT_EQ(f.remove("").error(), FavoritesError::InvalidInput);
    EXPECT_EQ(f.contains("").error(), FavoritesError::InvalidInput);
}
