// cpp-httplib's SSL helpers are gated behind this define. The vcpkg
// `cpp-httplib[openssl]` feature compiles with it set, but defining here too
// guards against the header being included from a TU that does not see the
// vcpkg-provided define on some toolchains.
#ifndef CPPHTTPLIB_OPENSSL_SUPPORT
#define CPPHTTPLIB_OPENSSL_SUPPORT
#endif

#include "core/network/HttpNetworkClient.hpp"

#include <string_view>

#include <httplib.h>

namespace easyenglish::core::network {

namespace {

/// Split "https://host[:port]/path?query" into ("https://host[:port]", "/path?query").
/// Returns an empty pair if the URL is malformed.
std::pair<std::string, std::string> splitUrl(std::string_view url) {
    const auto scheme_end = url.find("://");
    if (scheme_end == std::string_view::npos) {
        return {};
    }
    const auto path_start = url.find('/', scheme_end + 3);
    if (path_start == std::string_view::npos) {
        return {std::string(url), "/"};
    }
    return {std::string(url.substr(0, path_start)), std::string(url.substr(path_start))};
}

}  // namespace

HttpNetworkClient::HttpNetworkClient(int timeout_ms) noexcept : timeout_ms_(timeout_ms) {}

auto HttpNetworkClient::get(const std::string& url) const
    -> std::expected<std::string, NetworkError> {
    const auto [scheme_host, path] = splitUrl(url);
    if (scheme_host.empty()) {
        return std::unexpected(NetworkError::InvalidResponse);
    }

    httplib::Client cli(scheme_host);
    cli.set_connection_timeout(0, static_cast<long>(timeout_ms_) * 1000);  // us
    cli.set_read_timeout(0, static_cast<long>(timeout_ms_) * 1000);
    cli.set_follow_location(true);

    auto res = cli.Get(path);
    if (!res) {
        // httplib's Error enum collapses every non-network-success into one of
        // ~10 codes; bucket coarsely into our taxonomy.
        switch (res.error()) {
            case httplib::Error::ConnectionTimeout:
            case httplib::Error::Read:
            case httplib::Error::Write:
                return std::unexpected(NetworkError::Timeout);
            case httplib::Error::Connection:
            case httplib::Error::BindIPAddress:
            case httplib::Error::Canceled:
                return std::unexpected(NetworkError::Offline);
            default:
                return std::unexpected(NetworkError::HttpError);
        }
    }
    if (res->status < 200 || res->status >= 300) {
        return std::unexpected(NetworkError::HttpError);
    }
    return res->body;
}

}  // namespace easyenglish::core::network
