# Refactor Task: <一句话目标>

> 重构 = 行为不变、结构改变。如果会改变可观测行为 —— **写成 feature-task，不是这个**。

## Motivation

- 当前结构存在的问题（耦合 / 复杂度 / 重复 / 命名）：
- 重构后的目标形状（文字或伪代码）：

## Scope

- 涉及文件：列全
- 涉及契约：是否需要变更？若是，必须先开 ADR
- 跨模块？跨模块重构必须先开 ADR 才能开始

## Safety net

- [ ] 重构前所有相关测试已绿（贴 `ctest` 输出）
- [ ] 不新增功能 / 不删除既有测试
- [ ] 不修改测试以"配合"重构（除了纯命名跟随）

## Acceptance criteria

- [ ] `ctest --preset msvc-debug` 与重构前结果一致（用例数 + 通过数）
- [ ] `clang-tidy` 无新告警
- [ ] `python tools/check_core_no_ui.py` 通过
- [ ] benchmark 无 > 5% 回归

## Definition of Done

- diff 中没有逻辑变化（只剩搬运 / 重命名 / 拆分 / 合并）
- retro.md 写一行：重构前后某个度量值变化（行数 / 圈复杂度 / 依赖数 等）
