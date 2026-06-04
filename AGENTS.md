# AGENTS.md — EasyEnglish 仓库宪法

> 任何 AI 编程助手（Copilot CLI / Cursor / Claude Code / 其他）开始工作前 **必须** 读完本文件。
> 本文件刻意保持简短（≤ 150 行）。冗长规范会被礼貌地忽略。

## 1. 项目一句话

C++23 / Qt 6 写的 Windows 单词查询桌面应用。开发流程是"AI 起草 + 自动化测试 + 人工 review"。

## 2. 一定要先读

- 你要改动的模块的契约：`docs/contracts/<module>.md`
- 当前迭代任务卡：`docs/iterations/iter-NNN-*/task.md`
- 任何被引用的 ADR：`docs/adr/NNNN-*.md`

不读上面三类文件就开始改代码 = 违反本宪法。

## 3. 构建 / 测试 / 静态检查（**必须能跑通**才能提交）

```powershell
cmake --preset msvc-debug
cmake --build --preset msvc-debug --parallel
ctest --preset msvc-debug --output-on-failure
python tools\check_core_no_ui.py
```

## 4. 目录纪律

| 目录 | 可以依赖 | 禁止依赖 |
|---|---|---|
| `src/core/**` | `Qt6::Core`（容器/字符串） | `QtWidgets`, `QtGui`, `QtQuick`，任何 UI 头；`Q_OBJECT`（core 没有 moc 目标） |
| `src/app/**`  | `src/core/**`              | `QtWidgets`, `QtGui`, `src/ui/**` |
| `src/ui/**`   | `src/app/**`, `Qt6::Widgets` | 直接 `#include "core/.../*.hpp"`（应经 app 编排） |
| `tests/unit/**` | 对应 `src/core/<m>`, GTest, GMock | `Qt6::Widgets`, 真实网络, 非 fixture 数据库 |
| `tests/ui/**`   | `Qt6::Test`, `Qt6::Widgets`, mock 的 IDictionary/INetworkClient | 真实 SQLite 路径、真实网络 |

违反"core 不依赖 UI"会被 `tools/check_core_no_ui.py` 在 CI 上拦下。

## 5. 修改约束

1. **一次任务卡 = 一次 PR = 一个模块为主**。跨模块改动必须先开 ADR。
2. **禁止顺手重构** 不在任务卡范围内的代码。哪怕看着不顺眼。
3. **公开 API**（在 `docs/contracts/` 里写了的）若需变更，必须：
   - 更新对应契约文件；
   - 写一篇 ADR 解释为什么；
   - 在 PR 描述里把变更面列出来。
4. **新依赖** 只允许通过 `vcpkg.json` 引入。**禁止** `FetchContent`、git submodule、复制粘贴第三方源码。
5. **错误处理**：`src/core/**` 用 `std::expected<T, ErrorCode>`（C++23，必要时用 `tl::expected` 兜底），
   不允许异常穿越模块边界。
6. **不要编造 API**。不确定 Qt/标准库某个函数是否存在时，先回答"不确定"，再去查或问。

## 6. 代码风格

- 由 `.clang-format` 与 `.clang-tidy` 强制；不需要在 PR 里讨论格式。
- 命名空间统一前缀 `easyenglish::`，子命名空间 `core/app/ui/<module>`。
- 公共头文件用 `#pragma once`；不允许 using namespace 在头里。

## 7. 测试要求

- 任何 `src/core/**` 改动 **必须** 同步更新 `tests/unit/<m>/`。
- 任何 `src/ui/**` 改动 **必须** 同步更新 `tests/ui/`（行为 + 必要时快照基线）。
- 模糊匹配 / 排序类输出用 golden file（`tests/fixtures/.../*.golden`），更新 golden 必须在 PR 里高亮。
- 禁止用 `--update-snapshots` 一类参数无脑刷新 UI 基线。

## 8. 提交前 self-check（每次回复用户"完成"前都要答完）

1. 我改动的文件是否都在任务卡声明的模块范围内？
2. 我是否同步加/改了单元测试？测试在本地 `ctest` 通过吗？
3. 我是否动了任何契约（`docs/contracts/`）声明的公共 API？若动了，契约 + ADR 是否更新？
4. 我引入了新依赖吗？若有，是否只通过 `vcpkg.json`？
5. `src/core/**` 是否仍然不含 Qt UI 头？`python tools/check_core_no_ui.py` 通过吗？
6. 没有遗留魔法数字 / `// TODO(ai)` / `// AI generated` 之类注释？
7. 关键函数有简短的英文 doc comment 吗？

回答必须以"是 / 否 + 证据（命令输出或 diff 引用）"的形式写在 PR 描述里。

## 9. 提问与不确定性

- 拿不准就明说"I don't know"，不要瞎编。
- 如果任务卡定义模糊或与契约冲突，**先停下**写一条澄清问题，不要靠想象继续。
- 如果发现现存代码有 bug 但与本任务无关 —— 在 PR 描述里**记录**它，但**不要修**。开一张新任务卡。

## 10. 范围之外

- 不写新文档（除了契约 / ADR / 任务卡 / retro）。
- 不修改 `AGENTS.md` 本身，除非用户明确要求。
- 不删除 `docs/iterations/` 下任何已存在的历史记录。
