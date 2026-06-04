# EasyEnglish ：AI 全流程协作的 C++ 单词查询软件 — 实施计划 + 完成记录

> 目标：在 Windows 上用 C++ 实现一款单词查询软件，全程由 AI 协助构建。
> **状态：全部 12 个 todo 完成，main 分支 CI 全绿（61/61 tests pass）。**

---

## 完成情况速览

| Todo | 状态 | 关键交付 |
|---|---|---|
| `bootstrap-repo` | ✅ done | 目录结构 / CMake / vcpkg / .clang-format / .clang-tidy / .editorconfig |
| `governance-docs` | ✅ done | AGENTS.md / 契约 / ADR-0001 / prompt 模板 / PR 模板 |
| `iter-000-skeleton` | ✅ done | 空 Qt 主窗口 + CI 全链路 / `tools/check_core_no_ui.py` 守卫 |
| `iter-001-storage` | ✅ done | `Database`/`Statement` RAII + `mini_dict.sqlite` fixture（54 词） |
| `iter-002-dictionary-exact` | ✅ done | `IDictionary` + `SqliteDictionary::lookup` + benchmark |
| `iter-003-ui-mainwindow` | ✅ done | 搜索框 + 结果面板 + 状态行 + 5 个 QTest 行为测试 |
| `iter-004-history` | ✅ done | `HistoryStore`（UPSERT）+ 侧栏 + `Database::createOrOpen` |
| `iter-005-favorites` | ✅ done | `FavoritesStore`（幂等）+ ★/☆ ToolButton + Favorites tab |
| `iter-006-fuzzy` | ✅ done | Levenshtein `suggest()` + golden tests |
| `iter-007-network` | ✅ done | `INetworkClient` + `QtNetworkClient` + `ApiDictionary` + 9 个 Mock 测试 |
| `iter-008-package` | ✅ done | E2E QTest + Inno Setup `.iss` + `tools/build_installer.ps1` |
| `quality-gates` | ✅ done | clang-format CI · /W4/WX · ASan · arch 守卫 · benchmark 烟雾 |

仓库远端：https://github.com/DexterDreeeam/EasyEnglish

---

## 0. 实际选型（与原计划一致，少量调整）

| 维度 | 选择 | 备注 |
|---|---|---|
| UI 框架 | **Qt 6.8.3** | 通过 install-qt-action 装预编译二进制（CI 7m20s vs vcpkg 源码编译 1-3h） |
| 编译器 | MSVC v145 / 2022（VS18 Enterprise CI 镜像 windows-2025-vs2026） | C++23 (`std::expected`) |
| 构建系统 | **CMake 3.25+ + vcpkg manifest** | vcpkg 只管 sqlite3/fmt/gtest/benchmark；Qt 走 install-qt-action |
| 数据源 | 本地 SQLite + 在线 dictionaryapi.dev | 两条 IDictionary 实现 (SqliteDictionary, ApiDictionary) |
| 单元测试 | GoogleTest + GMock | 见各 `tests/unit/<module>/` |
| UI 测试 | QTest + QSignalSpy + (E2E) real-store 集成 | 没接 WinAppDriver/Squish — 跨外部进程的维护成本太高 |
| 静态分析 | clang-format `--Werror` 在 CI 拦截 | clang-tidy 完整接入留作后续（需 compile_commands + Qt 头路径） |
| 动态分析 | AddressSanitizer (MSVC `/fsanitize=address`) | Debug 构建默认 ON |
| 性能 | Google Benchmark (bench_lookup) | CI 烟雾跑，未设硬阈值（baseline 待积累后定） |
| CI | GitHub Actions windows-latest（实跑 windows-2025-vs2026） | 单次构建+测试约 7 分钟 |
| 界面语言 | 中文 + 英文 UI 字符串 via `tr()` | i18n 入口预留 |

---

## 1. 设计哲学：为什么"AI 协作"需要专门的工程框架

LLM 写代码的三大失败模式都源于"上下文塌缩"：

1. **遗忘 / 漂移**：跨文件、跨会话时记不住既定约定 → 用**显式规范文件**固化。
2. **过度自由**：把"补一个小函数"变成"顺手重构一整层" → 用**模块边界 + 任务卡 + 接口冻结**约束。
3. **难以验证**：生成代码看起来对，但行为没人检验 → 用**自动化测试做客观判官**，让 AI 自己驱动 red-green-refactor。

