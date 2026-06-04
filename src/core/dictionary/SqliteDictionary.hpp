#pragma once

#include <memory>
#include <mutex>

#include "core/dictionary/IDictionary.hpp"
#include "core/storage/Database.hpp"
#include "core/storage/Statement.hpp"

namespace easyenglish::core::dictionary {

/// `IDictionary` backed by the `easyenglish::core::storage::Database` schema
/// (`entries(headword TEXT PRIMARY KEY COLLATE NOCASE, phonetic TEXT,
/// definitions TEXT)` where `definitions` is a JSON array of strings).
///
/// The class owns the underlying `Database` and a cached prepared statement
/// to avoid re-preparing on every `lookup()`. A mutex serializes access to
/// the cached statement so `lookup()` is safe to call from multiple threads
/// (as required by the contract).
class SqliteDictionary final : public IDictionary {
public:
    static auto open(storage::Database db) -> std::expected<SqliteDictionary, DictError>;

    SqliteDictionary(const SqliteDictionary&) = delete;
    SqliteDictionary& operator=(const SqliteDictionary&) = delete;
    SqliteDictionary(SqliteDictionary&&) noexcept;
    SqliteDictionary& operator=(SqliteDictionary&&) noexcept;
    ~SqliteDictionary() override;

    auto lookup(std::string_view word) const -> std::expected<Entry, DictError> override;

    auto suggest(std::string_view prefix, std::size_t max = 10) const
        -> std::vector<std::string> override;

private:
    SqliteDictionary(storage::Database db, storage::Statement lookup_stmt) noexcept;

    static constexpr std::size_t kMaxWordLen = 128;

    mutable std::mutex stmt_mutex_;
    storage::Database db_;
    mutable storage::Statement lookup_stmt_;
};

}  // namespace easyenglish::core::dictionary
