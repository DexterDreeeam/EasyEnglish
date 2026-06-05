---
applyTo: "**/*.rs"
---

# Rust style — EasyEnglish

These rules apply to every `*.rs` file in the workspace. They complement, not
replace, `AGENTS.md` and `.clippy.toml` / `.rustfmt.toml`.

## Error handling

- Public functions in `src/core/**` and `src/<module>/lib.rs` return
  `Result<T, ModuleError>` where `ModuleError` is a `thiserror::Error` enum
  named for that module (`DictError`, `ConfigError`, `LookupError`, ...).
- Never panic in library code. Reserve `panic!` / `unreachable!` /
  `unwrap()` for `tests/` and `examples/`.
- Avoid `Box<dyn std::error::Error>`. Wrap in the module's typed enum.

## Naming

- Crate namespace: `easyenglish::<module>` if/when we add re-exports.
  Currently each crate is its own root (`ee_dict::`, `ee_core::`, ...).
- Types in PascalCase, functions / fields in snake_case, constants in
  SCREAMING_SNAKE_CASE.
- Test functions named `<unit>_<scenario>_<expected>` (see existing
  `Core/tests/test_*.rs`).

## Headers

- Every public item gets a `///` doc comment with one line of summary, blank
  line, then a short paragraph if needed.
- No `use` of `unwrap_or_default` for `Option<&str>` where the empty string
  semantically conflicts with "missing".

## Forbidden in `src/core/**` and `src/dict/**`

- Any `use` of `winit`, `egui`, `glfw`, `imgui`, `iced`, `tao`, `gtk`, `tauri`,
  `windows`, `windows-sys`, `cocoa`, `core-graphics`, `objc`, `x11`, `wayland-*`.
- Any `tokio` / `async-std` / `futures` (Phase 1 is sync everywhere).
- `reqwest` / `ureq` / `hyper` / `actix-*` / `axum` (no network in core).
- Direct calls to `std::env::set_*` (use Config).
- `unsafe` blocks (none of these modules need them; if you think you do,
  open an ADR first).

## Testing

- Integration tests live in `<Module>/tests/test_*.rs`.
- In-crate unit tests live in `<module>/src/*.rs` under `#[cfg(test)] mod tests`.
- Every test name appears in `<Module>/tests/.test.md`. CI (when we have it)
  will diff the two and fail on drift.
- Use `tempfile::tempdir()` for any test that touches the filesystem; never
  hard-code paths under `C:\` or `/tmp`.

## Cargo

- New deps must come from `[workspace.dependencies]` in the root `Cargo.toml`.
  No version pins in individual crates.
- Features ship behind `default-features = false` then explicitly opted in
  per crate.
