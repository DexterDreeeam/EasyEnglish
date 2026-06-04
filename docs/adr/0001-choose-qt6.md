# 0001. 选择 Qt 6 Widgets 作为 UI 框架

- **Status**: accepted
- **Date**: 2026-06-04
- **Iteration**: iter-000-skeleton

## Context

EasyEnglish 是 Windows 桌面端的单词查询工具，全程由 AI 协助编写。
UI 框架选择直接影响：

- AI 输出代码的稳定性（训练语料数量 + API 表面积）
- 自动化测试的可行性（特别是 UI 行为测试与快照对比）
- 后续跨平台扩展的成本
- 构建/依赖管理的复杂度

## Considered options

1. **Qt 6 Widgets (CMake + vcpkg)**
   - 优点：训练语料极多、文档系统化、信号槽天然支持依赖注入与 mock、
     原生 `QTest` 模块支持键鼠模拟与 `QSignalSpy`、`QWidget::grab()` 即截图
     便于快照测试、跨平台、与 GoogleTest 共存良好。
   - 缺点：二进制体积大、外观非"现代 Fluent"风格。
2. **WinUI 3 / WinAppSDK**
   - 优点：微软官方现代 UI、原生 Windows 外观。
   - 缺点：AI 训练语料相对稀少、API 仍在演进（破坏性变更风险）、
     测试自动化主要依赖 WinAppDriver（外部进程）、CMake/vcpkg 集成不顺畅。
3. **Dear ImGui + DirectX / GLFW**
   - 优点：极简、即时模式、单文件 fork 友好、AI 输出准确率高。
   - 缺点：非系统原生控件（IME 中文输入体验欠佳）、可访问性弱、
     业务型应用打磨成本高。
4. **wxWidgets**
   - 优点：原生控件外观。
   - 缺点：AI 训练语料 / 文档不如 Qt 系统化，社区活跃度低。
5. **原生 Win32 + 控件**
   - 优点：最轻、依赖最少。
   - 缺点：UI 自动化测试需自建（或上 WinAppDriver），开发与测试成本最高。

## Decision

选用 **Qt 6 Widgets**。开发与 CI 使用 Qt 官方预编译二进制（通过 `jurplel/install-qt-action` 在
CI 上自动安装，本地通过 Qt online installer 或 aqtinstall）；其余轻量依赖（sqlite3 / gtest /
benchmark / fmt）由 vcpkg manifest 管理。
UI 测试统一使用 `Qt6::Test` + `QSignalSpy` + `QWidget::grab()` 快照。

> 备注：原方案计划用 vcpkg 拉 `qtbase[widgets]`，但 vcpkg 从源码编译 Qt 在 CI 上耗时
> 1–3 小时，与每 PR 反馈周期不符。改用官方二进制后单次 CI 安装 Qt < 2 分钟。

## Consequences

- 正面：
  - AI 写 UI 代码的"幻觉率"最低，单 PR 通过率最高。
  - UI 行为测试与快照测试同进程、无外部驱动，CI 时间最短。
  - core 层只允许 `Qt6::Core`，杜绝 UI 渗入业务逻辑（由
    `tools/check_core_no_ui.py` 强制）。
- 代价：
  - 安装包体积约 30–50 MB（可接受）。
  - 外观非 Windows 11 Fluent 风格 — 必要时用 QSS 自定义。
- 对契约的影响：`docs/contracts/ui-mainwindow.md` 锁定 Qt 类型。
- 对测试的影响：`tests/ui/` 强制使用 `Qt6::Test`。

## References

- Qt 6 Test framework: https://doc.qt.io/qt-6/qttest-index.html
- vcpkg `qtbase` port: https://github.com/microsoft/vcpkg/tree/master/ports/qtbase
- 项目计划详见 session plan.md §1。
