# `ui/MainWindow` Contract

**Source path**: `src/ui/MainWindow.{hpp,cpp}`
**Owner test path**: `tests/ui/test_mainwindow_*.cpp`
**Status**: draft (full freeze at end of iter-003)

## 1. Public API (FROZEN — change requires ADR)

```cpp
namespace easyenglish::ui {

class MainWindow : public QMainWindow {
    Q_OBJECT
public:
    // Dependencies are injected — MainWindow MUST NOT construct concrete
    // dictionary or storage objects itself. This is what makes UI tests
    // deterministic and decoupled from network/database.
    explicit MainWindow(
        std::shared_ptr<core::dictionary::IDictionary> dict,
        QWidget* parent = nullptr);

signals:
    // Emitted whenever the user requests a search (Enter, button, etc.).
    void searchRequested(const QString& word);

    // Emitted after a successful lookup, before the view is updated.
    void resultReady(const core::dictionary::Entry& entry);
};

}  // namespace easyenglish::ui
```

## 2. Invariants

- `MainWindow` performs **no** I/O directly. It only emits signals and calls
  into the injected `IDictionary`.
- Tests can verify behavior via `QSignalSpy` without a real database.
- All user-visible strings go through `tr()`.

## 3. Forbidden

- Direct `#include "core/storage/..."`
- `new SqliteDictionary(...)` or any concrete factory call inside `MainWindow`.
- `QMessageBox::critical` / blocking dialogs for errors that occur on the
  search hot path (use inline status instead — easier to test).

## 4. Test surface

Required tests (each lands in iter-003):

- `MainWindowSearchTest::emitsSignalOnEnter`
- `MainWindowSearchTest::displaysEntryOnSuccess`
- `MainWindowSearchTest::showsNotFoundOnDictError`
- Snapshot baselines under `tests/snapshots/mainwindow/`:
  - `empty.png`
  - `with_result.png`
  - `not_found.png`

## 5. Performance budget

- First paint after construction < 200ms on cold start.

## 6. Change log

- (pending) — initial draft.
