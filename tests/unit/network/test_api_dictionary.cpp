#include <cstring>
#include <memory>
#include <optional>
#include <string>

#include <gtest/gtest.h>

#include "core/dictionary/ApiDictionary.hpp"
#include "core/network/INetworkClient.hpp"

using easyenglish::core::dictionary::ApiDictionary;
using easyenglish::core::dictionary::DictError;
using easyenglish::core::network::INetworkClient;
using easyenglish::core::network::NetworkError;

namespace {

/// Deterministic in-process replacement for HttpNetworkClient.
class MockNetworkClient final : public INetworkClient {
public:
    auto get(const std::string& url) const -> std::expected<std::string, NetworkError> override {
        last_url_ = url;
        ++calls_;
        if (canned_error_.has_value()) {
            return std::unexpected(canned_error_.value());
        }
        return canned_body_;
    }

    void setBody(std::string body) {
        canned_body_ = std::move(body);
        canned_error_.reset();
    }
    void setError(NetworkError e) {
        canned_error_ = e;
        canned_body_.clear();
    }

    [[nodiscard]] const std::string& lastUrl() const { return last_url_; }
    [[nodiscard]] int calls() const { return calls_; }

private:
    mutable int calls_{0};
    mutable std::string last_url_;
    std::string canned_body_;
    std::optional<NetworkError> canned_error_;
};

// Minimal MyMemory response shape; the real API also returns metadata fields
// we ignore. JSON intentionally non-pretty so the parsing exercises the
// "no whitespace" path too.
constexpr const char* kAppleJson = R"json(
{
  "responseData": {"translatedText": "苹果", "match": 1},
  "responseStatus": 200,
  "matches": [
    {"translation": "苹果", "quality": "70"},
    {"translation": "苹果树", "quality": "65"},
    {"translation": "苹果", "quality": "60"}
  ]
}
)json";

}  // namespace

TEST(ApiDictionary, ParsesPrimaryTranslation) {
    auto mock = std::make_shared<MockNetworkClient>();
    mock->setBody(kAppleJson);

    ApiDictionary dict(mock);
    auto entry = dict.lookup("apple");
    ASSERT_TRUE(entry.has_value());
    EXPECT_EQ(entry->headword, "apple");
    ASSERT_FALSE(entry->definitions.empty());
    EXPECT_EQ(entry->definitions.front(), "苹果");
}

TEST(ApiDictionary, AppendsUniqueAdditionalTranslations) {
    auto mock = std::make_shared<MockNetworkClient>();
    mock->setBody(kAppleJson);

    ApiDictionary dict(mock);
    auto entry = dict.lookup("apple");
    ASSERT_TRUE(entry.has_value());
    // Primary "苹果" + one unique additional "苹果树"; the duplicate "苹果"
    // in matches[] must be dropped.
    ASSERT_EQ(entry->definitions.size(), 2u);
    EXPECT_EQ(entry->definitions[0], "苹果");
    EXPECT_EQ(entry->definitions[1], "苹果树");
}

TEST(ApiDictionary, BuildsRequestUrlWithQueryString) {
    auto mock = std::make_shared<MockNetworkClient>();
    mock->setBody(kAppleJson);

    ApiDictionary dict(mock);
    dict.setBaseUrl("https://example.test/api");
    (void)dict.lookup("apple");
    EXPECT_EQ(mock->lastUrl(), "https://example.test/api?q=apple&langpair=en%7Czh-CN");
}

TEST(ApiDictionary, PercentEncodesWord) {
    auto mock = std::make_shared<MockNetworkClient>();
    mock->setBody("{\"responseData\":{\"translatedText\":\"热狗\"}}");
    ApiDictionary dict(mock);
    dict.setBaseUrl("https://example.test/api");
    (void)dict.lookup("hot dog");
    EXPECT_NE(mock->lastUrl().find("q=hot%20dog"), std::string::npos);
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

TEST(ApiDictionary, EchoOfQueryWordCountsAsNotFound) {
    // When MyMemory has no real translation it echoes the source word back.
    auto mock = std::make_shared<MockNetworkClient>();
    mock->setBody("{\"responseData\":{\"translatedText\":\"nosuch\"},\"matches\":[]}");
    ApiDictionary dict(mock);
    auto result = dict.lookup("nosuch");
    ASSERT_FALSE(result.has_value());
    EXPECT_EQ(result.error(), DictError::NotFound);
}

TEST(ApiDictionary, MalformedJsonMapsToNotFound) {
    auto mock = std::make_shared<MockNetworkClient>();
    mock->setBody("not json at all");
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
