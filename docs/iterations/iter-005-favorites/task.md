# Task: iter-005-favorites — favorites module + UI star button

## Context

- 涉及模块: `src/core/favorites/`, `src/ui/MainWindow`, `tests/unit/favorites/`, `tests/ui/`
- 相关契约: 新增 `docs/contracts/favorites.md`；扩展 `docs/contracts/ui-mainwindow.md`
- 依赖前置任务: `iter-004-history` ✅

## Out of scope

- 不实现模糊匹配（iter-006）

## Acceptance criteria

- [ ] `FavoritesStore::add/remove/contains/list`，幂等且大小写不敏感
- [ ] UI 星标按钮切换收藏；侧栏 History/Favorites 双 tab
- [ ] CI 全绿

## Definition of Done

- 契约 favorites.md frozen
- ui-mainwindow.md change log 更新
- retro 写完
