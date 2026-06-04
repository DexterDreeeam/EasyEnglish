#include "core/dictionary/ApiDictionary.hpp"

#include <cctype>
#include <cstdio>
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
    if (base.empty() || base.back() != '/') {
        base.push_back('/');
    }
    base_url_ = std::move(base);
}

auto ApiDictionary::lookup(std::string_view word) const -> std::expected<Entry, DictError> {
    if (word.empty() || word.size() > 128) {
        return std::unexpected(DictError::InvalidInput);
    }
    if (!client_) {
        return std::unexpected(DictError::StorageError);
    }

    const std::string url = base_url_ + urlEncode(word);
    auto body_or = client_->get(url);
    if (!body_or) {
        return std::unexpected(mapNetworkError(body_or.error()));
    }

    const auto root = nlohmann::json::parse(body_or.value(), nullptr, false);
    if (root.is_discarded() || !root.is_array() || root.empty() || !root[0].is_object()) {
        return std::unexpected(DictError::NotFound);
    }
    const auto& obj = root[0];

    Entry entry;
    if (obj.contains("word") && obj["word"].is_string()) {
        entry.headword = obj["word"].get<std::string>();
    }
    if (entry.headword.empty()) {
        entry.headword = std::string(word);  // fall back to caller's input
    }

    if (obj.contains("phonetics") && obj["phonetics"].is_array()) {
        for (const auto& p : obj["phonetics"]) {
            if (p.is_object() && p.contains("text") && p["text"].is_string()) {
                auto text = p["text"].get<std::string>();
                if (!text.empty()) {
                    entry.phonetic = std::move(text);
                    break;
                }
            }
        }
    }

    if (obj.contains("meanings") && obj["meanings"].is_array()) {
        for (const auto& m : obj["meanings"]) {
            if (!m.is_object() || !m.contains("definitions") || !m["definitions"].is_array()) {
                continue;
            }
            for (const auto& d : m["definitions"]) {
                if (d.is_object() && d.contains("definition") && d["definition"].is_string()) {
                    auto text = d["definition"].get<std::string>();
                    if (!text.empty()) {
                        entry.definitions.push_back(std::move(text));
                    }
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
