#include "ui/MainWindow.hpp"

#include <utility>

#include <QHBoxLayout>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QPushButton>
#include <QSplitter>
#include <QTextBrowser>
#include <QVBoxLayout>
#include <QWidget>

namespace easyenglish::ui {

namespace {

QString renderEntryHtml(const core::dictionary::Entry& e) {
    QString html;
    html += QStringLiteral("<h2>%1</h2>").arg(QString::fromStdString(e.headword).toHtmlEscaped());
    if (!e.phonetic.empty()) {
        html += QStringLiteral("<p><i>%1</i></p>")
                    .arg(QString::fromStdString(e.phonetic).toHtmlEscaped());
    }
    if (!e.definitions.empty()) {
        html += QStringLiteral("<ol>");
        for (const auto& def : e.definitions) {
            html += QStringLiteral("<li>%1</li>").arg(QString::fromStdString(def).toHtmlEscaped());
        }
        html += QStringLiteral("</ol>");
    }
    return html;
}

}  // namespace

MainWindow::MainWindow(std::shared_ptr<core::dictionary::IDictionary> dict,
                       std::shared_ptr<core::history::HistoryStore> history, QWidget* parent)
    : QMainWindow(parent), dict_(std::move(dict)), history_(std::move(history)) {
    setWindowTitle(tr("EasyEnglish"));
    resize(900, 600);

    auto* central = new QWidget(this);
    auto* root = new QVBoxLayout(central);

    auto* search_row = new QHBoxLayout();
    input_ = new QLineEdit(central);
    input_->setObjectName(QStringLiteral("searchInput"));
    input_->setPlaceholderText(tr("Type a word and press Enter…"));
    search_button_ = new QPushButton(tr("Search"), central);
    search_button_->setObjectName(QStringLiteral("searchButton"));
    search_button_->setEnabled(false);
    search_row->addWidget(input_, 1);
    search_row->addWidget(search_button_, 0);

    auto* body = new QSplitter(Qt::Horizontal, central);
    body->setObjectName(QStringLiteral("bodySplitter"));

    result_view_ = new QTextBrowser(body);
    result_view_->setObjectName(QStringLiteral("resultView"));
    result_view_->setReadOnly(true);

    history_list_ = new QListWidget(body);
    history_list_->setObjectName(QStringLiteral("historyList"));
    history_list_->setMaximumWidth(220);

    body->addWidget(result_view_);
    body->addWidget(history_list_);
    body->setStretchFactor(0, 1);
    body->setStretchFactor(1, 0);

    status_label_ = new QLabel(tr("Ready."), central);
    status_label_->setObjectName(QStringLiteral("statusLabel"));

    root->addLayout(search_row);
    root->addWidget(body, 1);
    root->addWidget(status_label_, 0);

    setCentralWidget(central);

    connect(input_, &QLineEdit::returnPressed, this, &MainWindow::onSearch);
    connect(search_button_, &QPushButton::clicked, this, &MainWindow::onSearch);
    connect(input_, &QLineEdit::textChanged, this, &MainWindow::onInputChanged);
    connect(history_list_, &QListWidget::itemActivated, this, &MainWindow::onHistoryItemActivated);

    refreshHistoryView();
}

void MainWindow::onInputChanged(const QString& text) {
    search_button_->setEnabled(!text.trimmed().isEmpty());
}

void MainWindow::onSearch() {
    const auto word = input_->text().trimmed();
    if (word.isEmpty()) {
        return;
    }
    emit searchRequested(word);

    if (!dict_) {
        status_label_->setText(tr("No dictionary configured."));
        result_view_->clear();
        return;
    }

    const auto utf8 = word.toUtf8();
    const auto result =
        dict_->lookup(std::string_view(utf8.constData(), static_cast<std::size_t>(utf8.size())));
    if (result.has_value()) {
        if (history_) {
            // Best-effort: storage errors here should not block the UI.
            (void)history_->record(std::string_view(result.value().headword));
            refreshHistoryView();
        }
        emit resultReady(QString::fromStdString(result.value().headword));
        result_view_->setHtml(renderEntryHtml(result.value()));
        status_label_->setText(tr("Found."));
    } else {
        using core::dictionary::DictError;
        result_view_->clear();
        switch (result.error()) {
            case DictError::NotFound:
                status_label_->setText(tr("Not found: %1").arg(word));
                break;
            case DictError::InvalidInput:
                status_label_->setText(tr("Invalid input."));
                break;
            case DictError::StorageError:
                status_label_->setText(tr("Storage error — please retry."));
                break;
        }
    }
}

void MainWindow::onHistoryItemActivated(QListWidgetItem* item) {
    if (item == nullptr) {
        return;
    }
    input_->setText(item->text());
    onSearch();
}

void MainWindow::refreshHistoryView() {
    if (history_ == nullptr || history_list_ == nullptr) {
        return;
    }
    auto recent = history_->recent();
    if (!recent.has_value()) {
        return;
    }
    history_list_->clear();
    for (const auto& entry : recent.value()) {
        history_list_->addItem(QString::fromStdString(entry.headword));
    }
}

}  // namespace easyenglish::ui
