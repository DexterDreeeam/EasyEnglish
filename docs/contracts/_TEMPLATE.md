# 模块契约模板

> 复制这份模板到 `docs/contracts/<module>.md`，填好每一节再提交。
> "Frozen" 段落一旦合并就视为冻结，再次修改需要 ADR。

# `<Module Name>` Contract

**Source path**: `src/core/<module>/`
**Owner test path**: `tests/unit/<module>/`
**Status**: draft / frozen / deprecated

## 1. Public API (FROZEN — change requires ADR)

```cpp
// Paste the canonical header excerpt here. Keep it ≤ 30 lines.
namespace easyenglish::core::<module> {

class IFoo {
public:
    virtual ~IFoo() = default;
    virtual auto bar(std::string_view input) const
        -> std::expected<Result, ErrorCode> = 0;
};

}  // namespace
```

## 2. Invariants

- bullet 1 (e.g. thread-safety guarantee)
- bullet 2 (e.g. ordering guarantee)
- bullet 3 (e.g. empty-input contract)

## 3. Error codes

| Code | Meaning | Caller should… |
|---|---|---|
| `NotFound`     |  …  |  …  |
| `InvalidInput` |  …  |  …  |
| `StorageError` |  …  |  …  |

## 4. Dependencies

- Allowed: `src/core/storage`, `Qt6::Core`
- Forbidden: `Qt6::Widgets`, `src/ui/**`, network I/O

## 5. Test fixtures

- `tests/fixtures/<file>` — describe shape and how to regenerate

## 6. Performance budget

- e.g. `bar()` p99 < 500us on 100k-entry corpus.
- Benchmark target: `bench_<module>` in `tests/benchmarks/`.

## 7. Change log

- YYYY-MM-DD — initial draft (iter-NNN).
