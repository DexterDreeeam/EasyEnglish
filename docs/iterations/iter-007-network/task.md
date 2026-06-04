# Task: iter-007-network — online dictionary client (interface + Mock + real)

## Context

- 涉及模块: `src/core/network/`, `src/core/dictionary/ApiDictionary*`, `tests/unit/network/`
- 相关契约: 新增 `docs/contracts/network.md`；扩展 dictionary 契约 change log

## Acceptance criteria

- [ ] `INetworkClient::get(url)` 同步 HTTP GET，超时/失败映射到 NetworkError
- [ ] `QtNetworkClient` 用 QNetworkAccessManager + QEventLoop + QTimer 实现
- [ ] `ApiDictionary` 实现 `IDictionary`，解析 dictionaryapi.dev 响应
- [ ] 测试只用 MockNetworkClient，零真实网络
- [ ] CI 全绿

## Out of scope

- UI 端"在线/本地结果合并"暂不接（可后续追加 CompositeDictionary）

## Definition of Done

- contracts/network.md frozen
- retro 写完
