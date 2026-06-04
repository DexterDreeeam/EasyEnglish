#include "core/dictionary/SqliteDictionary.hpp"

#include <utility>

#include <QJsonArray>
#include <QJsonDocument>
#include <QString>

namespace easyenglish::core::dictionary {

namespace {

constexpr const char* kLookupSql =
    "SELECT headword, phonetic, definitions FROM entries "
    "WHERE headword = ?1 COLLATE NOCASE LIMIT 1;";

std::vector<std::string> parseDefinitionsJson(const std::string& json) {
    // Definitions are persisted by tools/seed_db.py as a JSON array of strings.
    // QJsonDocument is already available via Qt6::Core, so no new dependency.
    std::vector<std::string> out;
    const auto doc = QJsonDocument::fromJson(QByteArray::fromStdString(json));
    if (!doc.isArray()) {
        return out;
    }
    const auto arr = doc.array();
    out.reserve(static_cast<std::size_t>(arr.size()));
    for (const auto& v : arr) {
        if (v.isString()) {
            out.emplace_back(v.toString().toStdString());
        }
    }
    return out;
}

DictError mapStorageError(storage::StorageError e) noexcept {
    switch (e) {
        case storage::StorageError::NotFound:
            return DictError::NotFound;
        case storage::StorageError::InvalidQuery:
        case storage::StorageError::ConstraintViolation:
        case storage::StorageError::IoError:
        case storage::StorageError::Busy:
            return DictError::StorageError;
    }
    return DictError::StorageError;
}

}  // namespace

SqliteDictionary::SqliteDictionary(storage::Database db, storage::Statement lookup_stmt) noexcept
    : db_(std::move(db)), lookup_stmt_(std::move(lookup_stmt)) {}

SqliteDictionary::SqliteDictionary(SqliteDictionary&& other) noexcept
    : db_(std::move(other.db_)), lookup_stmt_(std::move(other.lookup_stmt_)) {}

SqliteDictionary& SqliteDictionary::operator=(SqliteDictionary&& other) noexcept {
    if (this != &other) {
        db_ = std::move(other.db_);
        lookup_stmt_ = std::move(other.lookup_stmt_);
    }
    return *this;
}

SqliteDictionary::~SqliteDictionary() = default;

auto SqliteDictionary::open(storage::Database db) -> std::expected<SqliteDictionary, DictError> {
    auto stmt_or = db.prepare(kLookupSql);
    if (!stmt_or) {
        return std::unexpected(mapStorageError(stmt_or.error()));
    }
    return SqliteDictionary(std::move(db), std::move(stmt_or.value()));
}

auto SqliteDictionary::lookup(std::string_view word) const -> std::expected<Entry, DictError> {
    if (word.empty() || word.size() > kMaxWordLen) {
        return std::unexpected(DictError::InvalidInput);
    }

    std::lock_guard<std::mutex> guard(stmt_mutex_);

    if (auto r = lookup_stmt_.reset(); !r) {
        return std::unexpected(mapStorageError(r.error()));
    }
    if (auto r = lookup_stmt_.clearBindings(); !r) {
        return std::unexpected(mapStorageError(r.error()));
    }
    if (auto r = lookup_stmt_.bind(1, word); !r) {
        return std::unexpected(mapStorageError(r.error()));
    }

    auto step = lookup_stmt_.step();
    if (!step) {
        return std::unexpected(mapStorageError(step.error()));
    }
    if (!step.value()) {
        return std::unexpected(DictError::NotFound);
    }

    Entry entry;
    entry.headword = lookup_stmt_.columnText(0);
    entry.phonetic = lookup_stmt_.columnText(1);
    entry.definitions = parseDefinitionsJson(lookup_stmt_.columnText(2));
    return entry;
}

auto SqliteDictionary::suggest(std::string_view /*prefix*/, std::size_t /*max*/) const
    -> std::vector<std::string> {
    // Stubbed until iter-006-fuzzy. Returning empty is contract-conformant
    // ("empty prefix returns empty vector"). For non-empty prefixes, an empty
    // result is still permissible — there are simply no suggestions yet.
    return {};
}

}  // namespace easyenglish::core::dictionary
