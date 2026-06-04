#include "core/dictionary/ApiDictionary.hpp"

#include <cctype>
#include <cstdio>
#include <unordered_set>
#include <utility>

#include <nlohmann/json.hpp>

namespace easyenglish::core::dictionary {

namespace {

DictError mapNetworkError(network::NetworkError /*e*/) noexcept {
    return DictError::StorageError;  // bucketed as "data layer failed"
}

std::string urlEncode(std::string_view s) {
    std::string out;
    out.reserve(s.size() * 3);
    for (unsigned char c : s) {
        const bool unreserved = (c >= '0' && c <= '9') || (c >= 'A' && c <= 'Z') ||
                                (c >= 'a' && c <= 'z') || c == '-' || c == '_' || c == '.' ||
                                c == '~';
        if (unreserved) {
            out.push_back(static_cast<char>(c));
        } else {
            char buf[4];
            std::snprintf(buf, sizeof(buf), "%%%02X", c);
            out.append(buf, 3);
        }
    }
    return out;
}

}  // namespace

void ApiDictionary::setBaseUrl(std::string base) {
    base_url_ = std::move(base);
}

auto ApiDictionary::lookup(std::string_view word) const -> std::expected<Entry, DictError> {
    if (word.empty() || word.size() > 128) {
        return std::unexpected(DictError::InvalidInput);
    }
    if (!client_) {
        return std::unexpected(DictError::StorageError);
    }

    // MyMemory query string: ?q=<word>&langpair=en|zh-CN. We don't sign or
    // pass an email so we stay on the anonymous quota (~5k chars/day, plenty
    // for an interactive translator).
    const std::string url = base_url_ + "?q=" + urlEncode(word) + "&langpair=en%7Czh-CN";
    auto body_or = client_->get(url);
    if (!body_or) {
        return std::unexpected(mapNetworkError(body_or.error()));
    }

    const auto root = nlohmann::json::parse(body_or.value(), nullptr, false);
    if (root.is_discarded() || !root.is_object()) {
        return std::unexpected(DictError::NotFound);
    }

    Entry entry;
    entry.headword = std::string(word);

    // Primary translation: responseData.translatedText.
    if (const auto rd = root.find("responseData"); rd != root.end() && rd->is_object() &&
                                                   rd->contains("translatedText") &&
                                                   (*rd)["translatedText"].is_string()) {
        auto t = (*rd)["translatedText"].get<std::string>();
        // MyMemory returns the original word verbatim when nothing matches —
        // treat that as "not found" so users don't see a useless echo.
        if (!t.empty() && t != entry.headword) {
            entry.definitions.push_back(std::move(t));
        }
    }

    // Additional translations from matches[].translation, deduplicated.
    if (root.contains("matches") && root["matches"].is_array()) {
        std::unordered_set<std::string> seen(entry.definitions.begin(), entry.definitions.end());
        for (const auto& m : root["matches"]) {
            if (!m.is_object() || !m.contains("translation") || !m["translation"].is_string()) {
                continue;
            }
            auto t = m["translation"].get<std::string>();
            if (t.empty() || t == entry.headword) {
                continue;
            }
            if (seen.insert(t).second) {
                entry.definitions.push_back(std::move(t));
                if (entry.definitions.size() >= 8) {
                    break;
                }
            }
        }
    }

    if (entry.definitions.empty()) {
        return std::unexpected(DictError::NotFound);
    }
    return entry;
}

auto ApiDictionary::suggest(std::string_view /*prefix*/, std::size_t /*max*/) const
    -> std::vector<std::string> {
    return {};
}

}  // namespace easyenglish::core::dictionary