实际效果（这次执行）：
- 9 张任务卡 + 9 个迭代 retro 把所有变更都框在"已声明范围"内
- 7 份模块契约（storage / dictionary / history / favorites / network / ui-mainwindow + 模板）让后续迭代不需要回看实现就能 reuse
- CI 在每次提交后 7 分钟反馈，捕捉了 AUTOMOC 子目录依赖坑、QNetworkRequest Most-Vexing-Parse、Windows 文件锁等 7 处 AI 容易踩的坑

---

## 2. 顶层目录结构（最终）

```
EasyEnglish/
├── AGENTS.md                  ← AI 宪法（实际 ~70 行，远低于 150 行预算）
├── README.md
├── LICENSE                    ← MIT
├── CMakeLists.txt             ← C++23 + /W4 /WX + EASYENGLISH_ENABLE_ASAN
├── CMakePresets.json          ← msvc-debug / msvc-release
├── vcpkg.json                 ← sqlite3, fmt, gtest, benchmark
├── .clang-format / .clang-tidy / .editorconfig / .gitignore
├── .github/
│   ├── workflows/ci.yml       ← Configure / Build / Test / Bench / Format
│   └── pull_request_template.md
├── docs/
│   ├── architecture.md
│   ├── adr/0001-choose-qt6.md
│   ├── contracts/  storage / dictionary / history / favorites / network / ui-mainwindow + _TEMPLATE
│   ├── prompts/    feature-task / bugfix-task / refactor-task
│   └── iterations/ iter-000 ~ iter-008 各自 README + task + retro
├── src/
│   ├── core/
│   │   ├── storage/   Database, Statement, errors
│   │   ├── dictionary/ IDictionary, Entry, SqliteDictionary, ApiDictionary, errors
│   │   ├── history/   HistoryStore
│   │   ├── favorites/ FavoritesStore
│   │   └── network/   INetworkClient, QtNetworkClient
│   ├── app/           placeholder（未来抽 CompositeDictionary 等）
│   ├── ui/            MainWindow（含搜索/历史/收藏侧栏）
│   └── main.cpp
├── tests/
│   ├── unit/  storage / dictionary / history / favorites / network
│   ├── ui/    smoke + search + e2e_main_flow
│   ├── fixtures/  mini_dict.sqlite + fuzzy/{appl,banaba}.golden
│   └── benchmarks/  bench_smoke + bench_lookup
├── installer/   EasyEnglish.iss
└── tools/       check_core_no_ui.py + seed_db.py + build_installer.ps1
```

---

## 3-9. AI 协作框架 / 测试金字塔 / 风险与缓解 / 验收标准

（原计划完整保留，详见仓库内的 `AGENTS.md` / `docs/contracts/` / `docs/iterations/*/retro.md`）

---

## 10. 真实碰到的"学费"汇总（节选自各 iter retro）

| 迭代 | 坑 | 教训 / 已加固 |
|---|---|---|
| iter-000 | vcpkg 源码编译 qtbase → CI 1-3h | 换 install-qt-action 走二进制（7m20s） |
| iter-000 | `vcpkg.json` builtin-baseline 写日期占位 | 必须真 SHA；CI 立即抓 |
| iter-000 | `lukka/run-vcpkg` 拒绝 `'master'` | 完整 commit SHA 固定 |
| iter-001 | 全局 `CMAKE_AUTOMOC ON` + 子目录新依赖 → `_autogen_timestamp_deps does not exist` | 改为 per-target，写进 AGENTS.md |
| iter-003 | `resultReady(Entry)` 信号需 `Q_DECLARE_METATYPE` | 信号改传 `QString headword` |
| iter-004 | Windows SQLite 文件锁导致 `std::filesystem::remove` 抛异常 | 把 db handle scope 严格限定；`remove(path, ec)` 不抛 |
| iter-005 | UI 测试误用 `ASSERT_TRUE`（GoogleTest 宏） | QTest 必须用 `QVERIFY` |
| iter-007 | `QNetworkRequest req(QUrl(url))` Most-Vexing-Parse | 用花括号初始化 |

