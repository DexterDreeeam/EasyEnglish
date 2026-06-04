#include <algorithm>
#include <filesystem>
#include <fstream>
#include <sstream>

#include <gtest/gtest.h>
#include <nlohmann/json.hpp>

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

std::vector<std::string> readGolden(const std::string& name) {
    const fs::path p = fs::path(EASYENGLISH_FIXTURES_DIR) / "fuzzy" / (name + ".golden");
    std::ifstream in(p);
    std::stringstream ss;
    ss << in.rdbuf();
    const auto doc = nlohmann::json::parse(ss.str(), nullptr, false);
    std::vector<std::string> out;
    if (doc.is_discarded() || !doc.is_array()) {
        return out;
    }
    for (const auto& v : doc) {
        if (v.is_string()) {
            out.push_back(v.get<std::string>());
        }
    }
    return out;
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

TEST(SqliteDictionarySuggest, EmptyPrefixReturnsEmpty) {
    auto dict = openMiniDict();
    EXPECT_TRUE(dict.suggest("").empty());
}

TEST(SqliteDictionarySuggest, ExactMatchIsFirstResult) {
    auto dict = openMiniDict();
    auto results = dict.suggest("apple", 5);
    ASSERT_FALSE(results.empty());
    EXPECT_EQ(results.front(), "apple")
        << "An exact match (Levenshtein distance 0) must rank ahead of any near miss";
}

TEST(SqliteDictionarySuggest, RespectsMaxLimit) {
    auto dict = openMiniDict();
    auto results = dict.suggest("z", 3);
    EXPECT_LE(results.size(), 3u);
}

TEST(SqliteDictionarySuggest, IsCaseInsensitive) {
    auto dict = openMiniDict();
    auto a = dict.suggest("APPL", 3);
    auto b = dict.suggest("appl", 3);
    EXPECT_EQ(a, b);
}

TEST(SqliteDictionarySuggest, GoldenAppl) {
    auto dict = openMiniDict();
    const auto golden = readGolden("appl");
    ASSERT_FALSE(golden.empty()) << "Could not read appl.golden fixture";
    auto results = dict.suggest("appl", golden.size());
    EXPECT_EQ(results, golden)
        << "Golden drift. If this change is intentional, update tests/fixtures/fuzzy/appl.golden "
           "AND review whether the ranking algorithm change was deliberate.";
}

TEST(SqliteDictionarySuggest, GoldenBanaba) {
    // Typo of 'banana' — checks that distance-1 hit ranks first.
    auto dict = openMiniDict();
    const auto golden = readGolden("banaba");
    ASSERT_FALSE(golden.empty()) << "Could not read banaba.golden fixture";
    auto results = dict.suggest("banaba", golden.size());
    ASSERT_FALSE(results.empty());
    EXPECT_EQ(results.front(), "banana") << "Distance-1 hit must rank first";
}

TEST(SqliteDictionarySuggest, NeverReturnsErrorForUnknownPrefix) {
    auto dict = openMiniDict();
    auto results = dict.suggest("xyzzynosuch", 5);
    // Contract: suggest() never errors — always returns a (possibly empty) vector.
    // For a non-empty corpus we expect at least one suggestion (the closest word).
    EXPECT_FALSE(results.empty());
}
