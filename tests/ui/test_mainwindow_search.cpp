#include <memory>
#include <string>

#include <QtTest/QtTest>

#include "core/dictionary/Entry.hpp"
#include "core/dictionary/IDictionary.hpp"
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
};

QTEST_MAIN(MainWindowSearchTest)
#include "test_mainwindow_search.moc"