---

## 11. 后续可能的迭代（未做，仅记录）

- `quality-gates+`: 真正接入 clang-tidy（生成 compile_commands + Qt 头路径）
- `iter-009-composite-dict`: `app/CompositeDictionary` 合并 local + online 结果
- `iter-010-snapshots`: UI 截图回归（`QWidget::grab()` 像素 diff）
- `iter-011-i18n`: Qt linguist 工作流，做完中文 + 英文翻译
- `iter-012-perf-index`: SqliteDictionary 大字典优化（trie + edit-distance 剪枝）


---

## 0. 假设与决策（用户未在场时的默认选型，可推翻）

| 维度 | 选择 | 理由（针对 AI 协作 + 可测试性） |
|---|---|---|
| UI 框架 | **Qt 6 (Widgets)** | AI 训练语料最多、官方文档完整、自带 `QTest` 单元/UI 测试、跨平台便于以后扩展、信号槽机制天然解耦 |
| 编译器 | MSVC 2022 (v143) | Windows 一等公民，调试器/ASan/PDB 体验最好 |
| 构建系统 | **CMake ≥ 3.25 + vcpkg (manifest 模式)** | AI 写 `CMakeLists.txt` 错误率最低；vcpkg manifest 让依赖锁定可重现 |
| 数据源 | 本地 SQLite（主） + 在线 API（可插拔次） | SQLite 数据可预置、断言稳定，是单元测试基石；API 通过 mock 注入 |
| 单元测试 | GoogleTest + GMock | 行业标准，AI 输出最稳定 |
| UI 测试 | QTest + Squish/Qt Test 脚本 + 截图快照 | Qt 原生，自动化成本最低 |
| 静态分析 | clang-tidy + cppcheck + `/W4 /WX` | 在 CI 上强制门禁，杜绝 AI 隐式产生的低级错误 |
| 动态分析 | AddressSanitizer (MSVC `/fsanitize=address`) | 抓内存问题 |
| 性能 | Google Benchmark（仅核心查询路径） | 防止 AI 重构后性能退化 |
| CI | GitHub Actions（windows-latest） | 自动跑构建、lint、单测、UI 烟雾测试 |
| AI 协作严格度 | **标准级** | 不至于淹没 AI，又能保证一致性 |
| 界面语言 | 中文（预留 i18n 入口） | 与用户母语一致 |
| MVP 功能 | 查询 / 历史 / 收藏 / 模糊匹配 | 范围窄 → AI 单次任务可控 |

---

## 1. 设计哲学：为什么"AI 协作"需要专门的工程框架

LLM 写代码的三大失败模式都源于"上下文塌缩"：

1. **遗忘 / 漂移**：跨文件、跨会话时记不住既定约定 → 用**显式规范文件**固化。
2. **过度自由**：把"补一个小函数"变成"顺手重构一整层" → 用**模块边界 + 任务卡 + 接口冻结**约束。
3. **难以验证**：生成代码看起来对，但行为没人检验 → 用**自动化测试做客观判官**，让 AI 自己驱动 red-green-refactor。

下面所有规范都是为了**把这三件事变成可强制执行的工程制度**。

---

## 2. 顶层目录结构（仓库脚手架）

