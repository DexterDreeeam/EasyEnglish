#pragma once

#include <memory>

#include <QMainWindow>

#include "core/dictionary/Entry.hpp"
#include "core/dictionary/IDictionary.hpp"
#include "core/favorites/FavoritesStore.hpp"
#include "core/history/HistoryStore.hpp"

class QLineEdit;
class QPushButton;
class QTextBrowser;
class QLabel;
class QListWidget;
class QListWidgetItem;
class QToolButton;
class QTabWidget;

namespace easyenglish::ui {

/// Search-and-display main window. Dependencies are injected — the window
/// itself never constructs storage or dictionary objects, so tests can drive
/// it with a fake `IDictionary` and verify behavior with `QSignalSpy`.
class MainWindow : public QMainWindow {
    Q_OBJECT
public:
    /// All non-dict dependencies are optional. Passing nullptr disables the
    /// corresponding UI affordance (e.g. history list, favorites star).
    explicit MainWindow(std::shared_ptr<core::dictionary::IDictionary> dict,
                        std::shared_ptr<core::history::HistoryStore> history = nullptr,
                        std::shared_ptr<core::favorites::FavoritesStore> favorites = nullptr,
                        QWidget* parent = nullptr);
    ~MainWindow() override = default;

signals:
    /// Emitted when the user issues a search (Enter or button click).
    void searchRequested(const QString& word);

    /// Emitted after a successful lookup. Carries only the canonical headword
    /// (Qt-native type) so recipients don't need Q_DECLARE_METATYPE.
    void resultReady(const QString& headword);

    /// Emitted whenever the favorite state of the currently displayed entry
    /// changes (toggled on or off).
    void favoriteToggled(const QString& headword, bool isFavorite);

private slots:
    void onSearch();
    void onInputChanged(const QString& text);
    void onHistoryItemActivated(QListWidgetItem* item);
    void onFavoritesItemActivated(QListWidgetItem* item);
    void onFavoriteButtonClicked();

private:
    void refreshHistoryView();
    void refreshFavoritesView();
    void refreshFavoriteButton();

    std::shared_ptr<core::dictionary::IDictionary> dict_;
    std::shared_ptr<core::history::HistoryStore> history_;
    std::shared_ptr<core::favorites::FavoritesStore> favorites_;
    QLineEdit* input_{nullptr};
    QPushButton* search_button_{nullptr};
    QToolButton* favorite_button_{nullptr};
    QTextBrowser* result_view_{nullptr};
    QLabel* status_label_{nullptr};
    QListWidget* history_list_{nullptr};
    QListWidget* favorites_list_{nullptr};
    QTabWidget* side_tabs_{nullptr};
    QString current_headword_;  // canonical casing of the entry on screen
};

}  // namespace easyenglish::ui
