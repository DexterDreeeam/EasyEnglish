#include <filesystem>
#include <memory>

#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QPushButton>
#include <QTextBrowser>
#include <QToolButton>
#include <QtTest/QtTest>

#include "core/dictionary/SqliteDictionary.hpp"
#include "core/favorites/FavoritesStore.hpp"
#include "core/history/HistoryStore.hpp"
#include "core/storage/Database.hpp"
#include "ui/MainWindow.hpp"

#ifndef EASYENGLISH_FIXTURES_DIR
#error "EASYENGLISH_FIXTURES_DIR must be defined by the build system"
#endif

namespace fs = std::filesystem;

namespace {

std::shared_ptr<easyenglish::core::dictionary::IDictionary> openRealDict() {
    auto db = easyenglish::core::storage::Database::open(fs::path(EASYENGLISH_FIXTURES_DIR) /
                                                         "mini_dict.sqlite");
    if (!db.has_value()) {
        return {};
    }
    auto dict = easyenglish::core::dictionary::SqliteDictionary::open(std::move(db.value()));
    if (!dict.has_value()) {
        return {};
    }
    return std::make_shared<easyenglish::core::dictionary::SqliteDictionary>(
        std::move(dict.value()));
}

std::shared_ptr<easyenglish::core::history::HistoryStore> openMemoryHistory() {
    auto db = easyenglish::core::storage::Database::open(
        std::string(easyenglish::core::storage::Database::kInMemory));
    if (!db.has_value()) {
        return {};
    }
    auto store = easyenglish::core::history::HistoryStore::open(std::move(db.value()));
    if (!store.has_value()) {
        return {};
    }
    return std::make_shared<easyenglish::core::history::HistoryStore>(std::move(store.value()));
}

std::shared_ptr<easyenglish::core::favorites::FavoritesStore> openMemoryFavorites() {
    auto db = easyenglish::core::storage::Database::open(
        std::string(easyenglish::core::storage::Database::kInMemory));
    if (!db.has_value()) {
        return {};
    }
    auto store = easyenglish::core::favorites::FavoritesStore::open(std::move(db.value()));
    if (!store.has_value()) {
        return {};
    }
    return std::make_shared<easyenglish::core::favorites::FavoritesStore>(std::move(store.value()));
}

}  // namespace

class MainWindowEndToEnd : public QObject {
    Q_OBJECT
private slots:
    /// Full vertical slice: real SqliteDictionary over the shipped fixture,
    /// real (in-memory) history + favorites stores. Drives the UI like a user:
    /// type → enter → star → check history/favorites/result populated.
    void searchAndFavoriteFlow() {
        auto dict = openRealDict();
        QVERIFY(dict != nullptr);
        auto hist = openMemoryHistory();
        QVERIFY(hist != nullptr);
        auto favs = openMemoryFavorites();
        QVERIFY(favs != nullptr);

        easyenglish::ui::MainWindow w(dict, hist, favs);
        w.show();
        QVERIFY(QTest::qWaitForWindowExposed(&w));

        auto* input = w.findChild<QLineEdit*>(QStringLiteral("searchInput"));
        auto* button = w.findChild<QPushButton*>(QStringLiteral("searchButton"));
        auto* result = w.findChild<QTextBrowser*>(QStringLiteral("resultView"));
        auto* status = w.findChild<QLabel*>(QStringLiteral("statusLabel"));
        auto* hist_list = w.findChild<QListWidget*>(QStringLiteral("historyList"));
        auto* fav_list = w.findChild<QListWidget*>(QStringLiteral("favoritesList"));
        auto* fav_btn = w.findChild<QToolButton*>(QStringLiteral("favoriteButton"));
        QVERIFY(input && button && result && status && hist_list && fav_list && fav_btn);

        // Type a known headword from the shipped fixture.
        QTest::keyClicks(input, QStringLiteral("apple"));
        QVERIFY(button->isEnabled());
        QTest::keyClick(input, Qt::Key_Return);

        // Result rendered with phonetic + at least one definition.
        const auto rendered = result->toPlainText();
        QVERIFY(rendered.contains(QStringLiteral("apple")));
        QVERIFY(rendered.contains(QStringLiteral("fruit")));
        QVERIFY(status->text().contains(QStringLiteral("Found")));

        // History list now contains the canonical headword.
        QCOMPARE(hist_list->count(), 1);
        QCOMPARE(hist_list->item(0)->text(), QStringLiteral("apple"));

        // Star the entry → favorites list grows.
        QCOMPARE(fav_btn->text(), QStringLiteral("☆"));
        QTest::mouseClick(fav_btn, Qt::LeftButton);
        QCOMPARE(fav_btn->text(), QStringLiteral("★"));
        QCOMPARE(fav_list->count(), 1);
        QCOMPARE(fav_list->item(0)->text(), QStringLiteral("apple"));

        // Search a typo and verify the local fuzzy suggestions surface (via the
        // dictionary's suggest() — the UI doesn't render them yet, so the
        // assertion is on the dictionary directly to keep this test tight).
        auto suggestions = dict->suggest("appl", 3);
        QVERIFY(!suggestions.empty());
        QCOMPARE(QString::fromStdString(suggestions.front()), QStringLiteral("apple"));
    }
};

QTEST_MAIN(MainWindowEndToEnd)
#include "test_e2e_main_flow.moc"
