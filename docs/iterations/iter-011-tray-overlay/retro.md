# iter-011 Retrospective

- **What shipped**: 把 EasyEnglish 从"开门红的窗口式英→英查询工具"重塑成
  **托盘常驻 + Ctrl+Shift+滚轮唤起 + 无装饰单行输入框 + 下拉中文翻译 + Esc 关闭并恢复焦点**
  的即时翻译器。本地词典换成 EN→CN（85 词种子），在线 backup 换成 MyMemory
  (`api.mymemory.translated.net`)；新增 `src/platform/win32/` 平台层封装托盘 /
  低级鼠标钩子 / 单实例锁 / 前台窗口救回；ImGui 重画窗体，CJK 字体在 release
  pipeline 自动下载并打入安装包；版本号 0.2.0 → 0.3.0。

- **AI 走偏 / 教训（待 CI 验证后补充编译/运行期的坑）**
  1. 一开始 `Remove-Item` + `create` 之间出现工具缓存延迟："文件已存在"假阳性 →
     先 `Get-ChildItem` 校验再 `create`，碰到则用 `edit` 覆盖。
  2. 把 `tests/ui/*` 整套删了的同时漏改 `tests/CMakeLists.txt` → CI 立刻挂。
     这次提前在同一提交里更新（见 diff）。
  3. ApiDictionary 的旧测试假设 dictionaryapi.dev 的 array-of-object shape；
     完全换成 MyMemory 的 object/responseData/matches 结构后，9 个 case 全部重写。

- **AI 协作框架再次显灵**
  - 之前的契约严格禁止 core 依赖 UI；这次新增 `src/platform/` 后，guard 脚本
    `check_core_no_ui.py` 几乎不用动 —— platform 模块只链接 user32 / shell32，
    不污染 core 接口。
  - AppState 是 pure C++ → UI 重塑后单测可以原地改期望值（不用接 GUI 测试框架）。
  - ADR-0003 + 三份 contract（`ui-mainview.md`、`platform.md`、`dictionary.md` change log）
    把"为什么 0.3.0 不再有 history/favorites 侧栏"的来龙去脉钉死在 commit 里。

- **平台层的测试空缺（已知账面）**
  - 托盘 / 全局钩子 / 焦点恢复目前是 **集成测试** —— CI 只证明编译链接通过，
    真实行为靠人/装好 .exe 验证。
  - 下一轮 (iter-012) 计划加 `tests/platform/test_fake_shell.cpp`，
    把 `IPlatformShell` 做成可注入，驱动 AppState 的 "user pressed hotkey →
    overlay should show" 这一类行为，仍然不用真实窗口。
