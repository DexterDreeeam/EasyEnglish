#include <memory>
#include <string>

#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QPushButton>
#include <QTextBrowser>
#include <QToolButton>
#include <QtTest/QtTest>

#include "core/dictionary/Entry.hpp"
#include "core/dictionary/IDictionary.hpp"
#include "core/favorites/FavoritesStore.hpp"
#include "core/history/HistoryStore.hpp"
#include "core/storage/Database.hpp"
#include "ui/MainWindow.hpp"

namespace {

/// Hand-written test double — avoids GMock surface area for a UI test that
/// only needs three deterministic responses. Lives in the test TU only.
class FakeDictionary final : public easyenglish::core::dictionary::IDictionary {
public:
    auto lookup(std::string_view word) const
        -> std::expected<easyenglish::core::dictionary::Entry,
                         easyenglish::core::dictionary::DictError> override {
        ++calls_;
        if (word.empty()) {
            return std::unexpected(easyenglish::core::dictionary::DictError::InvalidInput);
        }
        if (word == hit_word_) {
            return canned_entry_;
        }
        return std::unexpected(easyenglish::core::dictionary::DictError::NotFound);
    }

    auto suggest(std::string_view /*prefix*/, std::size_t /*max*/) const
        -> std::vector<std::string> override {
        return {};
    }

    void setHit(std::string word, easyenglish::core::dictionary::Entry entry) {
        hit_word_ = std::move(word);
        canned_entry_ = std::move(entry);
    }

