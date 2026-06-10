# EasyEnglish

> Cross-platform English → Chinese instant translator, built with Rust + a cargo workspace.

EasyEnglish is an ambient workflow tool: a background-resident daemon summoned by a
global hotkey that pops up a floating translation box. Type an English word and instantly
get its Chinese definition (from the bundled offline dictionary), and attach an arbitrary
personal *Note* to any word (English → any content).

> This repository is the Rust rewrite. The earlier Qt → ImGui C++ implementation lives at
> the git `v0.3.0` tag (`git checkout v0.3.0`). The rewrite rationale is in
> `docs/adr/0004-rewrite-in-rust-m2a-style.md`.

## Status

**Phase 1** (current): the two core modules `Dict` and `Core` are complete.
**Phase 2** (later): fill in the `Win` / `Mac` / `Linux` platform layers
(tray + global hotkey + overlay + installer).

## Repository Layout

The layout follows [m2a](https://github.com/DexterDreeeam/M2A): every module has a
`.design.md` (architecture + sequence diagrams) and a `.interface.md` (Public/Private API),
cross-linked with ⬆️/⬇️.

```
EasyEnglish/
├── Cargo.toml           # cargo workspace
├── product.json         # runtime configuration (read by the Core module)
├── .design.md           # root design (links to each module)
├── .interface.md        # root interface (links to each module)
├── AGENTS.md            # AI collaboration constitution
├── Dict/                # offline dictionary data + SQLite access layer
│   ├── src/  data/  tests/
│   ├── .design.md  .interface.md
├── Core/                # config / history / Note / Lookup / AppState
│   ├── src/  tests/
│   ├── .design.md  .interface.md
├── App/   Win/  Mac/  Linux/   # platform layer (Phase 2 placeholders; each has .design.md / .interface.md)
└── docs/adr/            # architecture decision records
```

## Local Development

### One-time setup

Requires Rust stable (≥ 1.83):
```powershell
# https://rustup.rs/
rustup default stable
cargo install cargo-nextest --locked
```

### Everyday commands

```powershell
cargo build --workspace
cargo nextest run --workspace --no-tests=pass
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

> The repo-root `.cargo/config.toml` already enables the `rust-lld` linker +
> `split-debuginfo` by default; a full build takes ~90 s, incremental changes < 3 s.

### No CI

By design, this repository does **not** use GitHub Actions — all quality gates are run
locally by the developer via the commands above. If CI is added later, a 30-line
`.github/workflows/ci.yml` is enough.

## Contributing

Before any change, **first** read:
1. The root `.design.md` and `.interface.md`
2. The `.design.md` and `.interface.md` of the module you are changing
3. `AGENTS.md` (modification constraints)

Test conventions are in `<Module>/tests/.test.md`.

## License

MIT — see `LICENSE`.
