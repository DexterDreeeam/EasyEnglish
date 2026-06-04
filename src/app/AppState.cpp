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

AppState::AppState(std::shared_ptr<core::dictionary::IDictionary> local,
                   std::shared_ptr<core::dictionary::IDictionary> online,
                   std::shared_ptr<core::history::HistoryStore> history,
                   std::shared_ptr<core::favorites::FavoritesStore> favorites)
    : local_(std::move(local)),
      online_(std::move(online)),
      history_(std::move(history)),
      favorites_(std::move(favorites)) {}

bool AppState::inputIsNonEmpty() const {
    for (char c : input_buffer) {
        if (c == '\0')
            return false;
        if (!std::isspace(static_cast<unsigned char>(c)))
            return true;
    }
    return false;
}

void AppState::reset() {
    std::memset(input_buffer.data(), 0, input_buffer.size());
    translations_.clear();
    current_headword_.clear();
    current_phonetic_.clear();
    status_.clear();
}

void AppState::setHeadword(std::string_view word) {
    current_headword_.assign(word);
    current_phonetic_.clear();
    translations_.clear();
}

bool AppState::tryDictionary(const std::shared_ptr<core::dictionary::IDictionary>& dict,
                             std::string_view word, const char* label) {
    if (!dict)
        return false;
    auto result = dict->lookup(word);
    if (!result) {
        // Bubble up only StorageError (unexpected) as a status update; NotFound
        // is the normal "this dict can't help, try the next one" path.
        if (result.error() == core::dictionary::DictError::StorageError) {
            status_ = std::string(label) + ": storage error";
        }
        return false;
    }
    setHeadword(result->headword);
    current_phonetic_ = result->phonetic;
    translations_ = std::move(result->definitions);
    if (translations_.size() > kMaxTranslations) {
        translations_.resize(kMaxTranslations);
    }
    return true;
}

void AppState::submitSearch() {
    const std::string word = trim(input_buffer.data());
    if (word.empty()) {
        return;
    }

    bool hit = tryDictionary(local_, word, "local");
    if (!hit) {
        hit = tryDictionary(online_, word, "online");
    }

    if (hit) {
        status_ = "Found";
        if (history_) {
            (void)history_->record(std::string_view(current_headword_));
        }
        return;
    }

    setHeadword(word);
    if (status_.empty()) {
        status_ = "Not found: " + word;
    }
}

}  // namespace easyenglish::app
