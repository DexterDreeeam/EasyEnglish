# 0002. 切换 UI 框架：Qt 6 Widgets → Dear ImGui

- **Status**: accepted (supersedes [0001](./0001-choose-qt6.md))
- **Date**: 2026-06-04
- **Iteration**: iter-009-switch-to-imgui

## Context

ADR-0001 选了 Qt 6 Widgets，理由是 AI 训练语料多 + 自带 QTest + 跨平台。
经过 iter-000 ~ iter-008 的实践，团队（即用户）希望切换到 **Dear ImGui**。
直接原因：

- 体积 / 依赖更轻：丢掉 Qt 整套（数十 MB），ImGui 是几个 `.cpp` + GLFW + OpenGL。
- 即时模式（immediate mode）UI 心智模型更适合 AI 生成：渲染就是几行 `if (Button(...))`，
  没有信号槽 / 元对象系统这一类需要 moc 处理的额外约束。
- 用户偏好（明确请求）。

## Considered options

1. **Dear ImGui + GLFW + OpenGL 3** ✅
   - 优点：UI 代码极简，渲染流是顺序代码；ImGui 库通过 vcpkg 安装秒级完成；
     无 moc / 无 AUTOMOC / 无 .ui 文件；可在 CI 不装 Qt SDK。
   - 缺点：非原生控件（中文 IME 支持需要额外配置）；无类似 QTest 的 UI 自动化框架；
     可访问性弱；自带主题不如系统原生。
2. **保留 Qt**：本次否决。
3. **WinUI 3 / wxWidgets / 原生 Win32**：未重新评估，本次不引入。

## Decision

切到 **Dear ImGui (docking 关闭) + GLFW + OpenGL3 backend**，
JSON 解析换 `nlohmann/json`（替代 `QJsonDocument`），
HTTP 客户端换 `cpp-httplib`（替代 `QNetworkAccessManager`）。
所有依赖通过 vcpkg manifest 管理，CI 不再需要 `install-qt-action`。

## Consequences

- **正面**
  - 单次 CI 时间预期降到 ~2-4 分钟（vcpkg 装这些库远比 Qt 二进制下载快）。
  - core 层彻底脱离 Qt（之前为了 QJsonDocument / QString 还间接依赖 Qt6::Core）。
  - main.cpp + MainView 总代码量预计减半。
- **代价**
  - 失去 QTest / QSignalSpy → UI 行为测试改为对 **AppState（pure C++ model）** 做 gtest。
    渲染层只做"渲染当前 state"，不可测但视觉简单。
  - 失去 QStandardPaths → 用 std::filesystem + 简单的 user-data 目录策略
    （Windows 上 `%APPDATA%/EasyEnglish`）。
  - QToolButton/QListWidget 等"开箱即用"控件 → 在 ImGui 中要自己画
    （Selectable / Button 等已经足够，但样式偏极简）。
- **对契约的影响（每份都要改）**
  - `docs/contracts/dictionary.md`: `QString` → `std::string`
  - `docs/contracts/network.md`: `INetworkClient::get(const QString&)` → `(const std::string&)`，
    返回 `std::expected<std::string, NetworkError>`
  - `docs/contracts/ui-mainwindow.md`: 完全重写——MainView 是函数 + AppState 是 model；
    不再有 signal/slot，事件通过 state mutation 表达
  - `docs/contracts/storage.md` / `history.md` / `favorites.md`: 无变化（之前就没用 Qt 类型）
- **对测试的影响**
  - 删除 `tests/ui/test_mainwindow_smoke.cpp`、`test_mainwindow_search.cpp`、`test_e2e_main_flow.cpp`
  - 新增 `tests/app/test_app_state.cpp` 覆盖等价行为
  - core 测试只需要把 `QByteArray::fromRawData` 之类调用换成 `std::string`，逻辑不变
- **对 AGENTS.md 的影响**
  - "src/core/** 可以依赖 Qt6::Core" 这一行删除
  - "ui/** 可以依赖 Qt6::Widgets" → "可以依赖 imgui + glfw3"

## References

- ImGui: https://github.com/ocornut/imgui
- vcpkg imgui 端口：feature `glfw-binding`, `opengl3-binding`
- nlohmann/json: https://github.com/nlohmann/json
- cpp-httplib: https://github.com/yhirose/cpp-httplib
- 这次重构的执行细节见 `docs/iterations/iter-009-switch-to-imgui/`
