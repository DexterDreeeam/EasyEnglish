# iter-004 Retrospective

- **What shipped**: `src/core/history/HistoryStore` (SQLite UPSERT, case-insensitive PK, schema auto-create), `Database::createOrOpen` factory, MainWindow right-side QListWidget. 10 unit tests + 3 new UI tests.

- **AI走偏**: 早期想给 `resultReady` signal 传 `Entry` 值——会强制 `Q_DECLARE_METATYPE` 才能被 QSignalSpy 跨连接安全使用。改成传 `QString headword`，下游需要细节自己 lookup，零元类型注册成本。同时把 history 入口直接做进 MainWindow（不抽 app 层）是为了控范围，等 favorites 时如果重复模式再抽。

- **下次保留**：契约（`docs/contracts/history.md`）在写代码前先列接口、不变量与错误码 —— 实现写到一半就发现需要 `kDefaultRecent` 与 `recent(0)` 语义，预先约定避免临时拍脑袋。
