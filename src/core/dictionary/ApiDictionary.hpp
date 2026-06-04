#pragma once

#include <memory>
#include <string>
#include <utility>

#include "core/dictionary/IDictionary.hpp"
#include "core/network/INetworkClient.hpp"

namespace easyenglish::core::dictionary {

/// `IDictionary` that fetches definitions over HTTP from
/// `https://api.dictionaryapi.dev/api/v2/entries/en/<word>`. The endpoint is
/// free and requires no API key.
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

    /// The dictionaryapi.dev endpoint has no suggestions facility — always empty.
    auto suggest(std::string_view prefix, std::size_t max = 10) const
        -> std::vector<std::string> override;

    /// Override the default base URL (used by tests; production points at the
    /// dictionaryapi.dev base URL). Auto-appends a trailing slash if missing.
    void setBaseUrl(std::string base);

private:
    std::shared_ptr<network::INetworkClient> client_;
    std::string base_url_{"https://api.dictionaryapi.dev/api/v2/entries/en/"};
};

}  // namespace easyenglish::core::dictionary
