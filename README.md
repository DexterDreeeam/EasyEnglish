# EasyEnglish

> AI 协作开发的 Windows 单词查询桌面应用 — C++23 / **Dear ImGui + GLFW + OpenGL3** / CMake / vcpkg

本项目的特别之处在于：**全程由 AI 协助编写**。仓库里大量的"看似与功能无关"的文件
（`AGENTS.md`、`docs/contracts/`、`docs/adr/`、`docs/prompts/`、各种 CI 守卫脚本）
都是为了让 AI 在跨会话、跨模块、跨贡献者时仍然能产出一致、可验证、可回滚的代码。

> 早期（iter-000~008）使用 Qt 6 Widgets；iter-009 全栈切换到 Dear ImGui，
> 参见 [ADR-0002](./docs/adr/0002-switch-to-imgui.md)。

## 如何参与（人类 / AI 都适用）

1. **先读** [`AGENTS.md`](./AGENTS.md) — 这是仓库的"宪法"。
2. **再读** [`docs/contracts/`](./docs/contracts/) 中你要改动的模块契约。
3. **写任务卡**：复制 [`docs/prompts/feature-task.md`](./docs/prompts/feature-task.md) 到
   `docs/iterations/iter-NNN-<slug>/task.md`，填好 acceptance criteria。
4. **AI 实现 + 自检**：按 `AGENTS.md` 末尾的 self-check 清单逐项回答。
5. **PR**：模板会要求你贴出本地测试结果与影响范围。

完整开发方法见 [`docs/plan.md`](./docs/plan.md)。

## 本地构建

前置：Visual Studio 2022（Desktop C++ + Windows SDK）+ vcpkg + Python 3.x。
**不再需要 Qt SDK**（之前 iter-000~008 的 Qt 依赖已被 ImGui 取代）。

```powershell
$env:VCPKG_ROOT = "<path-to-vcpkg>"
cmake --preset msvc-debug
cmake --build --preset msvc-debug --parallel
ctest --preset msvc-debug --output-on-failure
python tools\check_core_no_ui.py
```

vcpkg 会自动拉取：sqlite3 / fmt / nlohmann-json / cpp-httplib[openssl] / glfw3 /
imgui[glfw-binding,opengl3-binding] / gtest / benchmark。

## 顶层布局

```
src/core/   纯逻辑层（不依赖 ImGui/GLFW/Qt 任何 UI 库）
src/app/    AppState：pure-C++ presentation model
src/ui/     MainView：ImGui 渲染函数
tests/      unit + app + benchmarks
docs/       contracts, adr, prompts, iterations
tools/      CI 守卫脚本与开发辅助
installer/  Inno Setup 打包脚本
```

## 打包安装程序（Windows）

```powershell
cmake --preset msvc-release
cmake --build --preset msvc-release --parallel
# 需要预装 Inno Setup 6（iscc.exe 在 PATH 或 Program Files 默认路径）
pwsh tools\build_installer.ps1
# 输出: installer\dist\EasyEnglishSetup-0.2.0.exe
```

详细职责见 [`docs/architecture.md`](./docs/architecture.md)。

## 许可证

MIT — 见 `LICENSE`。
