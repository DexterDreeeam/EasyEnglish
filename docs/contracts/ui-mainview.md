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
    static constexpr std::size_t kMaxTranslations = 8;

    AppState(std::shared_ptr<core::dictionary::IDictionary> local,
             std::shared_ptr<core::dictionary::IDictionary> online = nullptr,
             std::shared_ptr<core::history::HistoryStore> history = nullptr,
             std::shared_ptr<core::favorites::FavoritesStore> favorites = nullptr);

    std::array<char, kInputBufferSize> input_buffer{};

    const std::string& currentHeadword() const;
    const std::string& currentPhonetic() const;
    const std::vector<std::string>& currentTranslations() const;
    const std::string& status() const;
    bool hasResults() const;
    bool inputIsNonEmpty() const;

    void submitSearch();   // local first, online fallback, record to history
    void reset();          // clear buffer + results (called on overlay dismiss)
};

}  // namespace

namespace easyenglish::ui {

class MainView {
public:
    [[nodiscard]] static bool render(app::AppState& state, bool just_shown);
};

}  // namespace
```

## 2. Invariants

- **AppState performs no rendering or platform I/O.** It mutates its own fields
  only — fully testable without a window, OpenGL context, or HTTP server.
- `submitSearch()` consults `local_` first; on miss / no result it falls back
  to `online_` if configured. A successful hit is recorded to `history_`.
- `submitSearch()` with an empty / whitespace-only buffer is a no-op
  (no status change, no dict call).
- `currentTranslations()` is capped at `kMaxTranslations`.
- `reset()` zeroes the input buffer and clears all result fields. Status
  goes back to empty.
- StorageError from a dictionary is surfaced via `status()`; NotFound /
  InvalidInput are silent.

## 3. Required widget tree

- one borderless ImGui window `##overlay`
- one `##search` `InputText` with `EnterReturnsTrue | AutoSelectAll`
- if `currentHeadword()` non-empty: `Text` headword + disabled `Text` phonetic
- one `Selectable` per item in `currentTranslations()` (Push/PopID with index)
- if no translations but a status exists: a disabled `Text` status line

## 4. Test surface

Required (all live in `tests/app/test_app_state.cpp`):

- Default state, input-non-empty trim semantics
- Local hit / local miss → online hit / both miss → "Not found"
- Online not consulted when local hits (preferred ordering)
- Empty input is no-op
- `reset()` clears everything
- History append on hit only
- `kMaxTranslations` cap
- StorageError surfaces via `status()`

## 5. Performance budget

- One `MainView::render()` call should not allocate per-frame for steady
  state (rendering ≤ kDefaultRecent + ≤ kDefaultListLimit items).

## 6. Change log

- 2026-06-04 — iter-003: initial Qt MainWindow version (obsolete).
- 2026-06-04 — iter-009: full rewrite as `AppState` (pure model) + `MainView`
  (ImGui render fn). Qt removed entirely. See ADR-0002.
- 2026-06-04 — iter-011: **product surface reshaped as a frameless overlay
  translator.** AppState dropped history/favorites side panels and the star
  toggle; gained `currentTranslations()` / `currentPhonetic()` / `reset()`,
  ctor reordered to `(local, online, history, favorites)`. MainView is now a
  single-input overlay: input box + dropdown of Chinese translations + Esc
  dismissal. Window is hidden by default; shown only on Ctrl+Shift+WheelUp
  (see `docs/contracts/platform.md`). See ADR-0003.