    [[nodiscard]] int calls() const { return calls_; }

private:
    mutable int calls_{0};
    std::string hit_word_;
    easyenglish::core::dictionary::Entry canned_entry_;
};

QLineEdit* searchInput(easyenglish::ui::MainWindow& w) {
    return w.findChild<QLineEdit*>(QStringLiteral("searchInput"));
}
QPushButton* searchButton(easyenglish::ui::MainWindow& w) {
    return w.findChild<QPushButton*>(QStringLiteral("searchButton"));
}
QTextBrowser* resultView(easyenglish::ui::MainWindow& w) {
    return w.findChild<QTextBrowser*>(QStringLiteral("resultView"));
}
QLabel* statusLabel(easyenglish::ui::MainWindow& w) {
    return w.findChild<QLabel*>(QStringLiteral("statusLabel"));
}
QListWidget* historyList(easyenglish::ui::MainWindow& w) {
    return w.findChild<QListWidget*>(QStringLiteral("historyList"));
}
QListWidget* favoritesList(easyenglish::ui::MainWindow& w) {
    return w.findChild<QListWidget*>(QStringLiteral("favoritesList"));
}
QToolButton* favoriteButton(easyenglish::ui::MainWindow& w) {
    return w.findChild<QToolButton*>(QStringLiteral("favoriteButton"));
}

std::shared_ptr<easyenglish::core::history::HistoryStore> makeEmptyHistory() {
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

std::shared_ptr<easyenglish::core::favorites::FavoritesStore> makeEmptyFavorites() {
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

class MainWindowSearchTest : public QObject {
    Q_OBJECT
private slots:
    void searchButtonDisabledOnEmptyInput() {
        auto fake = std::make_shared<FakeDictionary>();
        easyenglish::ui::MainWindow w(fake);
        auto* button = searchButton(w);
        QVERIFY(button != nullptr);
        QVERIFY(!button->isEnabled());

        auto* input = searchInput(w);
        QVERIFY(input != nullptr);
        QTest::keyClicks(input, QStringLiteral("apple"));
        QVERIFY(button->isEnabled());

        input->clear();
        QVERIFY(!button->isEnabled());
    }

    void emitsSearchRequestedOnEnter() {
        auto fake = std::make_shared<FakeDictionary>();
        easyenglish::ui::MainWindow w(fake);
        QSignalSpy spy(&w, &easyenglish::ui::MainWindow::searchRequested);

        auto* input = searchInput(w);
        QVERIFY(input != nullptr);
        input->setText(QStringLiteral("apple"));
        QTest::keyClick(input, Qt::Key_Return);

        QCOMPARE(spy.count(), 1);
        QCOMPARE(spy.at(0).at(0).toString(), QStringLiteral("apple"));
        QCOMPARE(fake->calls(), 1);
    }

    void displaysEntryOnSuccessfulLookup() {
        auto fake = std::make_shared<FakeDictionary>();
        easyenglish::core::dictionary::Entry entry{"apple", "/ˈæp.əl/", {"a round fruit"}};
        fake->setHit("apple", entry);

        easyenglish::ui::MainWindow w(fake);
        QSignalSpy spy(&w, &easyenglish::ui::MainWindow::resultReady);

        auto* input = searchInput(w);
        input->setText(QStringLiteral("apple"));
        QTest::keyClick(input, Qt::Key_Return);

        QCOMPARE(spy.count(), 1);
        auto* view = resultView(w);
        QVERIFY(view != nullptr);
        const auto text = view->toPlainText();
        QVERIFY2(text.contains(QStringLiteral("apple")),
                 qPrintable(QStringLiteral("expected 'apple' in: %1").arg(text)));
        QVERIFY(text.contains(QStringLiteral("a round fruit")));

        auto* status = statusLabel(w);
        QVERIFY(status != nullptr);
        QVERIFY(status->text().contains(QStringLiteral("Found")));
    }

    void showsNotFoundOnMissingWord() {
        auto fake = std::make_shared<FakeDictionary>();
        easyenglish::ui::MainWindow w(fake);

        auto* input = searchInput(w);
        input->setText(QStringLiteral("nosuch"));
        QTest::keyClick(input, Qt::Key_Return);

        auto* status = statusLabel(w);
        QVERIFY(status->text().contains(QStringLiteral("Not found")));

        auto* view = resultView(w);
        QVERIFY(view->toPlainText().isEmpty());
    }

    void searchButtonClickAlsoTriggersLookup() {
        auto fake = std::make_shared<FakeDictionary>();
        easyenglish::ui::MainWindow w(fake);
        QSignalSpy spy(&w, &easyenglish::ui::MainWindow::searchRequested);

        searchInput(w)->setText(QStringLiteral("apple"));
        QTest::mouseClick(searchButton(w), Qt::LeftButton);

        QCOMPARE(spy.count(), 1);
    }

    void historyListUpdatesAfterSuccessfulLookup() {
        auto fake = std::make_shared<FakeDictionary>();
        fake->setHit("apple", {"apple", "/ˈæp.əl/", {"fruit"}});
        auto history = makeEmptyHistory();
        QVERIFY(history != nullptr);

        easyenglish::ui::MainWindow w(fake, history);
        auto* list = historyList(w);
        QVERIFY(list != nullptr);
        QCOMPARE(list->count(), 0);

        searchInput(w)->setText(QStringLiteral("apple"));
        QTest::keyClick(searchInput(w), Qt::Key_Return);

        QCOMPARE(list->count(), 1);
        QCOMPARE(list->item(0)->text(), QStringLiteral("apple"));
    }

    void historyListIgnoresMissingWords() {
        auto fake = std::make_shared<FakeDictionary>();
        auto history = makeEmptyHistory();
        QVERIFY(history != nullptr);

        easyenglish::ui::MainWindow w(fake, history);
        auto* list = historyList(w);
        QVERIFY(list != nullptr);

        searchInput(w)->setText(QStringLiteral("nosuch"));
        QTest::keyClick(searchInput(w), Qt::Key_Return);

        QCOMPARE(list->count(), 0);
    }

    void activatingHistoryItemRerunsSearch() {
        auto fake = std::make_shared<FakeDictionary>();
        fake->setHit("apple", {"apple", "/ˈæp.əl/", {"fruit"}});
        auto history = makeEmptyHistory();
        QVERIFY(history != nullptr);

        easyenglish::ui::MainWindow w(fake, history);
        searchInput(w)->setText(QStringLiteral("apple"));
        QTest::keyClick(searchInput(w), Qt::Key_Return);
        QCOMPARE(fake->calls(), 1);

        // Activate the just-recorded history entry → should trigger another lookup.
        // Use setCurrentRow + Key_Return instead of emitting itemActivated directly:
        // Qt 6 signals can technically be called from outside, but the test is
        // clearer when it drives the widget the same way a user would.
        auto* list = historyList(w);
        QVERIFY(list != nullptr);
        QCOMPARE(list->count(), 1);
        list->setCurrentRow(0);
        QTest::keyClick(list, Qt::Key_Return);
        QCOMPARE(fake->calls(), 2);
    }

    void favoriteButtonDisabledWhenNoEntry() {
        auto fake = std::make_shared<FakeDictionary>();
        auto fav = makeEmptyFavorites();
        QVERIFY(fav != nullptr);
        easyenglish::ui::MainWindow w(fake, nullptr, fav);
        auto* btn = favoriteButton(w);
        QVERIFY(btn != nullptr);
        QVERIFY(!btn->isEnabled());
        QCOMPARE(btn->text(), QStringLiteral("☆"));
    }

    void favoriteButtonTogglesAndUpdatesList() {
        auto fake = std::make_shared<FakeDictionary>();
        fake->setHit("apple", {"apple", "/ˈæp.əl/", {"fruit"}});
        auto fav = makeEmptyFavorites();
        QVERIFY(fav != nullptr);

        easyenglish::ui::MainWindow w(fake, nullptr, fav);
        QSignalSpy spy(&w, &easyenglish::ui::MainWindow::favoriteToggled);

        searchInput(w)->setText(QStringLiteral("apple"));
        QTest::keyClick(searchInput(w), Qt::Key_Return);

        auto* btn = favoriteButton(w);
        QVERIFY(btn != nullptr);
        QVERIFY(btn->isEnabled());
        QCOMPARE(btn->text(), QStringLiteral("☆"));

        QTest::mouseClick(btn, Qt::LeftButton);
        QCOMPARE(btn->text(), QStringLiteral("★"));
        auto* fav_list = favoritesList(w);
        QVERIFY(fav_list != nullptr);
        QCOMPARE(fav_list->count(), 1);
        QCOMPARE(fav_list->item(0)->text(), QStringLiteral("apple"));

        // Toggle back off.
        QTest::mouseClick(btn, Qt::LeftButton);
        QCOMPARE(btn->text(), QStringLiteral("☆"));
        QCOMPARE(fav_list->count(), 0);

        // The signal should fire twice with alternating bool argument.
        QCOMPARE(spy.count(), 2);
        QCOMPARE(spy.at(0).at(1).toBool(), true);
        QCOMPARE(spy.at(1).at(1).toBool(), false);
    }

    void activatingFavoritesItemRerunsSearch() {
        auto fake = std::make_shared<FakeDictionary>();
        fake->setHit("apple", {"apple", "/ˈæp.əl/", {"fruit"}});
        auto fav = makeEmptyFavorites();
        QVERIFY(fav != nullptr);
        QVERIFY(fav->add("apple").has_value());

        easyenglish::ui::MainWindow w(fake, nullptr, fav);
        auto* fav_list = favoritesList(w);
        QVERIFY(fav_list != nullptr);
        QCOMPARE(fav_list->count(), 1);

        fav_list->setCurrentRow(0);
        QTest::keyClick(fav_list, Qt::Key_Return);
        QCOMPARE(fake->calls(), 1);
    }
};

QTEST_MAIN(MainWindowSearchTest)
#include "test_mainwindow_search.moc"
