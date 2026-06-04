# Task: iter-000-skeleton — 建立最小可构建可测试脚手架

## Context (必读)

- **涉及模块**: 根目录构建系统、`src/{core,app,ui}/`、`tests/{unit,ui,benchmarks}/`、CI
- **相关契约**: `docs/contracts/ui-mainwindow.md`（仅占位声明，正式冻结在 iter-003）
- **相关 ADR**: `docs/adr/0001-choose-qt6.md`
- **依赖前置任务**: `bootstrap-repo`, `governance-docs`

## Out of scope

- 不实现任何真实业务逻辑（查询/历史/收藏全部留给后续迭代）
- 不引入 sqlite/网络代码（留给 iter-001 / iter-007）
- 不实现完整的 UI 主窗口（iter-003 才做）

## Acceptance criteria (机器可验证)

- [ ] `cmake --preset msvc-debug` 配置成功
- [ ] `cmake --build --preset msvc-debug --parallel` 构建成功
- [ ] `ctest --preset msvc-debug` 通过（smoke 测试 + MainWindowSmoke）
- [ ] `python tools/check_core_no_ui.py` 通过
- [ ] 双击 `EasyEnglish.exe` 弹出标题为 "EasyEnglish" 的空主窗口
- [ ] GitHub Actions `windows-build` job 在 PR 上跑绿

## Implementation hints

- vcpkg 取系统 `$env:VCPKG_ROOT`；CI 用 `lukka/run-vcpkg`。
- Qt 部署：开发期靠 `Qt6_DIR` 自动发现；安装期用 `windeployqt`（iter-008 再做）。
- 不要把任何真实查询逻辑塞进占位文件 — placeholder 就是占位。

## Definition of Done

- 仓库可以被任何人 clone 后按 README 步骤一次跑通
- `AGENTS.md`、契约模板、prompt 模板、ADR-0001 全部就位
- 在 `retro.md` 写 1–3 行总结
