---
applyTo: "**/*.rs"
---

# Rust Style Rules — EasyEnglish

1. **Error Handling**: Use `thiserror::Error` for module-specific errors (`DictError`, `StorageError`). Never `panic!`, `unwrap()`, or return `Box<dyn Error>` in library code.
2. **Layering Discipline**: No UI, OS, or Network dependencies in `Dict/` or `Core/`. Keep them pure Rust.
3. **Documentation**: Every public item must have a `///` doc comment summary.
4. **Testing**: Every change to `src/` must have matching Rust test code under `Tests/UnitTest/`. Do not add module-local `tests/` directories or inline `#[cfg(test)] mod tests`.
