# Task: iter-011-tray-overlay — 全面改造为托盘悬浮翻译器

## Context

- **涉及模块**: `tools/seed_db.py`, `src/core/dictionary/*`, `src/core/network/*`,
  `src/app/AppState.*`, `src/ui/MainView.*`, `src/main.cpp`,
  **新增** `src/platform/win32/*`、`assets/fonts/`、`installer/*`、`tests/*`
- **ADR**: `docs/adr/0003-tray-overlay.md`

## Out of scope

- 设置面板 / 历史 UI（保留 core 模块，但 UI 不暴露；下一轮通过托盘菜单接入）
- 非 Windows 平台（只提供 stub `IPlatformShell` 实现，让 CI 通过）

## Acceptance criteria

- [ ] `tools/seed_db.py` 输出英→中本地 fixture（~80 词）
- [ ] `SqliteDictionary` 测试 / golden 全部中文化并通过
- [ ] `ApiDictionary` 走 MyMemory，9 个单测全过（仍 mock）
- [ ] `IPlatformShell` 接口 + Win32 实现：托盘 / 全局快捷键 / 显示隐藏 / 单实例
- [ ] `MainView` 改写：无 title bar、单行输入、下拉中文翻译、Esc 关闭
- [ ] main.cpp 启动即隐藏；只在快捷键触发时 show；Esc → hide + 恢复前台
- [ ] CJK 字体 fallback：本地无字体不崩，运行时打印 warning；CI release 走完整字体
- [ ] CI 全绿 + Release v0.3.0 可下载且能在 Win10/11 启动并响应快捷键

## Implementation hints

- vcpkg 加 `nlohmann-json`（已有）+ `cpp-httplib`（已有）；不需要新依赖。
- Win32：`Shell_NotifyIconW` + `WH_MOUSE_LL` + `RegisterClassExW` 隐藏窗口。
- 在 GLFW window 上设置 `GLFW_FLOATING=true`, `GLFW_DECORATED=false`,
  `GLFW_RESIZABLE=false`, `GLFW_VISIBLE=false`（按需 show）。
- 焦点恢复：进 show 前 `GetForegroundWindow()` 缓存；hide 后 `SetForegroundWindow(prev)`。
- 字体 fallback：把 `assets/fonts/NotoSans-Regular.ttf` + `NotoSansSC-Regular.otf`
  通过 `ImFontConfig::MergeMode` 合并；缺失则不加载并打印 stderr 警告。

## Definition of Done

- 契约 platform.md 新建 frozen；dictionary.md / ui-mainview.md change log 更新
- retro 写
- v0.3.0 release 资产上线
