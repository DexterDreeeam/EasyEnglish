#include "ui/MainWindow.hpp"

#include <utility>

#include <QHBoxLayout>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QPushButton>
#include <QSplitter>
#include <QTabWidget>
#include <QTextBrowser>
#include <QToolButton>
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
                       std::shared_ptr<core::history::HistoryStore> history,
                       std::shared_ptr<core::favorites::FavoritesStore> favorites, QWidget* parent)
    : QMainWindow(parent),
      dict_(std::move(dict)),
      history_(std::move(history)),
      favorites_(std::move(favorites)) {
    setWindowTitle(tr("EasyEnglish"));
    resize(960, 640);

    auto* central = new QWidget(this);
    auto* root = new QVBoxLayout(central);

    auto* search_row = new QHBoxLayout();
    input_ = new QLineEdit(central);
    input_->setObjectName(QStringLiteral("searchInput"));
    input_->setPlaceholderText(tr("Type a word and press Enter…"));
    search_button_ = new QPushButton(tr("Search"), central);
    search_button_->setObjectName(QStringLiteral("searchButton"));
    search_button_->setEnabled(false);
    favorite_button_ = new QToolButton(central);
    favorite_button_->setObjectName(QStringLiteral("favoriteButton"));
    favorite_button_->setText(QStringLiteral("☆"));
    favorite_button_->setToolTip(tr("Add the current entry to favorites"));
    favorite_button_->setEnabled(false);
    search_row->addWidget(input_, 1);
    search_row->addWidget(search_button_, 0);
    search_row->addWidget(favorite_button_, 0);

    auto* body = new QSplitter(Qt::Horizontal, central);
    body->setObjectName(QStringLiteral("bodySplitter"));

    result_view_ = new QTextBrowser(body);
    result_view_->setObjectName(QStringLiteral("resultView"));
    result_view_->setReadOnly(true);

    side_tabs_ = new QTabWidget(body);
    side_tabs_->setObjectName(QStringLiteral("sideTabs"));
    side_tabs_->setMaximumWidth(260);

    history_list_ = new QListWidget(side_tabs_);
    history_list_->setObjectName(QStringLiteral("historyList"));
    side_tabs_->addTab(history_list_, tr("History"));

    favorites_list_ = new QListWidget(side_tabs_);
    favorites_list_->setObjectName(QStringLiteral("favoritesList"));
    side_tabs_->addTab(favorites_list_, tr("Favorites"));

    body->addWidget(result_view_);
    body->addWidget(side_tabs_);
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
    connect(favorites_list_, &QListWidget::itemActivated, this,
            &MainWindow::onFavoritesItemActivated);
    connect(favorite_button_, &QToolButton::clicked, this, &MainWindow::onFavoriteButtonClicked);

    refreshHistoryView();
    refreshFavoritesView();
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
        current_headword_ = QString::fromStdString(result.value().headword);
        if (history_) {
            (void)history_->record(std::string_view(result.value().headword));
            refreshHistoryView();
        }
        emit resultReady(current_headword_);
        result_view_->setHtml(renderEntryHtml(result.value()));
        status_label_->setText(tr("Found."));
        refreshFavoriteButton();
    } else {
        using core::dictionary::DictError;
        current_headword_.clear();
        result_view_->clear();
        refreshFavoriteButton();
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

void MainWindow::onFavoritesItemActivated(QListWidgetItem* item) {
    if (item == nullptr) {
        return;
    }
    input_->setText(item->text());
    onSearch();
}

void MainWindow::onFavoriteButtonClicked() {
    if (favorites_ == nullptr || current_headword_.isEmpty()) {
        return;
    }
    const auto utf8 = current_headword_.toUtf8();
    const std::string_view word(utf8.constData(), static_cast<std::size_t>(utf8.size()));

    auto contains = favorites_->contains(word);
    if (!contains.has_value()) {
        return;
    }
    bool now_favorite = false;
    if (contains.value()) {
        (void)favorites_->remove(word);
        now_favorite = false;
    } else {
        (void)favorites_->add(word);
        now_favorite = true;
    }
    refreshFavoritesView();
    refreshFavoriteButton();
    emit favoriteToggled(current_headword_, now_favorite);
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

void MainWindow::refreshFavoritesView() {
    if (favorites_ == nullptr || favorites_list_ == nullptr) {
        return;
    }
    auto list = favorites_->list();
    if (!list.has_value()) {
        return;
    }
    favorites_list_->clear();
    for (const auto& entry : list.value()) {
        favorites_list_->addItem(QString::fromStdString(entry.headword));
    }
}

void MainWindow::refreshFavoriteButton() {
    if (favorite_button_ == nullptr) {
        return;
    }
    if (favorites_ == nullptr || current_headword_.isEmpty()) {
        favorite_button_->setEnabled(false);
        favorite_button_->setText(QStringLiteral("☆"));
        return;
    }
    favorite_button_->setEnabled(true);
    const auto utf8 = current_headword_.toUtf8();
    auto contains = favorites_->contains(
        std::string_view(utf8.constData(), static_cast<std::size_t>(utf8.size())));
    const bool is_fav = contains.has_value() && contains.value();
    favorite_button_->setText(is_fav ? QStringLiteral("★") : QStringLiteral("☆"));
    favorite_button_->setToolTip(is_fav ? tr("Remove from favorites")
                                        : tr("Add the current entry to favorites"));
}

}  // namespace easyenglish::ui
