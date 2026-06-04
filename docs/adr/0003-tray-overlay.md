# 0003. 产品定型：托盘 + 全局快捷键 + 无装饰悬浮窗 + 英译中

- **Status**: accepted
- **Date**: 2026-06-04
- **Iteration**: iter-011-tray-overlay

## Context

到 iter-009 / 010 为止，EasyEnglish 是一个常驻主窗口、提供"输入→查英文释义/历史/收藏"的桌面应用，
搜索后端是本地 SQLite（英文释义）+ dictionaryapi.dev 备份。

用户反馈现实使用场景与上面假设完全不同：

- 真正的使用习惯是 **临时看一个英文单词的中文意思**，而不是常驻浏览。
- 不需要标题栏、菜单、侧栏，**只要一个单行输入框**。
- 应该 **常驻系统托盘、按全局快捷键 (Ctrl + Shift + 鼠标滚轮上) 唤起**；
  Esc 关闭，焦点回到先前的窗口（不能打断打字流）。
- 字体要清晰，**中文必须能渲染**，由安装包附带字体。
- 词典本质是 **英 → 中**：本地 fixture 给中文释义；在线 backup 也要中文（dictionaryapi.dev 仅英文）。

## Considered options

1. **维持当前 main-window UI**：与用户实际工作流不符，本次拒绝。
2. **Win32 平台层 + GLFW 无装饰悬浮窗 + ImGui** ✅
   - 用 `Shell_NotifyIconW` 在托盘放图标，message-only 窗口处理点击/菜单。
   - 全局监听 Ctrl+Shift+WheelUp：用 `SetWindowsHookEx(WH_MOUSE_LL)` 低级钩子
     （`RegisterHotKey` 不支持鼠标滚轮组合）。
   - GLFW 无装饰、置顶、按需 show / hide。Esc → hide 并 `SetForegroundWindow(prev_hwnd)`
     恢复之前抓到的前台窗口。
   - 单实例：命名互斥锁 `Local\EasyEnglish-SingleInstance` 防止多开。
3. **完全用 Qt 6 重做**：会得到更易接入系统集成的 API（QSystemTrayIcon 等），
   但 ADR-0002 刚切到 ImGui，重新接 Qt 不值得。
4. **第三方 launcher**（PowerToys Run 风格）：依赖外部框架，本次不引入。

## Decision

采用 **Option 2**：保留 ImGui，把平台依赖（托盘 / 钩子 / 焦点 / 单实例）下沉到
新的 `src/platform/win32/` 模块，通过纯虚接口 `IPlatformShell` 暴露给 `app::AppState`，
非 Windows 平台或单测可以用 fake shell。

产品形态变化：
- 启动 → 后台进程 + 托盘图标，无窗口。
- 全局热键 Ctrl + Shift + 鼠标滚轮上 → 悬浮窗弹出于鼠标所在显示器中心、自动 focus 输入框。
- 输入 → 立即查询本地英→中字典；下方下拉显示中文释义候选；选中 / Enter / 失焦关闭。
- 在线 backup 用 `https://api.mymemory.translated.net/get?q=<word>&langpair=en|zh-CN`
  （免 key 免费配额，公开服务）。
- Esc → 窗口隐藏 + 恢复先前前台窗口。
- 安装包附带 Noto Sans + Noto Sans SC 子集，运行时由 ImGui 加载（Latin + CJK merge）。

## Consequences

- **正面**
  - 使用心智模型与 PowerToys Run / 系统输入法候选窗一致，零学习成本。
  - core 层不变，AppState 只需新增/调整少量 fields（下拉候选 + 显示/隐藏请求）。
- **代价**
  - 仓库首次出现 Windows-specific 代码（`src/platform/win32`）；非 Win 平台用 stub。
  - 字体文件给 release 流水线增加一步下载，但 git 仓库不直接持有字节文件（gitignore + CI fetch）。
  - dictionaryapi.dev → MyMemory 后，单测覆盖的 JSON shape 完全不同；老的 golden / api 测试需要换。
  - history / favorites 模块功能上仍存在，但 UI 默认不显示（按"小输入框"原则）；
    iter-012 可考虑给托盘菜单加一个"打开历史"开关。
- **对契约的影响**
  - `docs/contracts/dictionary.md`：data semantics 改为 EN→CN（不改接口签名）
  - `docs/contracts/network.md`：无变化
  - `docs/contracts/ui-mainview.md`：重写为"frameless overlay"行为
  - 新增 `docs/contracts/platform.md`：`IPlatformShell` 接口冻结
- **对测试的影响**
  - `tests/app/test_app_state.cpp`：FakeDictionary 返回中文释义；新增 fakeShell 覆盖 show/hide
  - `tests/unit/dictionary/test_sqlite_dictionary.cpp`：fixture 中文化；golden 文件刷新
  - `tests/unit/network/test_api_dictionary.cpp`：换 MyMemory JSON
  - `tests/platform/`：iter-011 暂不写，platform 层 Win32 实现由集成验证（人工 / CI smoke）

## References

- Win32 tray API: `Shell_NotifyIconW`
- Win32 low-level mouse hook: `SetWindowsHookEx(WH_MOUSE_LL, ...)`
- GLFW frameless window: `glfwWindowHint(GLFW_DECORATED, GLFW_FALSE)`
- MyMemory translation API (free, no key): https://mymemory.translated.net/doc/spec.php
- Noto Sans + Noto Sans SC: https://github.com/notofonts/noto-fonts (SIL OFL 1.1)
