# `ui/MainView` Contract

**Source path**: `src/ui/MainView.{hpp,cpp}` + `src/app/AppState.{hpp,cpp}`
**Owner test path**: `tests/app/test_app_state.cpp`
**Status**: frozen (since iter-009 / ImGui rewrite — supersedes the old `ui-mainwindow.md` Qt contract)

## 1. Public API (FROZEN — change requires ADR)

```cpp
namespace easyenglish::app {

class AppState {
public:
    static constexpr std::size_t kInputBufferSize = 256;
    AppState(std::shared_ptr<core::dictionary::IDictionary> dict,
             std::shared_ptr<core::history::HistoryStore> history = nullptr,
             std::shared_ptr<core::favorites::FavoritesStore> favorites = nullptr);

    std::array<char, kInputBufferSize> input_buffer{};  // bound to ImGui::InputText

    const std::optional<core::dictionary::Entry>& currentEntry() const;
    const std::string& status() const;
    const std::vector<core::history::HistoryEntry>& recent() const;
    const std::vector<core::favorites::FavoriteEntry>& favorites() const;
    bool currentIsFavorite() const;
    bool hasFavorites() const;
    bool inputIsNonEmpty() const;

    void submitSearch();
    void submitSearchWord(std::string_view word);
    void toggleFavorite();
    void activateRecent(std::size_t index);
    void activateFavorite(std::size_t index);
};

}  // namespace

namespace easyenglish::ui {

class MainView {
public:
    static void render(app::AppState& state);  // call once per ImGui frame
};

}  // namespace
```

## 2. Invariants

- **AppState performs no rendering.** It mutates its own fields only; never
  touches GL, ImGui, GLFW. → unit-testable without any GPU / window.
- **MainView is stateless.** Every frame it reads `state` and calls
  `state.method()` in response to ImGui events. No frame-to-frame caching.
- Empty / whitespace-only input → `submitSearch()` is a no-op.
- A successful lookup updates: `current_entry_`, `status_`, `recent_`,
  `current_is_favorite_`. A miss clears `current_entry_` and writes the
  `Not found / Invalid / Storage error` message into `status_`.
- `toggleFavorite()` with no current entry or no favorites store → no-op.
- `activateRecent(i)` / `activateFavorite(i)` with out-of-range `i` → no-op.

## 3. Required widget ids in ImGui frame

The MainView always produces this widget tree (used by future snapshot tests):

- one top-level borderless window `EasyEnglishMain`
- search row: `##search` `InputText` + "Search" `Button` + "Star/Unstar" `Button`
- body splitter: `ResultPanel` child + `SidePanel` child with `##SideTabs` `TabBar`
  containing `History` and `Favorites` tabs
- bottom: status `TextWrapped`

## 4. Test surface

Required (all live in `tests/app/test_app_state.cpp`):

- Default state, input-non-empty trim semantics
- Hit / miss / empty search
- History appends on hit only
- Favorite toggle flips flag + list + idempotency
- Activate recent / favorite re-runs search
- Out-of-range index is safe
- `hasFavorites()` reflects ctor argument

## 5. Performance budget

- One `MainView::render()` call should not allocate per-frame for steady
  state (rendering ≤ kDefaultRecent + ≤ kDefaultListLimit items).

## 6. Change log

- 2026-06-04 — iter-003: initial Qt MainWindow version (now obsolete).
- 2026-06-04 — iter-009: full rewrite as `AppState` (pure model) + `MainView`
  (ImGui render fn). Qt removed entirely. See ADR-0002.

