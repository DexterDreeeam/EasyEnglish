# iter-001 Retrospective

- **What shipped**: src/core/storage Database/Statement RAII wrapper backed by SQLite (vcpkg `sqlite3`), 19/19 ctest green on CI ([run #26930920779](https://github.com/DexterDreeeam/EasyEnglish/actions/runs/26930920779)), tests/fixtures/mini_dict.sqlite seeded from `tools/seed_db.py`, contract `docs/contracts/storage.md` frozen.

- **AI走偏**: 全局 `CMAKE_AUTOMOC ON` + 在子目录 CMakeLists 里追加 `target_link_libraries(easyenglish_core PRIVATE unofficial::sqlite3::sqlite3)` 触发 CMake 的 `_autogen_timestamp_deps does not exist` 错误 —— 因为 autogen target 在 sqlite3 import target 创建之前就被生成。修复方式是把 AUTOMOC 改为 per-target（仅 ui 模块开启），core 是纯逻辑层根本不需要 moc。

- **下次保留**：本地 clang-format `-i` 一遍再 commit；fixture 的"期望计数"作为常量写在测试里并配一条人类可读的错误消息提醒同步 `seed_db.py`，避免 AI 偷改期望值。

- **下次改变**：每加一个 core 子模块时立即检查 CMake target 是否依赖在子目录中后注册的 import target；规划阶段就把"AUTOMOC 仅 UI 层"写进 AGENTS.md（已下一次 iter 顺手更新）。
