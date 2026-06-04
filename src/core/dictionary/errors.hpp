#pragma once

#include <string_view>

namespace easyenglish::core::dictionary {

enum class DictError {
    NotFound,
    InvalidInput,
    StorageError,
};

constexpr std::string_view toString(DictError e) noexcept {
    switch (e) {
        case DictError::NotFound:
            return "NotFound";
        case DictError::InvalidInput:
            return "InvalidInput";
        case DictError::StorageError:
            return "StorageError";
    }
    return "Unknown";
}

}  // namespace easyenglish::core::dictionary
