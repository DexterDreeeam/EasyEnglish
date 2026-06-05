# iter-014 Retrospective

- **What shipped**: `ee-core` 五个子模块全部就位（`config` / `notes` / `history` / `lookup` / `state`）。35 个 Core 集成测试 + 1 个 unit test 全过；workspace 总共 54/54 tests pass，~0.55 s。Core/.interface.md 冻结。

- **关键设计取舍**
  1. **NoteStore 是 runtime-only**（按用户要求）。`Note { word, content }` 中 `content` 是任意字符串——可以是翻译、助记、URL、句型；语义上等于 v0.3.0 "favorites" 的超集。若 Phase 2 需要 persist，只要加 `persist_to(path)` 即可，接口不破坏。
  2. **Lookup 顺序由 Config 控制**：`prefer_notes_over_dict` 默认 true，覆盖式 Note 优先于词典；用户写 `add_note("apple", "我自己的翻译")` 后立即生效。dict-first 路径也实现并测试，方便日后切换。
  3. **HistoryStore 测试用 fake_clock**。`with_clock(max, fn)` 是 doc-hidden 的 test-only 入口，避免 wall-clock-flake；生产路径仍走 `with_capacity` → `default_clock()`。
  4. **Config 反序列化用嵌套 RawConfig + Option**。所有 JSON 字段都是 `Option<T>`，缺省时 `into_config()` 注入 defaults，`partial_override_keeps_other_defaults` 只 set 一项就能验证整套合并逻辑。

- **AI 走偏 / 教训**
  1. `AppState::recent()` 起初想返回 `&[HistoryEntry]`，但 `VecDeque` 拿不到连续 slice；改为 `Vec<HistoryEntry>` 克隆返回，单次拷贝 ≤ 50 条 entry，代价可忽略。
  2. `LookupHit` 用 enum 比 struct + Source field 更贴 "Note OR Dict" 的二选一语义。
  3. 没有重复 iter-013 的 toolchain / Debug 坑——AGENTS.md §3 的提示直接生效了。
  4. **流程小事故**：`New-Item` 与 `create` 工具放在同一回合并行执行，create 早于 mkdir 完成，retro.md 第一次没写成；下一回合补写并 amend 推送。今后涉及"目录必须先存在"的 create 一律分两个回合。

- **数字**
  - 文件数：5 个 src + 5 个 tests + 1 个 .test.md + 1 个 retro
  - LOC：约 700（含测试）
  - 增量编译：< 3 s
