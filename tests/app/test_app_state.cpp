#include <cstring>
#include <expected>
#include <memory>
#include <string>
#include <string_view>
#include <vector>

#include <gtest/gtest.h>

#include "app/AppState.hpp"
#include "core/dictionary/Entry.hpp"
#include "core/dictionary/IDictionary.hpp"
#include "core/favorites/FavoritesStore.hpp"
#include "core/history/HistoryStore.hpp"
#include "core/storage/Database.hpp"

using easyenglish::app::AppState;
using easyenglish::core::dictionary::DictError;
using easyenglish::core::dictionary::Entry;
using easyenglish::core::dictionary::IDictionary;
using easyenglish::core::favorites::FavoritesStore;
using easyenglish::core::history::HistoryStore;
using easyenglish::core::storage::Database;

namespace {

/// Fake IDictionary used as the "local" dictionary in tests.
class FakeDictionary final : public IDictionary {
public:
    auto lookup(std::string_view word) const -> std::expected<Entry, DictError> override {
        ++calls_;
        if (word.empty()) {
            return std::unexpected(DictError::InvalidInput);
        }
        if (forced_error_.has_value()) {
            return std::unexpected(forced_error_.value());
        }
        if (word == hit_word_) {
            return canned_;
        }
        return std::unexpected(DictError::NotFound);
    }
    auto suggest(std::string_view /*prefix*/, std::size_t /*max*/) const
        -> std::vector<std::string> override {
        return {};
    }

    void setHit(std::string word, Entry entry) {
        hit_word_ = std::move(word);
        canned_ = std::move(entry);
    }
    void forceError(DictError e) { forced_error_ = e; }
    [[nodiscard]] int calls() const { return calls_; }

private:
    mutable int calls_{0};
    std::string hit_word_;
    Entry canned_;
    std::optional<DictError> forced_error_;
};

std::shared_ptr<HistoryStore> openMemHistory() {
    auto db = Database::open(std::string(Database::kInMemory));
    return std::make_shared<HistoryStore>(
        std::move(HistoryStore::open(std::move(db.value())).value()));
}

void typeInto(AppState& s, std::string_view text) {
    std::memset(s.input_buffer.data(), 0, s.input_buffer.size());
    std::memcpy(s.input_buffer.data(), text.data(),
                std::min(text.size(), s.input_buffer.size() - 1));
}

}  // namespace

TEST(AppState, DefaultStateIsEmpty) {
    AppState s(std::make_shared<FakeDictionary>());
    EXPECT_TRUE(s.currentHeadword().empty());
    EXPECT_TRUE(s.currentTranslations().empty());
    EXPECT_FALSE(s.hasResults());
    EXPECT_FALSE(s.inputIsNonEmpty());
}

TEST(AppState, InputIsNonEmptyDetectsTrimmedContent) {
    AppState s(std::make_shared<FakeDictionary>());
    typeInto(s, "   ");
    EXPECT_FALSE(s.inputIsNonEmpty());
    typeInto(s, "  apple ");
    EXPECT_TRUE(s.inputIsNonEmpty());
}

TEST(AppState, LocalHitPopulatesChineseTranslations) {
    auto local = std::make_shared<FakeDictionary>();
    local->setHit("apple", {"apple", "/ˈæp.əl/", {"苹果", "苹果树"}});
    AppState s(local);

    typeInto(s, "apple");
    s.submitSearch();

    EXPECT_EQ(s.currentHeadword(), "apple");
    EXPECT_EQ(s.currentPhonetic(), "/ˈæp.əl/");
    ASSERT_EQ(s.currentTranslations().size(), 2u);
    EXPECT_EQ(s.currentTranslations()[0], "苹果");
    EXPECT_EQ(s.currentTranslations()[1], "苹果树");
    EXPECT_EQ(s.status(), "Found");
    EXPECT_TRUE(s.hasResults());
}

