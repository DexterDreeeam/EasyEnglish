# EasyEnglish

> AI 协作开发的 Windows **托盘式英→中即时翻译器** — C++23 / Dear ImGui + GLFW + OpenGL3 / CMake / vcpkg

后台运行，托盘常驻；按 **Ctrl + Shift + 鼠标滚轮向上** 唤起一个无装饰的悬浮输入框，
输入英文 → 立刻显示中文翻译（本地词典优先、在线作 fallback）；Esc 关闭并把焦点
还给之前的窗口。

本项目的特别之处在于：**全程由 AI 协助编写**。仓库里大量的"看似与功能无关"的文件
（`AGENTS.md`、`docs/contracts/`、`docs/adr/`、`docs/prompts/`、各种 CI 守卫脚本）
都是为了让 AI 在跨会话、跨模块、跨贡献者时仍然能产出一致、可验证、可回滚的代码。

> 早期形态参见 ADR-0001（Qt 6 Widgets · 已废弃）→ ADR-0002（切 ImGui · 已实现）→
> ADR-0003（重塑为托盘悬浮翻译器 · 当前形态）。

## 下载安装（普通用户）

去 [GitHub Releases 页面](https://github.com/DexterDreeeam/EasyEnglish/releases) 下载
最新的 `EasyEnglishSetup-<version>.exe`，双击运行即可。安装时可勾选"开机启动"。
安装后程序在后台运行，按 **Ctrl + Shift + 鼠标滚轮向上** 唤出输入框：

- 输入单词 → 下拉显示候选翻译
- Enter 或点击某条翻译 → 关闭窗口
- Esc → 关闭窗口并把焦点还给之前的窗口
- 右键托盘图标 → "Show overlay" / "Quit"

首次启动会在 `%APPDATA%\EasyEnglish\` 下自动创建 `history.sqlite` 与 `favorites.sqlite`。

> 安装包附带 SHA256（同目录 `.sha256` 文件），可用 PowerShell 校验：
> `Get-FileHash .\EasyEnglishSetup-<ver>.exe -Algorithm SHA256`

## 本地构建（开发者）

前置：Visual Studio 2022（Desktop C++ + Windows SDK）+ vcpkg + Python 3.x。

```powershell
$env:VCPKG_ROOT = "<path-to-vcpkg>"
cmake --preset msvc-debug
cmake --build --preset msvc-debug --parallel
ctest --preset msvc-debug --output-on-failure
python tools\check_core_no_ui.py
```

vcpkg 会自动拉取：sqlite3 / fmt / nlohmann-json / cpp-httplib[openssl] / glfw3 /
imgui[glfw-binding,opengl3-binding] / gtest / benchmark。

> 字体（Noto Sans + Noto Sans SC）默认不在仓库里；本地运行会 fallback 到 ImGui
> 默认字体（中文方框）。Release 流水线会自动下载并打包到安装器。

## 顶层布局

```
src/core/      纯逻辑层（不依赖任何 UI / 平台库）
src/app/       AppState：pure-C++ presentation model
src/ui/        MainView：ImGui 渲染函数
src/platform/  IPlatformShell + Win32 实现（托盘 / 全局快捷键 / 焦点）
tests/         unit + app + benchmarks
docs/          contracts, adr, prompts, iterations
tools/         CI 守卫脚本与开发辅助
installer/     Inno Setup 打包脚本
assets/fonts/  Release 流水线下载的字体（gitignore）
```

## 打包安装程序（开发者本地）

```powershell
cmake --preset msvc-release
cmake --build --preset msvc-release --parallel
# 需要预装 Inno Setup 6（iscc.exe 在 PATH 或 Program Files 默认路径）
# 想中文渲染：先把 NotoSans-Regular.ttf + NotoSansSC-Regular.otf 放进 assets\fonts\
pwsh tools\build_installer.ps1
# 输出: installer\dist\EasyEnglishSetup-0.3.0.exe
```

详细职责见 [`docs/architecture.md`](./docs/architecture.md)。

## 许可证

MIT — 见 `LICENSE`。打包的字体遵循 SIL OFL 1.1（Noto 字体）。
