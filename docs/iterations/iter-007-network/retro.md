# iter-007 Retrospective

- **What shipped**: `INetworkClient` 抽象 + `QtNetworkClient`（QNetworkAccessManager + QEventLoop blocking）+ `ApiDictionary`（dictionaryapi.dev JSON 解析），9 个单测全部走 `MockNetworkClient`，CI 完全离线。

- **AI走偏**: 第一版想把 fetch 写成 callable 参数（避免接口），但接口抽象更便于 mock 与未来加 `CompositeDictionary`。坚持用 `INetworkClient`。同时 `setBaseUrl` 没强制 trailing slash 时容易凑出 `https://example.testapple` —— 加了一行自动补 `/` 兜底，并在测试里覆盖。

- **下次保留**：每个 IO 边界（DB、network）都先把抽象写出，concrete impl 与 mock 一起进同一个迭代；tests 永远只 link mock，避免 CI 触网。
