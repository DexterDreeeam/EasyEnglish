---
applyTo: "**/*.rs"
---

# Rust Style Rules — EasyEnglish

1. **Error Handling**: Use `thiserror::Error` for module-specific errors (`DictError`, `StorageError`). Never `panic!`, `unwrap()`, or return `Box<dyn Error>` in library code.
2. **Layering Discipline**: No UI, OS, or Network dependencies in `ee/Dict/` or `ee/Core/`. Keep them pure Rust.
3. **Documentation**: Every public item must have a `///` doc comment summary.
4. **Testing**: Every change to `ee/**/src/` must have matching Rust test code under `ee/Tests/UnitTest/`. Do not add module-local `tests/` directories or inline `#[cfg(test)] mod tests`.
