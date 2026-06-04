#include <QApplication>

#include "ui/MainWindow.hpp"

int main(int argc, char* argv[]) {
    QApplication app(argc, argv);
    QApplication::setApplicationName("EasyEnglish");
    QApplication::setOrganizationName("EasyEnglish");

    easyenglish::ui::MainWindow window;
    window.show();

    return QApplication::exec();
}
