#pragma once

#include <expected>
#include <string>

#include "core/network/INetworkClient.hpp"

namespace easyenglish::core::network {

/// Real `INetworkClient` backed by cpp-httplib. SSL is enabled via the vcpkg
/// `cpp-httplib[openssl]` feature, so `https://` URLs work out of the box.
///
/// One `httplib::Client` is created per call (cheap relative to network RTT),
/// making this class trivially thread-safe.
class HttpNetworkClient final : public INetworkClient {
public:
    static constexpr int kDefaultTimeoutMs = 5000;

    explicit HttpNetworkClient(int timeout_ms = kDefaultTimeoutMs) noexcept;
    ~HttpNetworkClient() override = default;

    auto get(const std::string& url) const -> std::expected<std::string, NetworkError> override;

private:
    int timeout_ms_;
};

}  // namespace easyenglish::core::network
