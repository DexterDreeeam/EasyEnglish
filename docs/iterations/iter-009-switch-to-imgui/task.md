# Task: iter-009-switch-to-imgui — 全栈把 Qt 换成 Dear ImGui

## Context (必读)

- **涉及模块**: 全部（build / core / app / ui / tests / CI / docs）
- **相关 ADR**: `docs/adr/0002-switch-to-imgui.md`（supersedes 0001）
- **依赖前置任务**: iter-000 ~ iter-008 + quality-gates ✅

## Out of scope

- 不重新设计核心数据模型（storage / dictionary / history / favorites 接口语义不变）
- 不引入 Vulkan / DirectX backend（OpenGL3 + GLFW 一种就够）
- 不做 docking / 多窗口
- 不重做 i18n（ImGui 中文渲染需要 font 配置，留下一轮）

## Acceptance criteria

- [ ] vcpkg.json：移除 Qt 相关；加入 `imgui[glfw-binding,opengl3-binding]` / `glfw3` /
      `nlohmann-json` / `cpp-httplib`
- [ ] CMakeLists：移除 `find_package(Qt6 ...)` 与所有 Qt 链接；新增 ImGui/GLFW/Json/httplib
- [ ] CI 工作流：移除 `jurplel/install-qt-action`；总时间 < 6 分钟
- [ ] `src/core/**` 不再依赖任何 Qt 头（`tools/check_core_no_ui.py` 仍通过；且 grep `Q[A-Z]` 也空）
- [ ] `src/app/AppState`：pure C++ model 持有 dict/history/favorites/online；方法
      `submitSearch(word)` / `toggleFavorite()` / `activateHistoryAt(i)` 等
- [ ] `src/ui/MainView::render(AppState&)`：单函数渲染（搜索框 / 结果 / 历史侧栏 / 收藏 tab / 星按钮）
- [ ] `src/main.cpp`：GLFW + ImGui-GL3 主循环
- [ ] tests/app/test_app_state.cpp：覆盖原 UI 行为（emit / hit / miss / favorite toggle / history append）
- [ ] 所有核心单测（storage/dictionary/history/favorites/network）仍通过
- [ ] `python tools/check_core_no_ui.py` 通过
- [ ] CI 全绿

## Implementation hints

- ImGui vcpkg 名：`imgui`；feature list 选 `glfw-binding,opengl3-binding`
- 自带的 GL3 loader (imgui_impl_opengl3_loader.h) 够用，不需要额外 glew/glad
- 用 `cpp-httplib` 时注意它是 header-only：vcpkg target `httplib::httplib`
- nlohmann_json：`find_package(nlohmann_json CONFIG REQUIRED)`，target `nlohmann_json::nlohmann_json`
- 删 src/ui/MainWindow.{hpp,cpp}、tests/ui/*.cpp、src/core/network/QtNetworkClient.{hpp,cpp}
- 留 src/app/placeholder.cpp 被真实的 AppState.cpp 替换；同时 src/app 链接 core
- ImGui 主循环里别忘了 `glfwTerminate()`；用 RAII 包一下更稳

## Definition of Done

- ADR-0002 已存在并把 0001 标 superseded
- 所有契约的 "FROZEN — change requires ADR" 行都已用本 ADR 解释
- 写 retro，列出迁移过程踩到的实际坑
