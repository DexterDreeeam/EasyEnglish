#pragma once

#include <string_view>

namespace easyenglish::core::storage {

enum class StorageError {
    NotFound,
    InvalidQuery,
    ConstraintViolation,
    IoError,
    Busy,
};

constexpr std::string_view toString(StorageError e) noexcept {
    switch (e) {
        case StorageError::NotFound:
            return "NotFound";
        case StorageError::InvalidQuery:
            return "InvalidQuery";
        case StorageError::ConstraintViolation:
            return "ConstraintViolation";
        case StorageError::IoError:
            return "IoError";
        case StorageError::Busy:
            return "Busy";
    }
    return "Unknown";
}

}  // namespace easyenglish::core::storage
