#pragma once

#include <cstddef>
#include <expected>
#include <string_view>
#include <vector>

#include "core/dictionary/Entry.hpp"
#include "core/dictionary/errors.hpp"

namespace easyenglish::core::dictionary {

/// Abstract dictionary interface (frozen, see docs/contracts/dictionary.md).
///
/// All methods are `const` and required to be safe for concurrent invocation
/// on the same instance — concrete implementations are responsible for any
/// internal synchronization. UI/app layers depend on this interface, never on
/// concrete subclasses, so they can be tested with mocks.
class IDictionary {
public:
    IDictionary() = default;
    IDictionary(const IDictionary&) = delete;
    IDictionary& operator=(const IDictionary&) = delete;
    IDictionary(IDictionary&&) = delete;
    IDictionary& operator=(IDictionary&&) = delete;
    virtual ~IDictionary() = default;

    /// Exact lookup. Empty input → `DictError::InvalidInput`.
    /// Case-insensitive: the returned `Entry::headword` is the dictionary's
    /// canonical casing, not the caller's input.
    virtual auto lookup(std::string_view word) const -> std::expected<Entry, DictError> = 0;

    /// Suggestions ordered by ascending edit distance. `max` caps result length.
    /// Empty prefix returns an empty vector (never an error).
    /// Iter-006 will implement; until then implementations may stub with [].
    virtual auto suggest(std::string_view prefix, std::size_t max = 10) const
        -> std::vector<std::string> = 0;
};

}  // namespace easyenglish::core::dictionary