```
EasyEnglish/
├── AGENTS.md                  ← 给 AI 的"宪法"，每次会话都应被读取
├── README.md                  ← 给人的项目说明
├── CMakeLists.txt
├── CMakePresets.json          ← 锁定 MSVC + vcpkg 配置，AI 不需要再"猜"
├── vcpkg.json                 ← 依赖清单（manifest 模式）
├── .clang-tidy
├── .clang-format
├── .editorconfig
├── .github/
│   ├── workflows/ci.yml       ← 构建 + 测试 + lint 门禁
│   └── pull_request_template.md
├── docs/
│   ├── architecture.md        ← C4 模型 / 模块关系图
│   ├── adr/                   ← Architecture Decision Records（每个重大决策一篇）
│   │   └── 0001-choose-qt6.md
│   ├── contracts/             ← 每个模块的"对外契约"（接口 + 不变量 + 错误码）
│   │   ├── dictionary.md
│   │   ├── storage.md
│   │   └── ui-mainwindow.md
│   ├── prompts/               ← 复用的 AI prompt 模板
│   │   ├── feature-task.md
│   │   ├── bugfix-task.md
│   │   └── refactor-task.md
│   └── iterations/            ← 每一轮迭代的任务卡 + 回顾
│       └── iter-001-mvp-lookup/
│           ├── task.md
│           ├── acceptance.md
│           └── retro.md
├── src/
│   ├── core/                  ← 纯逻辑层，零 UI 依赖（最重要！）
│   │   ├── dictionary/        ← 查询引擎 + 模糊匹配
│   │   ├── storage/           ← SQLite 封装
│   │   ├── history/
│   │   ├── favorites/
│   │   └── network/           ← 在线 API 客户端（接口 + Mock 实现）
│   ├── app/                   ← 应用服务层（编排 core）
│   └── ui/                    ← Qt Widgets（视图层，薄）
├── tests/
│   ├── unit/                  ← 1:1 对应 src/core 的每个子目录
│   ├── ui/                    ← QTest 驱动的 UI 测试
│   ├── snapshots/             ← UI 截图基线
│   ├── fixtures/              ← 预置 SQLite 词库 + API 录像
│   └── benchmarks/
└── tools/
    ├── seed_db.py             ← 生成测试词库
    └── snapshot_diff.py       ← 截图差异对比
```

**关键纪律**：`src/core/**` **不得 `#include` 任何 Qt UI 头**（只允许 `QtCore` 的容器/字符串）。这条规则写进 `AGENTS.md`，并由 CI 用 `grep` 检查。这样核心逻辑可以脱离 UI 独立测试，AI 写错时一定会被门禁挡下。

---

## 3. AI 协作框架（核心交付物 ①）

### 3.1 `AGENTS.md` — AI 的"宪法"

放在仓库根目录，任何 AI 编程助手（Copilot CLI / Cursor / Claude Code 等）启动会话时都会优先读取。**内容必须短而强约束**，建议覆盖：

1. **构建命令**：`cmake --preset msvc-debug && cmake --build --preset msvc-debug`
2. **测试命令**：`ctest --preset msvc-debug --output-on-failure`
3. **lint 命令**：`clang-tidy -p build/msvc-debug ...`
4. **代码风格**：指向 `.clang-format`；明确"不要在 `src/core/` 里 include Qt UI"
5. **依赖管理**：所有新依赖**必须**先改 `vcpkg.json`，禁止 `FetchContent`
6. **目录约束**：见上一节
7. **修改约束**：
   - 一次任务只动一个模块；跨模块改动必须先开一个 ADR
   - 不允许"顺手优化"无关代码
   - 公开接口（`contracts/` 里描述过的）若需变更，必须在 PR 描述里说明
8. **测试要求**：新增功能必须同时提交单元测试；UI 变更必须更新或新增 QTest
9. **错误处理约定**：core 层用 `std::expected<T, ErrorCode>`（C++23）或 `tl::expected`，禁止异常穿越模块边界
10. **如何提问**：拿不准就先回答 "I don't know"，禁止编造 API

> 经验：把 `AGENTS.md` 控制在 **150 行内**，且每条都用祈使句。冗长的规范 AI 会"礼貌地忽略"。

### 3.2 模块契约（`docs/contracts/*.md`）

每个 `src/core/<module>/` 子目录都有一份契约文档，结构：

```
# Dictionary Module Contract

## Public API (frozen, change requires ADR)
class IDictionary {
  virtual auto lookup(std::string_view word) const
      -> std::expected<Entry, DictError> = 0;
  virtual auto suggest(std::string_view prefix, size_t max = 10) const
      -> std::vector<std::string> = 0;
};

## Invariants
- lookup 必须是线程安全的（const 调用并发安全）
- suggest 返回的字符串按编辑距离升序排列
- 空字符串输入返回 DictError::InvalidInput

## Error codes
| Code | Meaning |
|---|---|
| NotFound | 词条不存在 |
| InvalidInput | 输入非法 |
| StorageError | 底层存储异常 |

## Dependencies
- 仅依赖 src/core/storage
- 禁止依赖 Qt（除 QtCore 容器）

## Test fixtures
- tests/fixtures/mini_dict.sqlite（200 词的固定子集）
```

