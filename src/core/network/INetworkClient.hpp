#pragma once

#include <expected>

#include <QByteArray>
#include <QString>

namespace easyenglish::core::network {

enum class NetworkError {
    Timeout,
    HttpError,
    InvalidResponse,
    Offline,
};

/// Synchronous HTTP GET abstraction. Concrete implementations: `QtNetworkClient`
/// (real, uses QNetworkAccessManager) and `MockNetworkClient` (in tests). Callers
/// depending on the interface rather than the concrete type makes lookups testable
/// without network access.
class INetworkClient {
public:
    INetworkClient() = default;
    INetworkClient(const INetworkClient&) = delete;
    INetworkClient& operator=(const INetworkClient&) = delete;
    INetworkClient(INetworkClient&&) = delete;
    INetworkClient& operator=(INetworkClient&&) = delete;
    virtual ~INetworkClient() = default;

    /// Returns response body bytes on 2xx, otherwise a NetworkError.
    /// Implementations must be safe to call from the UI thread (block but
    /// pump events) and from concurrent threads.
    virtual auto get(const QString& url) const -> std::expected<QByteArray, NetworkError> = 0;
};

}  // namespace easyenglish::core::network
