#include <cstring>
#include <memory>
#include <optional>

#include <gtest/gtest.h>

#include <QByteArray>
#include <QString>

#include "core/dictionary/ApiDictionary.hpp"
#include "core/network/INetworkClient.hpp"

using easyenglish::core::dictionary::ApiDictionary;
using easyenglish::core::dictionary::DictError;
using easyenglish::core::network::INetworkClient;
using easyenglish::core::network::NetworkError;

namespace {

/// Deterministic in-process replacement for QtNetworkClient. The test sets a
/// canned response (bytes or error) and inspects the requested URL afterwards.
class MockNetworkClient final : public INetworkClient {
public:
    auto get(const QString& url) const -> std::expected<QByteArray, NetworkError> override {
        last_url_ = url;
        ++calls_;
        if (canned_error_.has_value()) {
            return std::unexpected(canned_error_.value());
        }
        return canned_body_;
    }

    void setBody(QByteArray body) {
        canned_body_ = std::move(body);
        canned_error_.reset();
    }
    void setError(NetworkError e) {
        canned_error_ = e;
        canned_body_.clear();
    }

    [[nodiscard]] const QString& lastUrl() const { return last_url_; }
    [[nodiscard]] int calls() const { return calls_; }

private:
    mutable int calls_{0};
    mutable QString last_url_;
    QByteArray canned_body_;
    std::optional<NetworkError> canned_error_;
};

constexpr const char* kAppleJson = R"json(
[{
  "word":"apple",
  "phonetics":[{"text":"/ˈæp.əl/"}],
  "meanings":[
    {"definitions":[{"definition":"a fruit"}]},
    {"definitions":[{"definition":"the tree"}]}
  ]
}]
)json";

}  // namespace

TEST(ApiDictionary, ParsesValidJsonResponse) {
    auto mock = std::make_shared<MockNetworkClient>();
    mock->setBody(QByteArray::fromRawData(kAppleJson, static_cast<int>(strlen(kAppleJson))));

    ApiDictionary dict(mock);
    auto entry = dict.lookup("apple");
    ASSERT_TRUE(entry.has_value());
    EXPECT_EQ(entry->headword, "apple");
    EXPECT_EQ(entry->phonetic, "/ˈæp.əl/");
    ASSERT_EQ(entry->definitions.size(), 2u);
    EXPECT_EQ(entry->definitions[0], "a fruit");
    EXPECT_EQ(entry->definitions[1], "the tree");
}

TEST(ApiDictionary, BuildsRequestUrlFromBase) {
    auto mock = std::make_shared<MockNetworkClient>();
    mock->setBody(QByteArray::fromRawData(kAppleJson, static_cast<int>(strlen(kAppleJson))));

    ApiDictionary dict(mock);
    dict.setBaseUrl(QStringLiteral("https://example.test/api/"));
    (void)dict.lookup("apple");
    EXPECT_EQ(mock->lastUrl(), QStringLiteral("https://example.test/api/apple"));
}

TEST(ApiDictionary, AppendsTrailingSlashIfMissing) {
    auto mock = std::make_shared<MockNetworkClient>();
    mock->setBody(QByteArray::fromRawData(kAppleJson, static_cast<int>(strlen(kAppleJson))));

    ApiDictionary dict(mock);
    dict.setBaseUrl(QStringLiteral("https://example.test/api"));  // no slash
    (void)dict.lookup("apple");
    EXPECT_EQ(mock->lastUrl(), QStringLiteral("https://example.test/api/apple"));
}

TEST(ApiDictionary, PercentEncodesWord) {
    auto mock = std::make_shared<MockNetworkClient>();
    mock->setBody(QByteArray("[]"));
    ApiDictionary dict(mock);
    dict.setBaseUrl(QStringLiteral("https://example.test/"));
    (void)dict.lookup("hot dog");
    EXPECT_TRUE(mock->lastUrl().contains(QStringLiteral("hot%20dog")));
}

TEST(ApiDictionary, EmptyWordReturnsInvalidInput) {
    auto mock = std::make_shared<MockNetworkClient>();
    ApiDictionary dict(mock);
    auto result = dict.lookup("");
    ASSERT_FALSE(result.has_value());
    EXPECT_EQ(result.error(), DictError::InvalidInput);
    EXPECT_EQ(mock->calls(), 0) << "Empty input must short-circuit before any HTTP call";
}

TEST(ApiDictionary, NetworkOfflineMapsToStorageError) {
    auto mock = std::make_shared<MockNetworkClient>();
    mock->setError(NetworkError::Offline);
    ApiDictionary dict(mock);
    auto result = dict.lookup("apple");
    ASSERT_FALSE(result.has_value());
    EXPECT_EQ(result.error(), DictError::StorageError);
}

TEST(ApiDictionary, EmptyJsonArrayMapsToNotFound) {
    auto mock = std::make_shared<MockNetworkClient>();
    mock->setBody(QByteArray("[]"));
    ApiDictionary dict(mock);
    auto result = dict.lookup("nosuch");
    ASSERT_FALSE(result.has_value());
    EXPECT_EQ(result.error(), DictError::NotFound);
}

TEST(ApiDictionary, MalformedJsonMapsToNotFound) {
    auto mock = std::make_shared<MockNetworkClient>();
    mock->setBody(QByteArray("not json at all"));
    ApiDictionary dict(mock);
    auto result = dict.lookup("apple");
    ASSERT_FALSE(result.has_value());
    EXPECT_EQ(result.error(), DictError::NotFound);
}

TEST(ApiDictionary, SuggestAlwaysEmpty) {
    auto mock = std::make_shared<MockNetworkClient>();
    ApiDictionary dict(mock);
    EXPECT_TRUE(dict.suggest("appl", 5).empty());
}
