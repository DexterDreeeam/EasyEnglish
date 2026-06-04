#include "ui/MainWindow.hpp"

#include <QLabel>

namespace easyenglish::ui {

MainWindow::MainWindow(QWidget* parent) : QMainWindow(parent) {
    setWindowTitle(tr("EasyEnglish"));
    resize(800, 600);

    auto* placeholder = new QLabel(
        tr("EasyEnglish — scaffold ready. iter-003 will fill this in."), this);
    placeholder->setAlignment(Qt::AlignCenter);
    setCentralWidget(placeholder);
}

}  // namespace easyenglish::ui
