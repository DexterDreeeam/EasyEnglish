# 0004. 重写：C++/Qt/ImGui → Rust + m2a 风格 cargo workspace

- **Status**: accepted
- **Date**: 2026-06-05
- **Supersedes**: [0001](./0001-choose-qt6.md), [0002](./0002-switch-to-imgui.md), [0003](./0003-tray-overlay.md)
  （0001-0003 描述的是 v0.3.0 之前的 C++ 实现链路。本 ADR 整体作废它们。
  原 C++ 实现保留在 git 历史的 `v0.3.0` tag，`git checkout v0.3.0` 可获取。）

## Context

到 v0.3.0 为止，EasyEnglish 是一个 C++23 / ImGui / GLFW + Win32 平台层的桌面应用。
单元测试 70/70 通过，安装包发到了 GitHub Releases。功能足够 ship。

但仍有若干结构问题难以在 C++ 实现里以低代价解决：

1. **跨平台一致性**：Win32 平台层用了 LowLevelMouseHook / Shell_NotifyIcon 等 API；
   macOS / Linux 等价物都要重写。Rust 生态有 `tray-icon` / `global-hotkey` / `winit` 等
   现成 crate，跨平台行为更接近免费。
2. **依赖管理**：vcpkg + WiX + Inno Setup + Qt-action 配合复杂；CI 一次首跑 20+ 分钟。
   cargo workspace + cargo-wix / cargo-packager 整体简单一个量级。
3. **模块化文档**：仓库目录有清晰的 `core/app/ui/platform` 边界，但**没有强制的
   per-module 接口文档**——任何模块的公共 API 需要从代码里翻 `.hpp` 来确认。
4. **测试速度**：CMake + 链接 + AddressSanitizer 跑全套测试 ~7s，再加上启动开销
   全跑要 30 s+；Rust + nextest 同等规模预计 <10 s。

用户 2026-06-05 提出的明确要求：
- Rust 重写
- 模块化（cargo workspace）
- 快速本地编译 + 测试
- 可发布安装包
- 学习 `C:\r\m2a` 的 design / interface 文档约定
- 顶层模块：`Dict` / `Core` / `Win` / `Mac` / `Linux`（至少 4 个）
- 当前只关注 `Dict` + `Core`
- 删除 favorites，新增 **Note**（runtime EN→任意内容映射）
- **不要 GitHub Actions / 自动 release**

## Considered options

1. **保留 C++，继续增量改进** — 多平台 hot-key + 模块接口文档化都做得到，但工作量与
   全量重写相当（~3000 LOC C++），而且仍然要面对 CMake + vcpkg 的体感。**否**。
2. **Rust + cargo workspace 全量重写，m2a 风格文档** ✅
   - 5 个顶层 crate（`Dict` / `Core` / `Win` / `Mac` / `Linux`）
   - 每个模块 `.design.md` + `.interface.md` + `tests/.test.md`
   - 当前只实现 Dict + Core；Win/Mac/Linux 留占位骨架
   - 不写 `.github/workflows/`
3. **Rust + 单 crate + mod** — 也可以，但 mod 不能强制接口边界，重构惩罚比 crate
   边界轻得多。**否**。
4. **Rust 重写 + 抄之前 plan.md 的 10 crate 拆分** — `ee-app` / `ee-ui` / `ee-bin`
   等过于细碎；当前阶段只关心数据 + 逻辑两层，平台 + UI 还没动工。**与本次反馈不符**。

## Decision

采用 **Option 2**。

具体目录布局、模块职责、技术栈见根 `.design.md`。本 ADR 只钉死方向。

## Consequences

- **正面**
  - cargo workspace + 增量编译让任何一次修改只重编 1-2 个 crate
  - `.design.md` + `.interface.md` 让 AI 与人都有显式契约可读
  - 顶层 5 crate 一开始就把"哪个平台做什么"钉在仓库里，避免后续填充时大改布局
- **代价 / 取舍**
  - 失去 v0.3.0 已经发布的安装包（用户需要旧版本，告诉他/她去 Releases `v0.3.0` 下载）
  - 失去 favorites 功能（用户明确要求；通过 Note 部分覆盖等价需求）
  - Note 是 runtime data，重启清空（用户明确要求；将来若改主意，加 persist 入口即可，
    `NoteStore` 接口不会因此破坏）
  - 没有 CI 意味着 push 到 main 之前必须本机跑 fmt + clippy + nextest；AGENTS.md §3 已写明
- **对接口的影响**
  - 所有 v0.3.0 时代的 C++ 类（`Database` / `IDictionary` / `MainView` / `AppState` …）
    都不复存在。Rust 等价物在各模块 `.interface.md` 里重新定义并冻结。
- **对测试的影响**
  - Rust 测试惯例是 `tests/` 子目录的集成测试 + `#[cfg(test)] mod tests` 单元测试。
    我们采用前者作主力，每个 crate 一个 `tests/.test.md`（仿 m2a `.test/.test.md`）
    列出每个测试目的，便于人和 AI 都看一眼就知道覆盖面。

## References

- m2a 仓库（设计/接口文档约定的源）：`C:\r\m2a`
- v0.3.0 release（重写起点的 reference 行为）：
  https://github.com/DexterDreeeam/EasyEnglish/releases/tag/v0.3.0
- 之前的 plan.md（被本 ADR 替代）：session 历史保留
