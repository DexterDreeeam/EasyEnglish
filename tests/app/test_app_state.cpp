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

class FakeDictionary final : public IDictionary {
public:
    auto lookup(std::string_view word) const -> std::expected<Entry, DictError> override {
        ++calls_;
        if (word.empty()) {
            return std::unexpected(DictError::InvalidInput);
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

    [[nodiscard]] int calls() const { return calls_; }

private:
    mutable int calls_{0};
    std::string hit_word_;
    Entry canned_;
};

std::shared_ptr<HistoryStore> openMemHistory() {
    auto db = Database::open(std::string(Database::kInMemory));
    return std::make_shared<HistoryStore>(
        std::move(HistoryStore::open(std::move(db.value())).value()));
}

std::shared_ptr<FavoritesStore> openMemFavorites() {
    auto db = Database::open(std::string(Database::kInMemory));
    return std::make_shared<FavoritesStore>(
        std::move(FavoritesStore::open(std::move(db.value())).value()));
}

void typeInto(AppState& s, std::string_view text) {
    std::memset(s.input_buffer.data(), 0, s.input_buffer.size());
    std::memcpy(s.input_buffer.data(), text.data(),
                std::min(text.size(), s.input_buffer.size() - 1));
}

}  // namespace

TEST(AppState, DefaultStatusIsReady) {
    auto dict = std::make_shared<FakeDictionary>();
    AppState s(dict);
    EXPECT_EQ(s.status(), "Ready.");
    EXPECT_FALSE(s.currentEntry().has_value());
    EXPECT_FALSE(s.inputIsNonEmpty());
}

TEST(AppState, InputIsNonEmptyDetectsTrimmedContent) {
    auto dict = std::make_shared<FakeDictionary>();
    AppState s(dict);
    typeInto(s, "   ");
    EXPECT_FALSE(s.inputIsNonEmpty());
    typeInto(s, "  apple ");
    EXPECT_TRUE(s.inputIsNonEmpty());
}

TEST(AppState, SubmitSearchHitPopulatesEntry) {
    auto dict = std::make_shared<FakeDictionary>();
    dict->setHit("apple", {"apple", "/ˈæp.əl/", {"fruit"}});
    AppState s(dict);
    typeInto(s, "apple");
    s.submitSearch();

    ASSERT_TRUE(s.currentEntry().has_value());
    EXPECT_EQ(s.currentEntry()->headword, "apple");
    EXPECT_EQ(s.status(), "Found.");
    EXPECT_EQ(dict->calls(), 1);
}

TEST(AppState, SubmitSearchMissShowsNotFoundAndClearsEntry) {
    auto dict = std::make_shared<FakeDictionary>();
    AppState s(dict);
    typeInto(s, "nosuch");
    s.submitSearch();
    EXPECT_FALSE(s.currentEntry().has_value());
    EXPECT_NE(s.status().find("Not found"), std::string::npos);
}

TEST(AppState, SubmitSearchEmptyIsNoop) {
    auto dict = std::make_shared<FakeDictionary>();
    AppState s(dict);
    typeInto(s, "");
    s.submitSearch();
    EXPECT_EQ(dict->calls(), 0);
    EXPECT_EQ(s.status(), "Ready.");
}

TEST(AppState, HistoryAppendsAfterHit) {
    auto dict = std::make_shared<FakeDictionary>();
    dict->setHit("apple", {"apple", "", {"fruit"}});
    auto hist = openMemHistory();
    AppState s(dict, hist);

    typeInto(s, "apple");
    s.submitSearch();
    ASSERT_EQ(s.recent().size(), 1u);
    EXPECT_EQ(s.recent().front().headword, "apple");
}

TEST(AppState, HistoryNotAppendedOnMiss) {
    auto dict = std::make_shared<FakeDictionary>();
    auto hist = openMemHistory();
    AppState s(dict, hist);
    typeInto(s, "nosuch");
    s.submitSearch();
    EXPECT_TRUE(s.recent().empty());
}

TEST(AppState, ToggleFavoriteFlipsFlagAndUpdatesList) {
    auto dict = std::make_shared<FakeDictionary>();
    dict->setHit("apple", {"apple", "", {"fruit"}});
    auto fav = openMemFavorites();
    AppState s(dict, nullptr, fav);

    typeInto(s, "apple");
    s.submitSearch();
    EXPECT_FALSE(s.currentIsFavorite());
    EXPECT_TRUE(s.favorites().empty());

    s.toggleFavorite();
    EXPECT_TRUE(s.currentIsFavorite());
    ASSERT_EQ(s.favorites().size(), 1u);
    EXPECT_EQ(s.favorites().front().headword, "apple");

    s.toggleFavorite();
    EXPECT_FALSE(s.currentIsFavorite());
    EXPECT_TRUE(s.favorites().empty());
}

TEST(AppState, ToggleFavoriteWithoutEntryIsNoop) {
    auto dict = std::make_shared<FakeDictionary>();
    auto fav = openMemFavorites();
    AppState s(dict, nullptr, fav);
    s.toggleFavorite();  // no current entry
    EXPECT_TRUE(s.favorites().empty());
}

TEST(AppState, ActivateRecentRerunsSearch) {
    auto dict = std::make_shared<FakeDictionary>();
    dict->setHit("apple", {"apple", "", {"fruit"}});
    auto hist = openMemHistory();
    AppState s(dict, hist);

    typeInto(s, "apple");
    s.submitSearch();
    ASSERT_EQ(s.recent().size(), 1u);
    const int before = dict->calls();

    s.activateRecent(0);
    EXPECT_EQ(dict->calls(), before + 1);
}

TEST(AppState, ActivateFavoriteRerunsSearch) {
    auto dict = std::make_shared<FakeDictionary>();
    dict->setHit("apple", {"apple", "", {"fruit"}});
    auto fav = openMemFavorites();
    ASSERT_TRUE(fav->add("apple").has_value());
    AppState s(dict, nullptr, fav);
    ASSERT_EQ(s.favorites().size(), 1u);

    s.activateFavorite(0);
    EXPECT_EQ(dict->calls(), 1);
    EXPECT_TRUE(s.currentEntry().has_value());
}

TEST(AppState, OutOfRangeIndexIsSafe) {
    auto dict = std::make_shared<FakeDictionary>();
    AppState s(dict);
    s.activateRecent(99);    // no crash
    s.activateFavorite(99);  // no crash
    EXPECT_EQ(dict->calls(), 0);
}

TEST(AppState, HasFavoritesReflectsConstructor) {
    auto dict = std::make_shared<FakeDictionary>();
    {
        AppState s(dict);
        EXPECT_FALSE(s.hasFavorites());
    }
    {
        auto fav = openMemFavorites();
        AppState s(dict, nullptr, fav);
        EXPECT_TRUE(s.hasFavorites());
    }
}
