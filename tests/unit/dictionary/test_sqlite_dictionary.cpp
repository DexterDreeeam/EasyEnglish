#include <filesystem>

#include <gtest/gtest.h>

#include "core/dictionary/SqliteDictionary.hpp"
#include "core/storage/Database.hpp"

#ifndef EASYENGLISH_FIXTURES_DIR
#error "EASYENGLISH_FIXTURES_DIR must be defined by the build system"
#endif

namespace fs = std::filesystem;
using easyenglish::core::dictionary::DictError;
using easyenglish::core::dictionary::SqliteDictionary;
using easyenglish::core::storage::Database;

namespace {

SqliteDictionary openMiniDict() {
    auto db = Database::open(fs::path(EASYENGLISH_FIXTURES_DIR) / "mini_dict.sqlite");
    EXPECT_TRUE(db.has_value());
    auto dict = SqliteDictionary::open(std::move(db.value()));
    EXPECT_TRUE(dict.has_value());
    return std::move(dict.value());
}

}  // namespace

TEST(SqliteDictionaryLookup, ReturnsEntryForKnownWord) {
    auto dict = openMiniDict();
    auto entry = dict.lookup("apple");
    ASSERT_TRUE(entry.has_value()) << "apple should be present in mini_dict";
    EXPECT_EQ(entry->headword, "apple");
    EXPECT_EQ(entry->phonetic, "/ˈæp.əl/");
    ASSERT_FALSE(entry->definitions.empty());
    EXPECT_EQ(entry->definitions.front(), "a round fruit with red or green skin");
}

TEST(SqliteDictionaryLookup, IsCaseInsensitive) {
    auto dict = openMiniDict();
    auto entry = dict.lookup("APPLE");
    ASSERT_TRUE(entry.has_value());
    EXPECT_EQ(entry->headword, "apple")
        << "Canonical headword casing should be returned, not the caller's input";
}

TEST(SqliteDictionaryLookup, RejectsEmptyInput) {
    auto dict = openMiniDict();
    auto result = dict.lookup("");
    ASSERT_FALSE(result.has_value());
    EXPECT_EQ(result.error(), DictError::InvalidInput);
}

TEST(SqliteDictionaryLookup, RejectsOverlyLongInput) {
    auto dict = openMiniDict();
    const std::string too_long(200, 'x');
    auto result = dict.lookup(too_long);
    ASSERT_FALSE(result.has_value());
    EXPECT_EQ(result.error(), DictError::InvalidInput);
}

TEST(SqliteDictionaryLookup, ReturnsNotFoundForUnknownWord) {
    auto dict = openMiniDict();
    auto result = dict.lookup("xyzzy_no_such_word");
    ASSERT_FALSE(result.has_value());
    EXPECT_EQ(result.error(), DictError::NotFound);
}

TEST(SqliteDictionaryLookup, StatementIsReusableAcrossCalls) {
    // Exercises the prepared-statement cache + reset/clearBindings path.
    auto dict = openMiniDict();
    for (int i = 0; i < 5; ++i) {
        auto a = dict.lookup("apple");
        auto b = dict.lookup("banana");
        ASSERT_TRUE(a.has_value());
        ASSERT_TRUE(b.has_value());
        EXPECT_EQ(a->headword, "apple");
        EXPECT_EQ(b->headword, "banana");
    }
}

TEST(SqliteDictionarySuggest, IsStubbedToEmpty) {
    // suggest() is intentionally a stub until iter-006-fuzzy. The contract
    // explicitly allows an empty vector here; this test pins that behavior so
    // that iter-006 must update the assertion deliberately rather than by
    // accidental drift.
    auto dict = openMiniDict();
    EXPECT_TRUE(dict.suggest("").empty());
    EXPECT_TRUE(dict.suggest("appl").empty());
}
