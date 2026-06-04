#pragma once

#include <memory>

#include <QMainWindow>

#include "core/dictionary/Entry.hpp"
#include "core/dictionary/IDictionary.hpp"

class QLineEdit;
class QPushButton;
class QTextBrowser;
class QLabel;

namespace easyenglish::ui {

/// Search-and-display main window. Dependencies are injected — the window
/// itself never constructs storage or dictionary objects, so tests can drive
/// it with a fake `IDictionary` and verify behavior with `QSignalSpy`.
class MainWindow : public QMainWindow {
    Q_OBJECT
public:
    explicit MainWindow(std::shared_ptr<core::dictionary::IDictionary> dict,
                        QWidget* parent = nullptr);
    ~MainWindow() override = default;

signals:
    /// Emitted when the user issues a search (Enter or button click).
    void searchRequested(const QString& word);

    /// Emitted after a successful lookup, before the view is updated.
    /// Carries only the canonical headword — listeners (e.g. history) can
    /// re-lookup if they need the full `Entry`. Keeping the signal Qt-native
    /// avoids forcing every recipient to register custom metatypes.
    void resultReady(const QString& headword);

private slots:
    void onSearch();
    void onInputChanged(const QString& text);

private:
    std::shared_ptr<core::dictionary::IDictionary> dict_;
    QLineEdit* input_{nullptr};
    QPushButton* search_button_{nullptr};
    QTextBrowser* result_view_{nullptr};
    QLabel* status_label_{nullptr};
};

}  // namespace easyenglish::ui
