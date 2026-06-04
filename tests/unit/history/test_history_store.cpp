#include <chrono>
#include <thread>

#include <gtest/gtest.h>

#include "core/history/HistoryStore.hpp"
#include "core/storage/Database.hpp"

using easyenglish::core::history::HistoryError;
using easyenglish::core::history::HistoryStore;
using easyenglish::core::storage::Database;

namespace {

HistoryStore openEmpty() {
    auto db = Database::open(std::string(Database::kInMemory));
    EXPECT_TRUE(db.has_value());
    auto h = HistoryStore::open(std::move(db.value()));
    EXPECT_TRUE(h.has_value());
    return std::move(h.value());
}

}  // namespace

TEST(HistoryStore, EmptyByDefault) {
    auto h = openEmpty();
    auto recent = h.recent();
    ASSERT_TRUE(recent.has_value());
    EXPECT_TRUE(recent->empty());
}

TEST(HistoryStore, RecordNewWordHasCountOne) {
    auto h = openEmpty();
    ASSERT_TRUE(h.record("apple").has_value());

    auto recent = h.recent();
    ASSERT_TRUE(recent.has_value());
    ASSERT_EQ(recent->size(), 1u);
    EXPECT_EQ(recent->at(0).headword, "apple");
    EXPECT_EQ(recent->at(0).count, 1);
}

TEST(HistoryStore, RecordExistingWordIncrementsCount) {
    auto h = openEmpty();
    ASSERT_TRUE(h.record("apple").has_value());
    ASSERT_TRUE(h.record("apple").has_value());
    ASSERT_TRUE(h.record("apple").has_value());

    auto recent = h.recent();
    ASSERT_TRUE(recent.has_value());
    ASSERT_EQ(recent->size(), 1u);
    EXPECT_EQ(recent->at(0).count, 3);
}

TEST(HistoryStore, RecordIsCaseInsensitive) {
    auto h = openEmpty();
    ASSERT_TRUE(h.record("Apple").has_value());
    ASSERT_TRUE(h.record("APPLE").has_value());

    auto recent = h.recent();
    ASSERT_TRUE(recent.has_value());
    ASSERT_EQ(recent->size(), 1u) << "Apple/APPLE should collapse to one row";
    EXPECT_EQ(recent->at(0).count, 2);
}

TEST(HistoryStore, RecentOrdersByMostRecent) {
    auto h = openEmpty();
    ASSERT_TRUE(h.record("first").has_value());
    // Sleep to ensure distinct unix-seconds timestamps. Without the sleep two
    // back-to-back records may land in the same second and the secondary
    // ordering (PRIMARY KEY) would dominate, masking a real bug.
    std::this_thread::sleep_for(std::chrono::milliseconds(1100));
    ASSERT_TRUE(h.record("second").has_value());
    std::this_thread::sleep_for(std::chrono::milliseconds(1100));
    ASSERT_TRUE(h.record("third").has_value());

    auto recent = h.recent();
    ASSERT_TRUE(recent.has_value());
    ASSERT_EQ(recent->size(), 3u);
    EXPECT_EQ(recent->at(0).headword, "third");
    EXPECT_EQ(recent->at(1).headword, "second");
    EXPECT_EQ(recent->at(2).headword, "first");
}

TEST(HistoryStore, RecentRespectsMaxLimit) {
    auto h = openEmpty();
    for (int i = 0; i < 10; ++i) {
        ASSERT_TRUE(h.record("word_" + std::to_string(i)).has_value());
    }
    auto recent = h.recent(3);
    ASSERT_TRUE(recent.has_value());
    EXPECT_EQ(recent->size(), 3u);
}

TEST(HistoryStore, RecentZeroReturnsEmpty) {
    auto h = openEmpty();
    ASSERT_TRUE(h.record("apple").has_value());
    auto recent = h.recent(0);
    ASSERT_TRUE(recent.has_value());
    EXPECT_TRUE(recent->empty());
}

TEST(HistoryStore, ClearRemovesAll) {
    auto h = openEmpty();
    ASSERT_TRUE(h.record("a").has_value());
    ASSERT_TRUE(h.record("b").has_value());
    ASSERT_TRUE(h.clear().has_value());

    auto recent = h.recent();
    ASSERT_TRUE(recent.has_value());
    EXPECT_TRUE(recent->empty());
}

TEST(HistoryStore, RejectsEmptyInput) {
    auto h = openEmpty();
    auto result = h.record("");
    ASSERT_FALSE(result.has_value());
    EXPECT_EQ(result.error(), HistoryError::InvalidInput);
}

TEST(HistoryStore, CreateOrOpenBootstrapsNewFile) {
    const auto tmp =
        std::filesystem::temp_directory_path() / "easyenglish_history_bootstrap.sqlite";
    std::error_code ec;
    std::filesystem::remove(tmp, ec);

    {
        auto db = Database::createOrOpen(tmp);
        ASSERT_TRUE(db.has_value());
        auto h = HistoryStore::open(std::move(db.value()));
        ASSERT_TRUE(h.has_value());
        ASSERT_TRUE(h->record("persisted").has_value());
    }

    // Re-open and verify data survived. Scope tightly so the handle is closed
    // before we try to delete the file (Windows file-locking semantics).
    {
        auto db = Database::open(tmp);
        ASSERT_TRUE(db.has_value());
        auto h = HistoryStore::open(std::move(db.value()));
        ASSERT_TRUE(h.has_value());
        auto recent = h->recent();
        ASSERT_TRUE(recent.has_value());
        ASSERT_EQ(recent->size(), 1u);
        EXPECT_EQ(recent->at(0).headword, "persisted");
    }

    std::filesystem::remove(tmp, ec);  // best-effort cleanup; ignore errors
}
