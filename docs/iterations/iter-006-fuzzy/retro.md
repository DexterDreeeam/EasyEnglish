# iter-006 Retrospective

- **What shipped**: 真正的 `suggest()`（Levenshtein + 预加载 headwords cache + 字典序稳定排序），golden 测试 `tests/fixtures/fuzzy/{appl,banaba}.golden`，contract 冻结。

- **AI走偏**: 起初想把 `headwords_cache_` 通过 `mutable + lazy init + mutex` 实现按需加载——多余的复杂度。改为 open() 时直接 eager load，构造完成后只读，整个 `suggest()` 不再需要锁（`lookup()` 仍然有 stmt mutex，二者不竞争）。

- **下次保留**：golden 文件命名 `<input>.golden`，JSON 数组，断言 `suggest(input, golden.size()) == golden` — 简单、确定、易扩展。新增 golden 只要 `python tools/seed_db.py` 改了词表的话需要同步刷新。
