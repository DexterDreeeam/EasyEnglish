#pragma once

#include <QMainWindow>

namespace easyenglish::ui {

class MainWindow : public QMainWindow {
    Q_OBJECT
public:
    explicit MainWindow(QWidget* parent = nullptr);
    ~MainWindow() override = default;
};

}  // namespace easyenglish::ui