TEST(AppState, FallsBackToOnlineWhenLocalMisses) {
    auto local = std::make_shared<FakeDictionary>();
    auto online = std::make_shared<FakeDictionary>();
    online->setHit("apple", {"apple", "", {"苹果"}});

    AppState s(local, online);
    typeInto(s, "apple");
    s.submitSearch();

    EXPECT_EQ(local->calls(), 1) << "local must be queried first";
    EXPECT_EQ(online->calls(), 1) << "online must be queried when local misses";
    ASSERT_FALSE(s.currentTranslations().empty());
    EXPECT_EQ(s.currentTranslations().front(), "苹果");
    EXPECT_EQ(s.status(), "Found");
}

TEST(AppState, OnlineNotConsultedWhenLocalHits) {
    auto local = std::make_shared<FakeDictionary>();
    auto online = std::make_shared<FakeDictionary>();
    local->setHit("apple", {"apple", "", {"苹果"}});
    AppState s(local, online);

    typeInto(s, "apple");
    s.submitSearch();

    EXPECT_EQ(local->calls(), 1);
    EXPECT_EQ(online->calls(), 0);
}

TEST(AppState, BothMissShowsNotFound) {
    AppState s(std::make_shared<FakeDictionary>(), std::make_shared<FakeDictionary>());
    typeInto(s, "nosuch");
    s.submitSearch();

    EXPECT_TRUE(s.currentTranslations().empty());
    EXPECT_NE(s.status().find("Not found"), std::string::npos);
    EXPECT_EQ(s.currentHeadword(), "nosuch");
}

TEST(AppState, EmptySearchIsNoop) {
    auto local = std::make_shared<FakeDictionary>();
    AppState s(local);
    typeInto(s, "");
    s.submitSearch();
    EXPECT_EQ(local->calls(), 0);
    EXPECT_TRUE(s.status().empty());
}

TEST(AppState, ResetClearsBufferAndResults) {
    auto local = std::make_shared<FakeDictionary>();
    local->setHit("apple", {"apple", "", {"苹果"}});
    AppState s(local);

    typeInto(s, "apple");
    s.submitSearch();
    EXPECT_TRUE(s.hasResults());

    s.reset();
    EXPECT_FALSE(s.hasResults());
    EXPECT_TRUE(s.currentHeadword().empty());
    EXPECT_FALSE(s.inputIsNonEmpty());
}

TEST(AppState, HistoryIsRecordedOnSuccessfulHit) {
    auto local = std::make_shared<FakeDictionary>();
    local->setHit("apple", {"apple", "", {"苹果"}});
    auto hist = openMemHistory();
    AppState s(local, nullptr, hist);

    typeInto(s, "apple");
    s.submitSearch();

    auto recent = hist->recent();
    ASSERT_TRUE(recent.has_value());
    ASSERT_EQ(recent->size(), 1u);
    EXPECT_EQ(recent->at(0).headword, "apple");
}

TEST(AppState, HistoryNotRecordedOnMiss) {
    auto local = std::make_shared<FakeDictionary>();
    auto hist = openMemHistory();
    AppState s(local, nullptr, hist);
    typeInto(s, "nosuch");
    s.submitSearch();
    auto recent = hist->recent();
    ASSERT_TRUE(recent.has_value());
    EXPECT_TRUE(recent->empty());
}

TEST(AppState, TranslationListIsCappedAtMax) {
    auto local = std::make_shared<FakeDictionary>();
    std::vector<std::string> many(20);
    for (std::size_t i = 0; i < many.size(); ++i) {
        many[i] = "翻译" + std::to_string(i);
    }
    local->setHit("apple", {"apple", "", many});
    AppState s(local);
    typeInto(s, "apple");
    s.submitSearch();
    EXPECT_EQ(s.currentTranslations().size(), AppState::kMaxTranslations);
}

TEST(AppState, StorageErrorFromLocalSurfacesAsStatus) {
    auto local = std::make_shared<FakeDictionary>();
    local->forceError(DictError::StorageError);
    AppState s(local);
    typeInto(s, "apple");
    s.submitSearch();
    EXPECT_NE(s.status().find("storage error"), std::string::npos);
}
