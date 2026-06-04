# Task: iter-003-ui-mainwindow — search box + result panel

## Context

- **涉及模块**: `src/ui/MainWindow.{hpp,cpp}`, `src/main.cpp`, `tests/ui/`
- **相关契约**: `docs/contracts/ui-mainwindow.md`（本迭代收尾时 frozen）
- **依赖前置任务**: `iter-002-dictionary-exact` ✅

## Out of scope

- 不实现历史 / 收藏（iter-004 / iter-005）
- 不实现模糊匹配 UI（iter-006）
- 不实现 i18n 多语言切换

## Acceptance criteria

- [ ] MainWindow ctor 接受 `std::shared_ptr<IDictionary>` 注入（满足契约）
- [ ] 输入框 + 搜索按钮 + 结果显示面板 + 状态行
- [ ] 按 Enter / 点击按钮触发 `searchRequested(QString)` 信号 + 调用 dict->lookup
- [ ] 命中：结果面板显示 headword / phonetic / definitions
- [ ] 未命中：状态行显示 "Not found"，结果面板清空
- [ ] 空输入：搜索按钮禁用
- [ ] main.cpp 接通 SqliteDictionary + 用户数据目录中的 mini_dict.sqlite
- [ ] QTest: emitsSignalOnEnter / displaysEntryOnSuccess / showsNotFoundOnMiss / buttonDisabledOnEmpty 全过
- [ ] CI 全绿

## Implementation hints

- 用 `QSignalSpy` 验证 `searchRequested` 发射
- UI 测试构造一个最小的 `IDictionary` 测试替身（手写假实现，避免 GMock 复杂度）
- 信号 `resultReady(Entry)` 在 onSearch 内成功路径发射，便于测试断言

## Definition of Done

- 契约 `docs/contracts/ui-mainwindow.md` 状态 frozen
- retro 写完