**这是 AI 上下文的"短期记忆替代品"**：下次会话开始，只要让 AI 读契约文件，它就能在不读全部源码的情况下做正确的修改。

### 3.3 任务卡模板（`docs/prompts/feature-task.md`）

每个新功能/Bug 都先写一张卡，再让 AI 实现。模板：

```markdown
# Task: <一句话目标>

## Context (必读)
- 涉及模块: src/core/dictionary, src/ui/mainwindow
- 相关契约: docs/contracts/dictionary.md
- 相关 ADR: docs/adr/0003-fuzzy-algo.md

## Out of scope
- 不改 storage 层
- 不引入新依赖

## Acceptance criteria (机器可验证)
- [ ] `ctest -R DictionaryFuzzyTest` 全部通过
- [ ] UI 烟雾测试 `MainWindowSearchTest::testFuzzyHint` 通过
- [ ] `clang-tidy` 无新告警
- [ ] 查询 "appl" 1ms 内返回 ≥1 个建议（benchmark）

## Implementation hints (可选)
- 可考虑 Levenshtein + 前缀剪枝
- 复用 src/core/dictionary/trie.hpp

## Definition of Done
- 新增/更新 tests/unit/dictionary/
- 更新 contracts/dictionary.md 的 Invariants 段落（如果改了语义）
- 在 docs/iterations/iter-NNN/retro.md 写一行总结
```

把这张卡作为 prompt 喂给 AI，它生成的代码会显著更聚焦、可验收。

### 3.4 ADR（架构决策记录）

