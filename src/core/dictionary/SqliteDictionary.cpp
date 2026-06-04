#include "core/dictionary/SqliteDictionary.hpp"

#include <algorithm>
#include <cctype>
#include <utility>
#include <vector>

#include <nlohmann/json.hpp>

namespace easyenglish::core::dictionary {

namespace {

constexpr const char* kLookupSql =
    "SELECT headword, phonetic, definitions FROM entries "
    "WHERE headword = ?1 COLLATE NOCASE LIMIT 1;";

constexpr const char* kListHeadwordsSql = "SELECT headword FROM entries;";

std::vector<std::string> parseDefinitionsJson(const std::string& json) {
    // Definitions are persisted by tools/seed_db.py as a JSON array of strings.
    std::vector<std::string> out;
    const auto doc = nlohmann::json::parse(json, nullptr, false);
    if (doc.is_discarded() || !doc.is_array()) {
        return out;
    }
    out.reserve(doc.size());
    for (const auto& v : doc) {
        if (v.is_string()) {
            out.push_back(v.get<std::string>());
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

std::string toLowerAscii(std::string_view s) {
    std::string out;
    out.reserve(s.size());
    for (char c : s) {
        out.push_back(static_cast<char>(std::tolower(static_cast<unsigned char>(c))));
    }
    return out;
}

/// Standard Levenshtein distance with two rolling rows.
int levenshtein(std::string_view a, std::string_view b) {
    if (a.empty())
        return static_cast<int>(b.size());
    if (b.empty())
        return static_cast<int>(a.size());
    const std::size_t m = a.size();
    const std::size_t n = b.size();
    std::vector<int> prev(n + 1);
    std::vector<int> curr(n + 1);
    for (std::size_t j = 0; j <= n; ++j) {
        prev[j] = static_cast<int>(j);
    }
    for (std::size_t i = 1; i <= m; ++i) {
        curr[0] = static_cast<int>(i);
        for (std::size_t j = 1; j <= n; ++j) {
            const int cost = (a[i - 1] == b[j - 1]) ? 0 : 1;
            curr[j] = std::min({prev[j] + 1, curr[j - 1] + 1, prev[j - 1] + cost});
        }
        std::swap(prev, curr);
    }
    return prev[n];
}

}  // namespace

SqliteDictionary::SqliteDictionary(storage::Database db, storage::Statement lookup_stmt,
                                   std::vector<std::string> headwords) noexcept
    : db_(std::move(db)),
      lookup_stmt_(std::move(lookup_stmt)),
      headwords_cache_(std::move(headwords)) {
    std::sort(headwords_cache_.begin(), headwords_cache_.end());
    headwords_lower_.reserve(headwords_cache_.size());
    for (const auto& hw : headwords_cache_) {
        headwords_lower_.push_back(toLowerAscii(hw));
    }
}

SqliteDictionary::SqliteDictionary(SqliteDictionary&& other) noexcept
    : db_(std::move(other.db_)),
      lookup_stmt_(std::move(other.lookup_stmt_)),
      headwords_cache_(std::move(other.headwords_cache_)),
      headwords_lower_(std::move(other.headwords_lower_)) {}

SqliteDictionary& SqliteDictionary::operator=(SqliteDictionary&& other) noexcept {
    if (this != &other) {
        db_ = std::move(other.db_);
        lookup_stmt_ = std::move(other.lookup_stmt_);
        headwords_cache_ = std::move(other.headwords_cache_);
        headwords_lower_ = std::move(other.headwords_lower_);
    }
    return *this;
}

SqliteDictionary::~SqliteDictionary() = default;

auto SqliteDictionary::open(storage::Database db) -> std::expected<SqliteDictionary, DictError> {
    auto stmt_or = db.prepare(kLookupSql);
    if (!stmt_or) {
        return std::unexpected(mapStorageError(stmt_or.error()));
    }
    std::vector<std::string> headwords;
    auto list_or = db.prepare(kListHeadwordsSql);
    if (!list_or) {
        return std::unexpected(mapStorageError(list_or.error()));
    }
    while (true) {
        auto step = list_or.value().step();
        if (!step) {
            return std::unexpected(mapStorageError(step.error()));
        }
        if (!step.value()) {
            break;
        }
        headwords.push_back(list_or.value().columnText(0));
    }
    return SqliteDictionary(std::move(db), std::move(stmt_or.value()), std::move(headwords));
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

auto SqliteDictionary::suggest(std::string_view prefix, std::size_t max) const
    -> std::vector<std::string> {
    if (prefix.empty() || max == 0) {
        return {};
    }
    const std::string query = toLowerAscii(prefix);

    struct Scored {
        std::size_t index;
        int distance;
    };
    std::vector<Scored> scored;
    scored.reserve(headwords_lower_.size());
    for (std::size_t i = 0; i < headwords_lower_.size(); ++i) {
        scored.push_back({i, levenshtein(query, headwords_lower_[i])});
    }
    std::stable_sort(scored.begin(), scored.end(),
                     [](const Scored& a, const Scored& b) { return a.distance < b.distance; });

    std::vector<std::string> out;
    const std::size_t n = std::min(max, scored.size());
    out.reserve(n);
    for (std::size_t i = 0; i < n; ++i) {
        out.push_back(headwords_cache_[scored[i].index]);
    }
    return out;
}

}  // namespace easyenglish::core::dictionary

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

std::string toLowerAscii(std::string_view s) {
    std::string out;
    out.reserve(s.size());
    for (char c : s) {
        out.push_back(static_cast<char>(std::tolower(static_cast<unsigned char>(c))));
    }
    return out;
}

/// Standard Levenshtein distance with two rolling rows. Inputs are expected to
/// be already lowercased ASCII; non-ASCII bytes are compared verbatim, which is
/// fine for spelling-suggestion ranking on Latin-script dictionaries.
int levenshtein(std::string_view a, std::string_view b) {
    if (a.empty())
        return static_cast<int>(b.size());
    if (b.empty())
        return static_cast<int>(a.size());
    const std::size_t m = a.size();
    const std::size_t n = b.size();
    std::vector<int> prev(n + 1);
    std::vector<int> curr(n + 1);
    for (std::size_t j = 0; j <= n; ++j) {
        prev[j] = static_cast<int>(j);
    }
    for (std::size_t i = 1; i <= m; ++i) {
        curr[0] = static_cast<int>(i);
        for (std::size_t j = 1; j <= n; ++j) {
            const int cost = (a[i - 1] == b[j - 1]) ? 0 : 1;
            curr[j] = std::min({prev[j] + 1, curr[j - 1] + 1, prev[j - 1] + cost});
        }
        std::swap(prev, curr);
    }
    return prev[n];
}

}  // namespace

SqliteDictionary::SqliteDictionary(storage::Database db, storage::Statement lookup_stmt,
                                   std::vector<std::string> headwords) noexcept
    : db_(std::move(db)),
      lookup_stmt_(std::move(lookup_stmt)),
      headwords_cache_(std::move(headwords)) {
    // Sort and build lower-cased parallel array so suggest() ranks are
    // deterministic (alphabetical tiebreak after edit-distance).
    std::sort(headwords_cache_.begin(), headwords_cache_.end());
    headwords_lower_.reserve(headwords_cache_.size());
    for (const auto& hw : headwords_cache_) {
        headwords_lower_.push_back(toLowerAscii(hw));
    }
}

SqliteDictionary::SqliteDictionary(SqliteDictionary&& other) noexcept
    : db_(std::move(other.db_)),
      lookup_stmt_(std::move(other.lookup_stmt_)),
      headwords_cache_(std::move(other.headwords_cache_)),
      headwords_lower_(std::move(other.headwords_lower_)) {}

SqliteDictionary& SqliteDictionary::operator=(SqliteDictionary&& other) noexcept {
    if (this != &other) {
        db_ = std::move(other.db_);
        lookup_stmt_ = std::move(other.lookup_stmt_);
        headwords_cache_ = std::move(other.headwords_cache_);
        headwords_lower_ = std::move(other.headwords_lower_);
    }
    return *this;
}

SqliteDictionary::~SqliteDictionary() = default;

auto SqliteDictionary::open(storage::Database db) -> std::expected<SqliteDictionary, DictError> {
    auto stmt_or = db.prepare(kLookupSql);
    if (!stmt_or) {
        return std::unexpected(mapStorageError(stmt_or.error()));
    }
    // Eagerly load all headwords for suggest(); a 100k-entry dictionary is only
    // a few MB of strings and avoiding SQL roundtrips per keystroke matters.
    std::vector<std::string> headwords;
    auto list_or = db.prepare(kListHeadwordsSql);
    if (!list_or) {
        return std::unexpected(mapStorageError(list_or.error()));
    }
    while (true) {
        auto step = list_or.value().step();
        if (!step) {
            return std::unexpected(mapStorageError(step.error()));
        }
        if (!step.value()) {
            break;
        }
        headwords.push_back(list_or.value().columnText(0));
    }
    return SqliteDictionary(std::move(db), std::move(stmt_or.value()), std::move(headwords));
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

auto SqliteDictionary::suggest(std::string_view prefix, std::size_t max) const
    -> std::vector<std::string> {
    if (prefix.empty() || max == 0) {
        return {};
    }
    const std::string query = toLowerAscii(prefix);

    struct Scored {
        std::size_t index;
        int distance;
    };
    std::vector<Scored> scored;
    scored.reserve(headwords_lower_.size());
    for (std::size_t i = 0; i < headwords_lower_.size(); ++i) {
        scored.push_back({i, levenshtein(query, headwords_lower_[i])});
    }
    // Sort by ascending distance, then alphabetical tiebreak (headwords_cache_
    // is already sorted, so a stable sort preserves that order within ties).
    std::stable_sort(scored.begin(), scored.end(),
                     [](const Scored& a, const Scored& b) { return a.distance < b.distance; });

    std::vector<std::string> out;
    const std::size_t n = std::min(max, scored.size());
    out.reserve(n);
    for (std::size_t i = 0; i < n; ++i) {
        out.push_back(headwords_cache_[scored[i].index]);
    }
    return out;
}

}  // namespace easyenglish::core::dictionary
