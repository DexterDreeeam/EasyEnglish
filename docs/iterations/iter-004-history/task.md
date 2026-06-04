# Task: iter-004-history — query history module + UI integration

## Context

- **涉及模块**: `src/core/history/`, `src/core/storage/Database` (扩展), `src/ui/MainWindow`, `tests/unit/history/`, `tests/ui/`
- **相关契约**: 新增 `docs/contracts/history.md`；扩展 `docs/contracts/storage.md`
- **依赖前置任务**: `iter-003-ui-mainwindow` ✅

## Out of scope

- 不实现收藏（iter-005）
- 不实现模糊匹配（iter-006）
- 不做导出/清除 UI（仅核心 API 提供 `clear()`，UI 由后续迭代决定）

## Acceptance criteria

- [ ] `Database::createOrOpen(path)` 新工厂方法（与 `open` 共存）
- [ ] `HistoryStore::record(word)` upsert：新词 count=1，已存在 count+1 + 更新 last_at
- [ ] `HistoryStore::recent(N)` 按 last_at desc 返回最多 N 条
- [ ] `HistoryStore::clear()` 清空
- [ ] 大小写不敏感（COLLATE NOCASE on headword）
- [ ] 单测：upsert / 排序 / 上限 / clear / 大小写
- [ ] UI：右侧 QListWidget 列出最近 20 条；resultReady 触发刷新；点击侧栏项重发搜索
- [ ] CI 全绿
- [ ] 契约文档：storage 改 v2 加 createOrOpen；history 新建并 frozen

## Implementation hints

- 用 `INSERT ... ON CONFLICT(headword) DO UPDATE SET ...` （SQLite UPSERT）
- last_at 用 `std::chrono::system_clock::now()` 转 unix 秒（int64）
- UI 侧用 `QHBoxLayout` 分左右：左是 result_view，右是 history_list

## Definition of Done

- 新增 contracts/history.md（frozen）
- 更新 contracts/storage.md change log
- 写 retro
