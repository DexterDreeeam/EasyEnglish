# iter-002 Retrospective

- **What shipped**: `IDictionary` interface + `SqliteDictionary` (prepared-statement cache, mutex-guarded shared access). Definitions parsed via QJsonDocument from Qt6::Core — zero new deps. 7 unit tests + 2 benchmarks. CI 100% (run #26931245481). Lookup contract frozen.

- **AI走偏**: 几乎没有 — 在 iter-001 的 AUTOMOC 修复之后，模块叠加非常顺。唯一一处 PowerShell `New-Item` 与 `create` 工具并行竞态导致 task.md 第一次写失败（目录还没生成），重试解决。

- **下次保留**：每个 `core/<m>` 都通过 `target_sources(easyenglish_core PRIVATE ${CMAKE_CURRENT_LIST_DIR}/...)` 在 subdir CMakeLists 里追加 — 路径用绝对的 `${CMAKE_CURRENT_LIST_DIR}` 保证多层 add_subdirectory 也不会找错。
