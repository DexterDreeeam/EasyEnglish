# Task: iter-008-package — end-to-end test + Inno Setup installer

## Context

- 涉及模块: `tests/ui/test_e2e_main_flow.cpp`, `installer/EasyEnglish.iss`, `tools/build_installer.ps1`
- 相关契约: 无新契约（无新公共 API）

## Acceptance criteria

- [ ] 一个 E2E QTest 把 SqliteDictionary(real) + HistoryStore(in-memory) + FavoritesStore(in-memory) + MainWindow 串起来，模拟用户搜索 + 收藏 + 检查 history
- [ ] 提交 Inno Setup `.iss` 脚本 + `tools/build_installer.ps1`
- [ ] CI 全绿（E2E 加入测试集）

## Out of scope

- 不在 CI 跑 Inno Setup（runner 没装；本地 PowerShell 可用）
- 不接 WinAppDriver（额外维护成本暂不引入）

## Definition of Done

- README 更新如何打包
- retro
