#include <filesystem>
#include <memory>

#include <QApplication>
#include <QDir>
#include <QFileInfo>
#include <QMessageBox>
#include <QStandardPaths>

#include "core/dictionary/SqliteDictionary.hpp"
#include "core/favorites/FavoritesStore.hpp"
#include "core/history/HistoryStore.hpp"
#include "core/storage/Database.hpp"
#include "ui/MainWindow.hpp"

namespace {

std::filesystem::path locateDictionary() {
    const QString next_to_exe =
        QDir(QCoreApplication::applicationDirPath()).filePath(QStringLiteral("mini_dict.sqlite"));
    if (QFileInfo::exists(next_to_exe)) {
        return std::filesystem::path(next_to_exe.toStdString());
    }
#ifdef EASYENGLISH_FIXTURES_DIR
    const std::filesystem::path fixture =
        std::filesystem::path(EASYENGLISH_FIXTURES_DIR) / "mini_dict.sqlite";
    if (std::filesystem::exists(fixture)) {
        return fixture;
    }
#endif
    return {};
}

std::filesystem::path userDataPath(const QString& filename) {
    const QString dir = QStandardPaths::writableLocation(QStandardPaths::AppLocalDataLocation);
    if (dir.isEmpty()) {
        return {};
    }
    return std::filesystem::path(QDir(dir).filePath(filename).toStdString());
}

}  // namespace

int main(int argc, char* argv[]) {
    QApplication app(argc, argv);
    QApplication::setApplicationName("EasyEnglish");
    QApplication::setOrganizationName("EasyEnglish");

    using namespace easyenglish;

    auto dict_path = locateDictionary();
    std::shared_ptr<core::dictionary::IDictionary> dict;

    if (!dict_path.empty()) {
        auto db = core::storage::Database::open(dict_path);
        if (db.has_value()) {
            auto sqlite_dict = core::dictionary::SqliteDictionary::open(std::move(db.value()));
            if (sqlite_dict.has_value()) {
                dict = std::make_shared<core::dictionary::SqliteDictionary>(
                    std::move(sqlite_dict.value()));
            }
        }
    }

    if (!dict) {
        QMessageBox::warning(nullptr, QApplication::tr("EasyEnglish"),
                             QApplication::tr("No dictionary database found. "
                                              "The UI will start but lookups will fail."));
    }

    std::shared_ptr<core::history::HistoryStore> history;
    if (const auto path = userDataPath(QStringLiteral("history.sqlite")); !path.empty()) {
        auto db = core::storage::Database::createOrOpen(path);
        if (db.has_value()) {
            auto store = core::history::HistoryStore::open(std::move(db.value()));
            if (store.has_value()) {
                history = std::make_shared<core::history::HistoryStore>(std::move(store.value()));
            }
        }
    }

    std::shared_ptr<core::favorites::FavoritesStore> favorites;
    if (const auto path = userDataPath(QStringLiteral("favorites.sqlite")); !path.empty()) {
        auto db = core::storage::Database::createOrOpen(path);
        if (db.has_value()) {
            auto store = core::favorites::FavoritesStore::open(std::move(db.value()));
            if (store.has_value()) {
                favorites =
                    std::make_shared<core::favorites::FavoritesStore>(std::move(store.value()));
            }
        }
    }

    ui::MainWindow window(dict, history, favorites);
    window.show();

    return QApplication::exec();
}
