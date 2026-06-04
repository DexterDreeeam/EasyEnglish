# `network` Contract

**Source path**: `src/core/network/`
**Owner test path**: `tests/unit/network/`
**Status**: frozen (since iter-007)

## 1. Public API (FROZEN — change requires ADR)

```cpp
namespace easyenglish::core::network {

enum class NetworkError {
    Timeout,
    HttpError,
    InvalidResponse,
    Offline,
};

class INetworkClient {
public:
    virtual ~INetworkClient() = default;
    virtual auto get(const std::string& url) const
        -> std::expected<std::string, NetworkError> = 0;
};

class HttpNetworkClient final : public INetworkClient {
public:
    explicit HttpNetworkClient(int timeout_ms = 5000) noexcept;
    auto get(const std::string& url) const -> std::expected<std::string, NetworkError> override;
};

}
```

## 2. Invariants

- `get()` is **synchronous** — blocks until response / timeout / error.
- Safe to call from any thread (each call constructs its own `httplib::Client`).
- `Timeout` if no response arrives in budget. 2xx → body bytes. Non-2xx → `HttpError`.
  DNS / connection failures → `Offline`.

## 3. Dependencies

- Allowed: standard library, `cpp-httplib` (vcpkg, header-only), `openssl` (HTTPS).
- Forbidden: Qt, ImGui, any UI library.

## 4. Test fixtures

- Tests use a hand-rolled `MockNetworkClient` (test-only) that returns canned
  bytes/errors. **No real network access in CI** — that would make tests flaky.

## 5. Change log

- 2026-06-04 — iter-007: initial implementation as `QtNetworkClient`
  (QNetworkAccessManager + QEventLoop).
- 2026-06-04 — iter-009: replaced with `HttpNetworkClient` (cpp-httplib with
  OpenSSL feature for HTTPS). INetworkClient signature changed from
  `(const QString&) -> std::expected<QByteArray, …>` to
  `(const std::string&) -> std::expected<std::string, …>`. See ADR-0002.
