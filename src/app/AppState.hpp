#pragma once

#include <array>
#include <memory>
#include <string>
#include <vector>

#include "core/dictionary/IDictionary.hpp"
#include "core/favorites/FavoritesStore.hpp"
#include "core/history/HistoryStore.hpp"

namespace easyenglish::app {

/// Pure-C++ presentation model for the frameless overlay translator.
///
/// MainView reads `current_translations()` + `status()` each frame and calls
/// the action methods on user input. AppState performs no rendering and no
/// platform calls — fully unit-testable.
///
/// As of iter-011 the app's product surface is "type English, see Chinese
/// translations". Local SQLite is consulted first; if it misses, the optional
/// online dictionary is queried. History/favorites stores stay around for
/// future tray-menu features but are not surfaced in the current UI.
class AppState {
public:
    static constexpr std::size_t kInputBufferSize = 256;
    static constexpr std::size_t kMaxTranslations = 8;

    AppState(std::shared_ptr<core::dictionary::IDictionary> local,
             std::shared_ptr<core::dictionary::IDictionary> online = nullptr,
             std::shared_ptr<core::history::HistoryStore> history = nullptr,
             std::shared_ptr<core::favorites::FavoritesStore> favorites = nullptr);

    // ---- Bound to ImGui::InputText each frame -----------------------------
    std::array<char, kInputBufferSize> input_buffer{};

    // ---- Read-only state the view binds to --------------------------------
    [[nodiscard]] const std::vector<std::string>& currentTranslations() const {
        return translations_;
    }
    [[nodiscard]] const std::string& currentHeadword() const { return current_headword_; }
    [[nodiscard]] const std::string& currentPhonetic() const { return current_phonetic_; }
    [[nodiscard]] const std::string& status() const { return status_; }
    [[nodiscard]] bool inputIsNonEmpty() const;
    [[nodiscard]] bool hasResults() const { return !translations_.empty(); }

    // ---- Actions ----------------------------------------------------------

    /// Trigger a lookup using the current input buffer. Local dictionary
    /// first; on miss/error, falls back to the online dictionary if one is
    /// configured. Records a successful hit to history.
    void submitSearch();

    /// Clear input + translation results back to the initial empty state.
    /// Called when the overlay is dismissed so the next show starts fresh.
    void reset();

private:
    void setHeadword(std::string_view word);
    bool tryDictionary(const std::shared_ptr<core::dictionary::IDictionary>& dict,
                       std::string_view word, const char* label);

    std::shared_ptr<core::dictionary::IDictionary> local_;
    std::shared_ptr<core::dictionary::IDictionary> online_;
    std::shared_ptr<core::history::HistoryStore> history_;
    std::shared_ptr<core::favorites::FavoritesStore> favorites_;

    std::string current_headword_;
    std::string current_phonetic_;
    std::vector<std::string> translations_;
    std::string status_;
};

}  // namespace easyenglish::app
