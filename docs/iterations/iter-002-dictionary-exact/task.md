# Task: iter-002-dictionary-exact — exact lookup via SQLite

## Context

- **涉及模块**: `src/core/dictionary/`, `tests/unit/dictionary/`, `tests/benchmarks/`
- **相关契约**: `docs/contracts/dictionary.md`（lookup 部分本迭代冻结，suggest 留 iter-006）
- **依赖前置任务**: `iter-001-storage` ✅

## Out of scope

- 不实现 `suggest()` / 模糊匹配（iter-006）
- 不接 UI（iter-003）
- 不引入新依赖

## Acceptance criteria

- [ ] `IDictionary` 接口 + `SqliteDictionary` 实现
- [ ] `lookup("apple")` 返回正确 `Entry`（包含 headword/phonetic/definitions）
- [ ] `lookup("APPLE")` 也能命中（大小写不敏感）
- [ ] `lookup("")` → `DictError::InvalidInput`
- [ ] `lookup("nonexistent")` → `DictError::NotFound`
- [ ] `suggest()` 返回空（占位实现，iter-006 真实现）
- [ ] benchmark `bench_lookup_exact` p99 < 500us（mini fixture）
- [ ] CI 全绿（windows-build + clang-format）

## Implementation hints

- 用预编译语句缓存（成员持有一个常用的 prepared `Statement`），避免每次查询 `prepare()` 开销
- definitions 字段是 JSON 数组（seed_db.py 写入），用 Qt 的 `QJsonDocument`（已 link Qt6::Core，零新依赖）

## Definition of Done

- 新增 `src/core/dictionary/{IDictionary,SqliteDictionary,Entry,errors}.hpp/.cpp`
- 新增 `tests/unit/dictionary/test_*.cpp`
- 新增 `tests/benchmarks/bench_lookup.cpp`
- 更新 `docs/contracts/dictionary.md` 把 lookup 段冻结
- 写 retro
