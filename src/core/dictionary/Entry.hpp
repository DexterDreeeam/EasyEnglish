#pragma once

#include <string>
#include <vector>

namespace easyenglish::core::dictionary {

/// One dictionary lookup result. Field semantics are documented in
/// docs/contracts/dictionary.md and frozen as of iter-002.
struct Entry {
    std::string headword;
    std::string phonetic;                  // IPA, may be empty
    std::vector<std::string> definitions;  // one per sense; non-empty when present

    friend bool operator==(const Entry&, const Entry&) = default;
};

}  // namespace easyenglish::core::dictionary
