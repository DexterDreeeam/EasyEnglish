#pragma once

#include <expected>

#include "core/network/INetworkClient.hpp"

namespace easyenglish::core::network {

/// `INetworkClient` implemented with Qt's `QNetworkAccessManager`. Wraps the
/// async API in a blocking call using a local event loop, with a per-request
/// timeout to stop UI threads from hanging on a slow endpoint.
class QtNetworkClient final : public INetworkClient {
public:
    static constexpr int kDefaultTimeoutMs = 5000;

    explicit QtNetworkClient(int timeout_ms = kDefaultTimeoutMs) noexcept;
    ~QtNetworkClient() override = default;

    auto get(const QString& url) const -> std::expected<QByteArray, NetworkError> override;

private:
    int timeout_ms_;
};

}  // namespace easyenglish::core::network