`docs/adr/NNNN-<slug>.md`，模板见 [MADR](https://adr.github.io/madr/)。任何**跨模块**、**改公共 API**、**引入新依赖**的决定都要写一篇。这是给 AI 的"为什么这么做"的长期记忆，避免下一次 AI 因为不知道历史而做出回归。

### 3.5 Prompt 复用与 PR 模板

- `docs/prompts/` 存可复用 prompt（重构、加测试、修 bug 等）。
- `.github/pull_request_template.md` 强制填写：
  - 关联任务卡 / Issue
  - 修改的契约清单
  - 测试列表 + 本地运行结果
  - AI 生成代码自查表（详见 §6）

---

## 4. 迭代开发流程（核心交付物 ②）

采用 **"小步红绿循环"** ，每轮迭代 = 1 张任务卡 = 1 个 PR：

```
            ┌─────────────────────────────────────┐
            │  iter-NNN 任务卡 (人或 AI 协助起草) │
            └──────────────┬──────────────────────┘
                           ▼
              ① AI 阅读: AGENTS.md + 相关 contract + 任务卡
                           ▼
              ② AI 先写 / 更新测试 (red)         ← 用 AI 写测试比写实现更安全
                           ▼
              ③ 运行 ctest → 应失败
                           ▼
              ④ AI 写最小实现 (green)
                           ▼
              ⑤ 运行 ctest + clang-tidy + ASan
                           ▼
              ⑥ AI 自检清单 (§6) → 若失败回到 ②
                           ▼
              ⑦ 人工 review（聚焦契约/边界，而非语法）
                           ▼
              ⑧ 合并；在 retro.md 写 1–3 行学到的事
                           ▼
              ⑨ 若新 API 被冻结进契约，写 ADR
```

### 4.1 MVP 拆分（建议迭代顺序）

| 迭代 | 目标 | 涉及模块 |
|---|---|---|
| iter-000 | 脚手架：CMake + vcpkg + CI + 空 Qt 窗口 | 全部 |
| iter-001 | core/storage：SQLite 封装 + 单测 | core/storage |
| iter-002 | core/dictionary：精确查询 + 单测 + benchmark | core/dictionary |
| iter-003 | ui/mainwindow：搜索框 + 结果面板 + QTest 烟雾 | ui |
| iter-004 | core/history：查询历史记录 + UI 集成 | core/history, ui |
| iter-005 | core/favorites：收藏夹 + UI 集成 | core/favorites, ui |
| iter-006 | core/dictionary：模糊匹配 / 拼写建议 | core/dictionary |
| iter-007 | core/network：在线 API 客户端（接口 + Mock） | core/network |
| iter-008 | 集成 & 端到端 UI 测试 + 安装包 (WiX/Inno) | 全部 |

**单次 PR 控制在 ~400 行以内** —— 这是 AI 协作的甜蜜区，超出后 review 质量会断崖下降。

---

## 5. 测试策略（核心交付物 ③）

### 5.1 测试金字塔（自下而上）

```
                ┌─────────────────────────┐
                │  E2E (1–3 个关键路径)    │  Squish 或 WinAppDriver
                ├─────────────────────────┤
                │  UI 组件测试 (QTest)    │  ~ 每个 Widget 1–3 个
                ├─────────────────────────┤
                │  集成测试 (core + sqlite) │  少量、跨 ≤ 2 个模块
                ├─────────────────────────┤
                │   单元测试 (GoogleTest)  │  大量、覆盖 core 90%+
                └─────────────────────────┘
```

### 5.2 功能模块测试（`tests/unit/`）

- 每个 `src/core/<m>/` 对应 `tests/unit/<m>/`，文件名 `test_<class>.cpp`
- 使用 **预置 SQLite 词库**（`tests/fixtures/mini_dict.sqlite`，约 200 词，git 跟踪）
  → 断言可写成 `EXPECT_EQ(dict.lookup("apple")->phonetic, "/ˈæp.əl/")`
- 在线 API 客户端只测**接口**，用 GMock 实现 `INetworkClient` 做依赖注入
- 模糊匹配等带"建议"的功能采用 **golden file** 测试：
  ```
  tests/fixtures/fuzzy/appl.golden  ->  ["apple","apply","ample"]
  ```
  golden 更新需 reviewer 显式批准，防止 AI 偷偷"修正"期望值。

### 5.3 UI 模块测试（`tests/ui/`）

三层组合，覆盖 AI 最容易跑偏的视图层：

| 层级 | 工具 | 验证什么 | 例子 |
|---|---|---|---|
| **行为测试** | `QTest::keyClicks` / `QTest::mouseClick` + `QSignalSpy` | 交互 → 信号发出 / 内部状态正确 | 输入 "apple" 后 `searchRequested(QString)` 被发射 |
| **视觉快照** | `QWidget::grab()` → PNG，存 `tests/snapshots/` | 像素级 diff（容差 < 1%） | 主窗口初始布局、深色主题、空状态 |
| **端到端** | Qt Squish *或* WinAppDriver + WebDriverIO | 真实启动 .exe，模拟用户全流程 | 启动 → 输入 → 复制结果 → 收藏 |

**约定**：
- `tests/ui/test_<Window>.cpp` 中**禁止访问真实数据库 / 网络**；所有依赖通过依赖注入或 Mock 替换（这条由代码评审强制）。
- 快照基线变更必须在 PR 描述里贴 diff 截图，并由人审。AI 不得自行用 `--update-snapshots` 参数无脑刷新。

### 5.4 静态 & 动态门禁（CI 强制）

- `clang-tidy`：启用 `bugprone-*, modernize-*, performance-*, readability-identifier-naming`
- `cppcheck --enable=warning,performance,portability`
- MSVC `/W4 /WX`：警告即错误
- **架构约束扫描**（自写 5 行 Python）：检查 `src/core/**/*.h(pp)?` 是否包含 `<QtWidgets`，违规 → CI 失败
- Debug 构建打开 `/fsanitize=address`，CI 跑单测时启用

### 5.5 性能门禁（防回归）

`tests/benchmarks/bench_lookup.cpp` 在 CI 跑 Google Benchmark，断言：

```
lookup_exact_p99 < 500us
lookup_fuzzy_p99 < 5ms (10万词典)
```

回归 > 20% 让 CI 失败。

---

## 6. AI 自检清单（每次提交前 AI 必须自答）

把这张清单作为 prompt 末尾追加，让 AI 在提交前 self-review：

1. 我修改的文件是否都在任务卡声明的范围内？
2. 我是否新增/更新了**单元测试**？测试在本地通过吗？
3. 我有没有改动 `docs/contracts/` 中声明的公共接口？若有，是否更新了契约 + 写了 ADR？
4. 我是否引入了新依赖？若有，是否只通过 `vcpkg.json`？
5. `src/core/**` 是否出现了 Qt UI 头？
6. 是否运行了 `clang-tidy` 与 `ctest`？输出贴出来。
7. 有没有用魔法数字 / TODO / `// AI generated` 注释？请清理。
8. 关键函数是否有简短英文 doc comment？

> 这张表会在 PR 模板里再要人填一遍，形成"AI 自检 + 人复核"双层验证。

---

## 7. 风险与缓解

| 风险 | 触发场景 | 缓解 |
|---|---|---|
| AI 在跨会话间忘记约定 | 新会话 / 换 IDE | 强制每次首条指令引用 `AGENTS.md` 与相关契约文件 |
| AI 引入隐藏依赖 | 写网络代码偷偷 include `<curl>` | vcpkg manifest + CI 检测未声明依赖 |
| 测试被 AI"调整"以通过 | 改实现时顺手改测试 | golden 文件 + 任务卡 acceptance 由人写 + PR 里高亮测试 diff |
| UI 快照频繁假阳性 | 字体渲染差异 | CI 固定 Windows 镜像 + 容差阈值；快照按 DPI 分目录 |
| 范围蔓延 | 一个 PR 改 5 个模块 | PR 模板要求列出"修改的模块数"，> 1 自动打 `needs-discussion` 标签 |
| 性能慢慢退化 | 多次重构后 | Google Benchmark 在 CI 比对基线 |

---

## 8. 验收标准（整个项目层面）

- [ ] 全新克隆者执行 `cmake --preset msvc-debug && cmake --build --preset msvc-debug && ctest --preset msvc-debug` 一次成功
- [ ] CI 在 PR 上自动跑：构建 / 单测 / UI 烟雾 / clang-tidy / ASan
- [ ] `AGENTS.md` 存在且 ≤ 150 行
- [ ] 每个 `src/core/<m>/` 都有对应契约与单测目录
- [ ] MVP 功能（查询 / 历史 / 收藏 / 模糊匹配）端到端可用
- [ ] 安装包（Inno Setup 或 WiX）一键产出
- [ ] 至少 3 篇 ADR 记录关键决策

---

## 9. 待办分解（同步入 SQL `todos`）

> ID 用 kebab-case，便于后续 `WHERE id = 'xxx'` 引用。

- `bootstrap-repo`：建仓库脚手架（目录、AGENTS.md、CMake、vcpkg、CI 空跑通）
- `governance-docs`：写 AGENTS.md / 契约模板 / 任务卡模板 / PR 模板 / ADR-0001
- `iter-000-skeleton`：CMake + vcpkg + 空 Qt 主窗口 + CI 绿
- `iter-001-storage`：core/storage SQLite 封装 + 单测 + fixtures
- `iter-002-dictionary-exact`：精确查询 + benchmark
- `iter-003-ui-mainwindow`：搜索框 + 结果面板 + QTest 烟雾 + 快照基线
- `iter-004-history`：查询历史 + UI 集成
- `iter-005-favorites`：收藏夹 + UI 集成
- `iter-006-fuzzy`：模糊匹配 + golden tests
- `iter-007-network`：在线 API 客户端（接口 + Mock + 真实实现）
- `iter-008-package`：端到端 UI 测试 + Inno Setup 安装包
- `quality-gates`：clang-tidy / ASan / 架构扫描脚本 / benchmark 门禁全部接入 CI

依赖关系：除 `bootstrap-repo` 与 `governance-docs` 外，`iter-NNN` 严格按序；`quality-gates` 与 `iter-001` 并行。

---

## 10. 备注 / 待和你确认的点（不阻塞推进）

1. **UI 框架**默认选了 Qt 6。如果你更想要"原生 Windows 体验"，可切到 WinUI 3，但 AI 协助难度与测试成本会升一档（会写 ADR 说明）。
2. **在线 API** 默认接 `dictionaryapi.dev`（免 key 免费）。若需要中英互译，再加 youdao / 自建后端。
3. **i18n** 仅预留入口，MVP 只交付中文。
4. **打包**默认 Inno Setup（简单）。若需 MSIX，加一轮迭代。
5. **性能基准的阈值**（500us / 5ms）是经验值，需要 iter-002 实测后回填。
