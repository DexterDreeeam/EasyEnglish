#pragma once

#include <array>
#include <memory>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

#include "core/dictionary/Entry.hpp"
#include "core/dictionary/IDictionary.hpp"
#include "core/favorites/FavoritesStore.hpp"
#include "core/history/HistoryStore.hpp"

namespace easyenglish::app {

/// Pure-C++ presentation model. The UI layer (ImGui `MainView`) reads its
/// public fields each frame and calls its action methods on user input.
/// AppState contains no rendering code — that's what makes it unit-testable
/// without a window or OpenGL context.
class AppState {
public:
    /// Width of the search input buffer (ImGui::InputText fills a char*).
    static constexpr std::size_t kInputBufferSize = 256;

    AppState(std::shared_ptr<core::dictionary::IDictionary> dict,
             std::shared_ptr<core::history::HistoryStore> history = nullptr,
             std::shared_ptr<core::favorites::FavoritesStore> favorites = nullptr);

    // ---- Mutable data the UI binds to each frame --------------------------
    std::array<char, kInputBufferSize> input_buffer{};

    // ---- Read-only state for the UI ---------------------------------------
    [[nodiscard]] const std::optional<core::dictionary::Entry>& currentEntry() const {
        return current_entry_;
    }
    [[nodiscard]] const std::string& status() const { return status_; }
    [[nodiscard]] const std::vector<core::history::HistoryEntry>& recent() const { return recent_; }
    [[nodiscard]] const std::vector<core::favorites::FavoriteEntry>& favorites() const {
        return favorites_;
    }
    [[nodiscard]] bool currentIsFavorite() const { return current_is_favorite_; }
    [[nodiscard]] bool hasFavorites() const { return favorites_store_ != nullptr; }
    [[nodiscard]] bool inputIsNonEmpty() const;

    // ---- Actions (called by MainView in response to UI events) ------------

    /// Read the current `input_buffer` (trimmed), call dictionary, update state.
    /// No-op if input is empty.
    void submitSearch();

    /// Convenience for tests / history-activation: load the word into the
    /// input buffer then run `submitSearch()`.
    void submitSearchWord(std::string_view word);

    /// Toggle favorite status of the currently-displayed entry. No-op if no
    /// favorites store is configured or no current entry.
    void toggleFavorite();

    void activateRecent(std::size_t index);
    void activateFavorite(std::size_t index);

private:
    void refreshHistory();
    void refreshFavorites();
    void refreshFavoriteFlag();
    void setInputBuffer(std::string_view word);

    std::shared_ptr<core::dictionary::IDictionary> dict_;
    std::shared_ptr<core::history::HistoryStore> history_;
    std::shared_ptr<core::favorites::FavoritesStore> favorites_store_;

    std::optional<core::dictionary::Entry> current_entry_;
    std::string status_{"Ready."};
    std::vector<core::history::HistoryEntry> recent_;
    std::vector<core::favorites::FavoriteEntry> favorites_;
    bool current_is_favorite_{false};
};

}  // namespace easyenglish::app
