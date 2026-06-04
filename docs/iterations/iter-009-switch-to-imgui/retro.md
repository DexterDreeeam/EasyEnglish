# iter-009 Retrospective

- **What shipped**: 全栈把 Qt 6 Widgets 换成 Dear ImGui + GLFW + OpenGL3。
  Core 层彻底脱 Qt（用 nlohmann/json 替 QJsonDocument、std::string 替 QString）；
  新增 src/app/AppState（纯 C++ presentation model）+ src/ui/MainView（无状态 ImGui 渲染）。
  Network 端 QtNetworkClient → HttpNetworkClient（cpp-httplib[openssl]）。
  71/71 tests pass on CI（[#26939140851](https://github.com/DexterDreeeam/EasyEnglish/actions/runs/26939140851)，
  包含 14 个新的 AppState 模型测试 + 全部 core 单测原样保留）。

- **AI 走偏 / 学到的坑（5 处，都已修复）**
  1. **Botched edit**：批量 `edit(old_str → new_str)` 漏掉了 SqliteDictionary.cpp 后半段
     原有的辅助函数副本，编译时出现 `'storage': is not a class or namespace name`
     的诡异错误。教训：跨大段的改写直接 `Remove-Item` + `create` 覆盖比一连串 `edit` 安全。
  2. **`ImGuiChildFlags_Border` undeclared**：1.90 才加，vcpkg 的 pinned 版本更旧。
     回退到长期稳定的 `BeginChild(name, size, bool border)` 重载。
  3. **MSVC `getenv` /W4 当 error**：`std::getenv` 触发安全 CRT 弃用警告。
     `#define _CRT_SECURE_NO_WARNINGS` 在 main.cpp 顶部即可，比 `_dupenv_s` 干净。
  4. **WIN32 子系统找不到 WinMain**：`add_executable(... WIN32)` 默认要 WinMain；
     ImGui/GLFW 用 `int main()`。CMake `target_link_options(... /ENTRY:mainCRTStartup)`
     保留 WINDOWS 子系统但走标准 main 入口。
  5. **`opengl32.lib` 没链**：ImGui 的 GL3 backend 自己处理高版本 GL，但 main.cpp 直接
     用了 `glClear / glClearColor / glViewport`（GL 1.1）。Windows 必须显式 link `opengl32`。

- **AI 协作框架的实际价值（这一轮被反复验证）**
  - 契约里"core 不依赖 UI"的硬约束在切框架时是救命的：core 模块除了 Qt 类型替换
    几乎没碰 —— 业务逻辑直接复用，所有核心单测原样保留。
  - 守卫脚本 `tools/check_core_no_ui.py` 同步加了 ImGui / GLFW 检测，确保下次有人想
    在 core 偷偷 include `<imgui.h>` 会立刻被 CI 拦下。
  - 任务卡 + retro + ADR-0002 形成完整审计链：以后回看为什么 0.2.0 不用 Qt，3 分钟读完。

- **下次保留 / 改变**
  - 保留：每次大改写 = ADR + iter 卡 + 契约升级三件套同步进行。
  - 改变：vcpkg 首次构建 openssl + glfw + imgui ≈ 20 分钟（首轮 CI 21m31s）。
    后续可以考虑：① CI 中预热 vcpkg binary cache，② 把 cpp-httplib openssl 改 boringssl，
    或 ③ 暂时砍掉 HTTPS 让 ApiDictionary 在 CI 里只走 mock。本轮先付了"首次构建"的学费。
