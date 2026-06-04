# Task: <一句话目标>

> 复制本文件到 `docs/iterations/iter-NNN-<slug>/task.md`。
> 删除所有"<…>"占位提示后再提交。

## Context (必读)

- **涉及模块**: `<src/core/dictionary>`, `<src/ui/mainwindow>`
- **相关契约**: `docs/contracts/<dictionary>.md`
- **相关 ADR**:  `docs/adr/<NNNN-...>.md`（无则写 "none"）
- **依赖前置任务**: `<iter-NNN>` 必须先合并

## Out of scope (明确写出不要做什么)

- 不改 `<storage>` 层
- 不引入新依赖
- 不重构 `<…>` 中无关代码

## Acceptance criteria (机器可验证)

- [ ] `ctest --preset msvc-debug -R <TestPattern>` 全部通过
- [ ] 新增/更新的 UI 测试 `<TestName>` 通过
- [ ] `python tools/check_core_no_ui.py` 通过
- [ ] `clang-tidy` 无新告警
- [ ] benchmark `<bench_name>` p99 < `<阈值>`
- [ ] 用户视角验收：在 `<场景>` 下能看到 `<预期结果>`

## Implementation hints (可选)

- 提示 1
- 可参考 `<src/core/.../trie.hpp>`
- 已知陷阱：`<...>`

## Definition of Done

- 新增/更新 `tests/unit/<module>/`
- 如果改了公共 API：更新 `docs/contracts/<module>.md` + 新增 ADR
- 在 `docs/iterations/iter-NNN-<slug>/retro.md` 写 1–3 行：
  做了什么 / AI 在哪走偏过 / 下次怎么避免
