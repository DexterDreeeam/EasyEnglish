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
    virtual auto get(const QString& url) const
        -> std::expected<QByteArray, NetworkError> = 0;
};

class QtNetworkClient final : public INetworkClient {
public:
    explicit QtNetworkClient(int timeout_ms = 5000) noexcept;
    auto get(const QString& url) const -> std::expected<QByteArray, NetworkError> override;
};

}
```

## 2. Invariants

- `get()` is **synchronous** — it blocks until response, timeout, or error.
- Safe to call from any thread (each call constructs its own `QNetworkAccessManager`).
- Returns `Timeout` if no response arrives within the configured ms budget.
- 2xx responses return body bytes; non-2xx returns `HttpError`.
- DNS / connection failures return `Offline`.

## 3. Dependencies

- Allowed: `Qt6::Core`, `Qt6::Network`
- Forbidden: Qt UI, blocking on UI-owned event loops other than the local one
  spawned by `QtNetworkClient::get()`.

## 4. Test fixtures

- Tests use a hand-rolled `MockNetworkClient` (test-only) that returns canned
  bytes/errors. **No real network access in CI** — that would make tests flaky.

## 5. Change log

- 2026-06-04 — iter-007: initial implementation + frozen. `QtNetworkClient`
  wraps `QNetworkAccessManager.get` with a `QEventLoop` + `QTimer` for sync
  semantics. `ApiDictionary` (in dictionary module) uses this to hit
  dictionaryapi.dev.
