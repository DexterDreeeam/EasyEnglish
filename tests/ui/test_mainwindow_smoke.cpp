#include <memory>

#include <QtTest/QtTest>

#include "ui/MainWindow.hpp"

class MainWindowSmoke : public QObject {
    Q_OBJECT
private slots:
    void constructsAndShows() {
        easyenglish::ui::MainWindow window(nullptr, nullptr);
        window.show();
        QVERIFY(QTest::qWaitForWindowExposed(&window));
        QCOMPARE(window.windowTitle(), QStringLiteral("EasyEnglish"));
    }
};

QTEST_MAIN(MainWindowSmoke)
#include "test_mainwindow_smoke.moc"
