#include <filesystem>

#include <gtest/gtest.h>

#include "core/storage/Database.hpp"

#ifndef EASYENGLISH_FIXTURES_DIR
#error "EASYENGLISH_FIXTURES_DIR must be defined by the build system"
#endif

namespace fs = std::filesystem;
using easyenglish::core::storage::Database;

namespace {

constexpr int kExpectedEntryCount = 54;

fs::path miniDictPath() {
    return fs::path(EASYENGLISH_FIXTURES_DIR) / "mini_dict.sqlite";
}

}  // namespace

TEST(MiniDictFixture, OpensSuccessfully) {
    ASSERT_TRUE(fs::exists(miniDictPath())) << "Fixture missing at " << miniDictPath()
                                            << " — regenerate with `python tools/seed_db.py`";
    auto db = Database::open(miniDictPath());
    ASSERT_TRUE(db.has_value());
}

TEST(MiniDictFixture, ContainsExpectedNumberOfEntries) {
    auto db = Database::open(miniDictPath());
    ASSERT_TRUE(db.has_value());

    auto stmt_or = db.value().prepare("SELECT COUNT(*) FROM entries;");
    ASSERT_TRUE(stmt_or.has_value());
    auto& stmt = stmt_or.value();

    ASSERT_TRUE(stmt.step().value());
    EXPECT_EQ(stmt.columnInt64(0), kExpectedEntryCount)
        << "Fixture entry count drifted from the seeded list. "
           "Update tools/seed_db.py WORDS or kExpectedEntryCount, not both silently.";
}

TEST(MiniDictFixture, LookupAppleReturnsExpectedPhonetic) {
    auto db = Database::open(miniDictPath());
    ASSERT_TRUE(db.has_value());

    auto stmt_or =
        db.value().prepare("SELECT phonetic FROM entries WHERE headword = ?1 COLLATE NOCASE;");
    ASSERT_TRUE(stmt_or.has_value());
    auto& stmt = stmt_or.value();

    ASSERT_TRUE(stmt.bind(1, std::string_view("apple")).has_value());
    ASSERT_TRUE(stmt.step().value());
    EXPECT_EQ(stmt.columnText(0), "/ˈæp.əl/");
}

TEST(MiniDictFixture, LookupIsCaseInsensitive) {
    auto db = Database::open(miniDictPath());
    ASSERT_TRUE(db.has_value());

    auto stmt_or =
        db.value().prepare("SELECT headword FROM entries WHERE headword = ?1 COLLATE NOCASE;");
    ASSERT_TRUE(stmt_or.has_value());
    auto& stmt = stmt_or.value();

    ASSERT_TRUE(stmt.bind(1, std::string_view("APPLE")).has_value());
    ASSERT_TRUE(stmt.step().value());
    EXPECT_EQ(stmt.columnText(0), "apple");
}
