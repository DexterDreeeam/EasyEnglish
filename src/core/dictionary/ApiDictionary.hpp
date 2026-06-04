#pragma once

#include <memory>
#include <string>
#include <utility>

#include "core/dictionary/IDictionary.hpp"
#include "core/network/INetworkClient.hpp"

namespace easyenglish::core::dictionary {

/// `IDictionary` that fetches English -> Chinese translations from
/// https://api.mymemory.translated.net/get?q=<word>&langpair=en|zh-CN
/// — an anonymous, free, no-key endpoint suitable for occasional lookups.
///
/// Network I/O is delegated to an injected `INetworkClient`; tests pass a
/// mock that returns canned bytes so this class is exercised fully offline
/// in CI.
class ApiDictionary final : public IDictionary {
public:
    explicit ApiDictionary(std::shared_ptr<network::INetworkClient> client) noexcept
        : client_(std::move(client)) {}
    ~ApiDictionary() override = default;

    auto lookup(std::string_view word) const -> std::expected<Entry, DictError> override;

    /// MyMemory has no suggestions facility — always empty.
    auto suggest(std::string_view prefix, std::size_t max = 10) const
        -> std::vector<std::string> override;

    /// Override the default endpoint base (used by tests). The query string
    /// `?q=<word>&langpair=en|zh-CN` is appended by `lookup()`.
    void setBaseUrl(std::string base);

private:
    std::shared_ptr<network::INetworkClient> client_;
    std::string base_url_{"https://api.mymemory.translated.net/get"};
};

}  // namespace easyenglish::core::dictionary
