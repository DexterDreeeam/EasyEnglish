#include "core/dictionary/ApiDictionary.hpp"

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QString>
#include <QUrl>

namespace easyenglish::core::dictionary {

namespace {

DictError mapNetworkError(network::NetworkError e) noexcept {
    switch (e) {
        case network::NetworkError::Timeout:
        case network::NetworkError::Offline:
        case network::NetworkError::InvalidResponse:
        case network::NetworkError::HttpError:
            return DictError::StorageError;  // bucketed as "data layer failed"
    }
    return DictError::StorageError;
}

}  // namespace

void ApiDictionary::setBaseUrl(QString base) {
    if (!base.endsWith(QLatin1Char('/'))) {
        base.append(QLatin1Char('/'));
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
    const QString encoded = QString::fromUtf8(
        QUrl::toPercentEncoding(QByteArray(word.data(), static_cast<qsizetype>(word.size()))));
    const QString url = base_url_ + encoded;

    auto bytes_or = client_->get(url);
    if (!bytes_or) {
        return std::unexpected(mapNetworkError(bytes_or.error()));
    }

    const auto doc = QJsonDocument::fromJson(bytes_or.value());
    if (!doc.isArray()) {
        return std::unexpected(DictError::NotFound);
    }
    const auto arr = doc.array();
    if (arr.isEmpty() || !arr.at(0).isObject()) {
        return std::unexpected(DictError::NotFound);
    }

    const QJsonObject root = arr.at(0).toObject();

    Entry entry;
    entry.headword = root.value(QStringLiteral("word")).toString().toStdString();
    if (entry.headword.empty()) {
        entry.headword = std::string(word);  // fall back to caller's casing
    }

    // Phonetics: prefer the first non-empty `text` from the `phonetics` array.
    if (const auto phonetics = root.value(QStringLiteral("phonetics")); phonetics.isArray()) {
        for (const auto& p : phonetics.toArray()) {
            if (!p.isObject())
                continue;
            const auto text = p.toObject().value(QStringLiteral("text")).toString();
            if (!text.isEmpty()) {
                entry.phonetic = text.toStdString();
                break;
            }
        }
    }

    // Definitions: meanings[].definitions[].definition (flattened).
    if (const auto meanings = root.value(QStringLiteral("meanings")); meanings.isArray()) {
        for (const auto& m : meanings.toArray()) {
            if (!m.isObject())
                continue;
            const auto defs = m.toObject().value(QStringLiteral("definitions"));
            if (!defs.isArray())
                continue;
            for (const auto& d : defs.toArray()) {
                if (!d.isObject())
                    continue;
                const auto text = d.toObject().value(QStringLiteral("definition")).toString();
                if (!text.isEmpty()) {
                    entry.definitions.push_back(text.toStdString());
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
