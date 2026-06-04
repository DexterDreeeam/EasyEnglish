# Task: iter-001-storage — SQLite RAII wrapper

## Context (必读)

- **涉及模块**: `src/core/storage/`, `tests/unit/storage/`, `tools/seed_db.py`, `tests/fixtures/`
- **相关契约**: `docs/contracts/storage.md`（draft → frozen 在本迭代收尾）
- **相关 ADR**: 无（实现细节，没改架构决策）
- **依赖前置任务**: `iter-000-skeleton` ✅

## Out of scope

- 不实现 `dictionary` 模块（iter-002）
- 不引入 QtSql（契约明确禁止）
- 不实现 schema 迁移引擎（够用即可，单 schema 写死在 seed 中）

## Acceptance criteria

- [ ] `Database::open(":memory:")` 与文件路径都能成功；缺失文件返回 `StorageError::IoError`
- [ ] `execute("CREATE TABLE ...; INSERT INTO ...; SELECT ...")` 路径全过
- [ ] `prepare` + `bind` + `step` + 列访问可读出 INSERT 进去的数据
- [ ] 错误路径：非法 SQL → `InvalidQuery`；UNIQUE 冲突 → `ConstraintViolation`
- [ ] Move 语义正确（赋值后旧实例不会重复 close）
- [ ] `tests/fixtures/mini_dict.sqlite` 由 `tools/seed_db.py` 生成、提交进 git
- [ ] `ctest --preset msvc-debug -R Storage` 全部通过
- [ ] `python tools/check_core_no_ui.py` 通过
- [ ] CI windows-build + clang-format 全绿

## Implementation hints

- 用 `find_package(unofficial-sqlite3)` 或 `find_package(SQLite3)`（vcpkg port 名 `sqlite3`，CMake 模块名 `unofficial-sqlite3` / `SQLite::SQLite3`）
- 错误码映射：`SQLITE_CONSTRAINT* → ConstraintViolation`、`SQLITE_BUSY → Busy`、其余非 OK → `IoError`/`InvalidQuery`
- `Statement` 持有 `sqlite3_stmt*`，移动后置空，析构调 `sqlite3_finalize`
- 用 `std::expected` (C++23)，错误处理一致

## Definition of Done

- 新增 `src/core/storage/{Database,Statement,errors}.hpp/.cpp`
- 新增 `tests/unit/storage/test_database.cpp` 等
- 删除 `src/core/storage/` 下原 placeholder（如果有）和 `src/core/placeholder.cpp`（首个真实模块就位后即可移除）
- 更新 `docs/contracts/storage.md` 把 Status 改 frozen
- 写 `retro.md`
