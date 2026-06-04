#include <filesystem>
#include <memory>

#include <QApplication>
#include <QDir>
#include <QFileInfo>
#include <QMessageBox>
#include <QStandardPaths>

#include "core/dictionary/SqliteDictionary.hpp"
#include "core/history/HistoryStore.hpp"
#include "core/storage/Database.hpp"
#include "ui/MainWindow.hpp"

namespace {

/// Locate the dictionary file shipped alongside the executable. For dev/CI
/// runs we fall back to the in-tree fixture so the .exe is always usable
/// without an installer step.
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

std::filesystem::path userHistoryPath() {
    const QString dir = QStandardPaths::writableLocation(QStandardPaths::AppLocalDataLocation);
    if (dir.isEmpty()) {
        return {};
    }
    return std::filesystem::path(
        QDir(dir).filePath(QStringLiteral("history.sqlite")).toStdString());
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
    if (const auto history_path = userHistoryPath(); !history_path.empty()) {
        auto db = core::storage::Database::createOrOpen(history_path);
        if (db.has_value()) {
            auto store = core::history::HistoryStore::open(std::move(db.value()));
            if (store.has_value()) {
                history = std::make_shared<core::history::HistoryStore>(std::move(store.value()));
            }
        }
    }

    ui::MainWindow window(dict, history);
    window.show();

    return QApplication::exec();
}
