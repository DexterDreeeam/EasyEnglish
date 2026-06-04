# Task: iter-006-fuzzy — Levenshtein-based suggest()

## Context

- 涉及模块: `src/core/dictionary/SqliteDictionary`, `tests/unit/dictionary/`, `tests/fixtures/fuzzy/`
- 相关契约: `docs/contracts/dictionary.md`（suggest 段冻结）
- 依赖前置任务: `iter-002-dictionary-exact` ✅

## Acceptance criteria

- [ ] `suggest("")` → `[]`
- [ ] `suggest("apple", N)` 第一个结果是 `"apple"`（精确匹配 distance 0）
- [ ] golden test for "appl" → `["apple","apply","ample"]`
- [ ] CI 全绿

## Implementation hints

- 加载所有 headword 到内存 cache（一次 open），suggest brute-force Levenshtein
- 排序：distance 升序，alphabetical 次序（cache 预先 sorted + std::stable_sort 即可）

## Definition of Done

- 契约 dictionary.md change log 更新
- retro 写完
