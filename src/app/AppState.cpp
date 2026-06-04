#include "app/AppState.hpp"

#include <algorithm>
#include <cctype>
#include <cstring>
#include <utility>

namespace easyenglish::app {

namespace {

std::string trim(std::string_view s) {
    auto begin = s.begin();
    auto end = s.end();
    while (begin != end && std::isspace(static_cast<unsigned char>(*begin)))
        ++begin;
    while (end != begin && std::isspace(static_cast<unsigned char>(*(end - 1))))
        --end;
    return std::string(begin, end);
}

}  // namespace

AppState::AppState(std::shared_ptr<core::dictionary::IDictionary> dict,
                   std::shared_ptr<core::history::HistoryStore> history,
                   std::shared_ptr<core::favorites::FavoritesStore> favorites)
    : dict_(std::move(dict)), history_(std::move(history)), favorites_store_(std::move(favorites)) {
    refreshHistory();
    refreshFavorites();
}

bool AppState::inputIsNonEmpty() const {
    for (char c : input_buffer) {
        if (c == '\0')
            return false;
        if (!std::isspace(static_cast<unsigned char>(c)))
            return true;
    }
    return false;
}

void AppState::setInputBuffer(std::string_view word) {
    const auto n = std::min(word.size(), input_buffer.size() - 1);
    std::memcpy(input_buffer.data(), word.data(), n);
    input_buffer[n] = '\0';
    // Zero-fill the rest so debug tools / hex dumps don't show stale chars.
    std::memset(input_buffer.data() + n + 1, 0, input_buffer.size() - n - 1);
}

void AppState::submitSearch() {
    const std::string word = trim(input_buffer.data());
    if (word.empty()) {
        return;
    }

    if (!dict_) {
        current_entry_.reset();
        current_is_favorite_ = false;
        status_ = "No dictionary configured.";
        return;
    }

    auto result = dict_->lookup(word);
    if (result.has_value()) {
        current_entry_ = std::move(result.value());
        status_ = "Found.";
        if (history_) {
            (void)history_->record(std::string_view(current_entry_->headword));
            refreshHistory();
        }
        refreshFavoriteFlag();
    } else {
        current_entry_.reset();
        current_is_favorite_ = false;
        using core::dictionary::DictError;
        switch (result.error()) {
            case DictError::NotFound:
                status_ = "Not found: " + word;
                break;
            case DictError::InvalidInput:
                status_ = "Invalid input.";
                break;
            case DictError::StorageError:
                status_ = "Storage error — please retry.";
                break;
        }
    }
}

void AppState::submitSearchWord(std::string_view word) {
    setInputBuffer(word);
    submitSearch();
}

void AppState::toggleFavorite() {
    if (favorites_store_ == nullptr || !current_entry_.has_value()) {
        return;
    }
    const std::string_view word = current_entry_->headword;
    auto contains = favorites_store_->contains(word);
    if (!contains.has_value()) {
        return;
    }
    if (contains.value()) {
        (void)favorites_store_->remove(word);
    } else {
        (void)favorites_store_->add(word);
    }
    refreshFavorites();
    refreshFavoriteFlag();
}

void AppState::activateRecent(std::size_t index) {
    if (index >= recent_.size())
        return;
    submitSearchWord(recent_[index].headword);
}

void AppState::activateFavorite(std::size_t index) {
    if (index >= favorites_.size())
        return;
    submitSearchWord(favorites_[index].headword);
}

void AppState::refreshHistory() {
    recent_.clear();
    if (history_ == nullptr)
        return;
    auto r = history_->recent();
    if (r.has_value()) {
        recent_ = std::move(r.value());
    }
}

void AppState::refreshFavorites() {
    favorites_.clear();
    if (favorites_store_ == nullptr)
        return;
    auto l = favorites_store_->list();
    if (l.has_value()) {
        favorites_ = std::move(l.value());
    }
}

void AppState::refreshFavoriteFlag() {
    current_is_favorite_ = false;
    if (favorites_store_ == nullptr || !current_entry_.has_value())
        return;
    auto contains = favorites_store_->contains(current_entry_->headword);
    current_is_favorite_ = contains.has_value() && contains.value();
}

}  // namespace easyenglish::app
