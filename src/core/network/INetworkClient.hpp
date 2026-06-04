#pragma once

#include <expected>
#include <string>

namespace easyenglish::core::network {

enum class NetworkError {
    Timeout,
    HttpError,
    InvalidResponse,
    Offline,
};

/// Synchronous HTTP GET abstraction. Concrete implementations:
/// - `HttpNetworkClient` (real, cpp-httplib)
/// - `MockNetworkClient` (test-only)
/// Callers depend on the interface rather than the concrete type so unit tests
/// run fully offline.
class INetworkClient {
public:
    INetworkClient() = default;
    INetworkClient(const INetworkClient&) = delete;
    INetworkClient& operator=(const INetworkClient&) = delete;
    INetworkClient(INetworkClient&&) = delete;
    INetworkClient& operator=(INetworkClient&&) = delete;
    virtual ~INetworkClient() = default;

    /// Returns response body on 2xx, otherwise a NetworkError. Implementations
    /// must be safe to call from multiple threads concurrently.
    virtual auto get(const std::string& url) const -> std::expected<std::string, NetworkError> = 0;
};

}  // namespace easyenglish::core::network
