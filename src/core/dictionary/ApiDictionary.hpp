#pragma once

#include <memory>
#include <utility>

#include "core/dictionary/IDictionary.hpp"
#include "core/network/INetworkClient.hpp"

namespace easyenglish::core::dictionary {

/// `IDictionary` that fetches definitions over HTTP from
/// `https://api.dictionaryapi.dev/api/v2/entries/en/<word>`. The endpoint is
/// free, requires no API key, and returns a stable JSON shape suitable for
/// parsing without third-party libraries (QJsonDocument is enough).
///
/// All actual network I/O is delegated to an injected `INetworkClient` —
/// tests pass a mock that returns canned bytes, so this class is exercised
/// fully offline in CI.
class ApiDictionary final : public IDictionary {
public:
    explicit ApiDictionary(std::shared_ptr<network::INetworkClient> client) noexcept
        : client_(std::move(client)) {}
    ~ApiDictionary() override = default;

    auto lookup(std::string_view word) const -> std::expected<Entry, DictError> override;

    /// The dictionaryapi.dev endpoint has no suggestions facility — we
    /// always return empty. iter-006's local fuzzy match covers this need.
    auto suggest(std::string_view prefix, std::size_t max = 10) const
        -> std::vector<std::string> override;

    /// Override the default base URL (used by tests; the real client points at
    /// `https://api.dictionaryapi.dev/api/v2/entries/en/`).
    void setBaseUrl(QString base);

private:
    std::shared_ptr<network::INetworkClient> client_;
    QString base_url_{QStringLiteral("https://api.dictionaryapi.dev/api/v2/entries/en/")};
};

}  // namespace easyenglish::core::dictionary
