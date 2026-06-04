# Bugfix Task: <一句话描述 bug>

## Reproduction

- 复现步骤：
  1. …
  2. …
- 实际：…
- 预期：…
- 触发版本 / commit: …

## Suspected scope

- 最可能的模块：`<src/core/...>`
- 相关契约：`docs/contracts/<...>.md`

## Out of scope

- 不修与 bug 无关的代码（即便看起来糟糕）。发现新 bug 写新任务卡。

## Acceptance criteria

- [ ] **先**加一个能复现该 bug 的回归测试 `<TestName>`，并确认它在修复前 fail
- [ ] 该测试在修复后通过
- [ ] 既有测试套件全部通过：`ctest --preset msvc-debug`
- [ ] 在 retro.md 中写一行 root cause（不超过 50 字）

## Definition of Done

- 回归测试保留在 `tests/unit/...` 或 `tests/ui/...`
- 如果根因在契约不变量上：更新契约 + ADR
