# 0004. Rewrite: C++/Qt/ImGui → Rust + m2a-style cargo workspace

- **Status**: accepted
- **Date**: 2026-06-05
- **Supersedes**: [0001](./0001-choose-qt6.md), [0002](./0002-switch-to-imgui.md), [0003](./0003-tray-overlay.md)
  (0001-0003 describe the C++ implementation chain prior to v0.3.0. This ADR voids them as a whole.
  The original C++ implementation is preserved at the git `v0.3.0` tag; `git checkout v0.3.0` retrieves it.)

## Context

Up to v0.3.0, EasyEnglish was a C++23 / ImGui / GLFW + Win32 platform-layer desktop app.
Unit tests passed 70/70, and the installer shipped to GitHub Releases. The features were enough to ship.

But several structural problems remained hard to solve cheaply within the C++ implementation:

1. **Cross-platform consistency**: the Win32 platform layer used APIs like LowLevelMouseHook / Shell_NotifyIcon;
   the macOS / Linux equivalents all need rewriting. The Rust ecosystem has ready crates such as `tray-icon` / `global-hotkey` / `winit`,
   so cross-platform behavior is closer to free.
2. **Dependency management**: vcpkg + WiX + Inno Setup + the Qt action are complex to coordinate; the first CI run takes 20+ minutes.
   A cargo workspace + cargo-wix / cargo-packager is a whole order of magnitude simpler.
3. **Modular documentation**: the repo directory had clear `core/app/ui/platform` boundaries, but **no enforced
   per-module interface docs** — the public API of any module had to be confirmed by digging through `.hpp` files in the code.
4. **Test speed**: CMake + linking + AddressSanitizer ran the full test suite in ~7s, and with startup overhead
   a full run took 30 s+; Rust + nextest at the same scale is expected to be <10 s.

Explicit requirements the user raised on 2026-06-05:
- Rust rewrite
- Modular (cargo workspace)
- Fast local builds + tests
- Shippable installer
- Learn the design / interface doc conventions of `C:\r\m2a`
- Top-level modules: `Dict` / `Core` / `Win` / `Mac` / `Linux` (at least 4)
- For now focus only on `Dict` + `Core`
- Remove favorites, add **Note** (a runtime EN → arbitrary-content mapping)
- **No GitHub Actions / automated release**

## Considered options

1. **Keep C++, keep improving incrementally** — multi-platform hotkeys + documenting module interfaces are both achievable, but the effort
   is comparable to a full rewrite (~3000 LOC C++), and you still face the friction of CMake + vcpkg. **No.**
2. **Full rewrite in Rust + cargo workspace, m2a-style docs** ✅
   - 5 top-level crates (`Dict` / `Core` / `Win` / `Mac` / `Linux`)
   - Each module has `.design.md` + `.interface.md`; tests are centralized under `Tests/UnitTest` and `Tests/UITest`
   - For now implement only Dict + Core; leave Win/Mac/Linux as placeholder skeletons
   - Do not write `.github/workflows/`
3. **Rust + single crate + mod** — also possible, but mod cannot enforce interface boundaries, and the refactoring penalty is much
   lighter with crate boundaries. **No.**
4. **Rust rewrite + the 10-crate split from the earlier plan.md** — `ee-app` / `ee-ui` / `ee-bin`
   etc. are too granular; the current phase only cares about the data + logic layers, and the platform + UI work has not started.
   **Does not match this round of feedback.**

## Decision

Adopt **Option 2**.

The concrete directory layout, module responsibilities, and tech stack are in the root `.design.md`. This ADR only pins the direction.

## Consequences

- **Positive**
  - The cargo workspace + incremental compilation means any one change only recompiles 1-2 crates
  - `.design.md` + `.interface.md` give both AI and humans an explicit, readable contract
  - The 5 top-level crates pin "which platform does what" into the repo from the start, avoiding large layout changes when filling them in later
- **Costs / trade-offs**
  - Lose the already-released v0.3.0 installer (a user who needs the old version is told to download `v0.3.0` from Releases)
  - Lose the favorites feature (per the user's explicit request; partially covered by the equivalent need through Note)
  - Note is runtime data, cleared on restart (per the user's explicit request; if they change their mind later, just add a persist entry point,
    and the `NoteStore` interface will not break because of it)
  - No CI means fmt + clippy + nextest must be run locally before pushing to main; repository instructions state this
- **Impact on interfaces**
  - All the v0.3.0-era C++ classes (`Database` / `IDictionary` / `MainView` / `AppState` …)
    no longer exist. The Rust equivalents are redefined and frozen in each module's `.interface.md`.
- **Impact on testing**
  - The original per-crate `tests/` convention has been replaced by the centralized `Tests/UnitTest` Rust test crate.
  - UI automation specifications live separately as markdown under `Tests/UITest`.

## References

- The m2a repo (source of the design/interface doc conventions): `C:\r\m2a`
- The v0.3.0 release (reference behavior the rewrite starts from):
  https://github.com/DexterDreeeam/EasyEnglish/releases/tag/v0.3.0
- The earlier plan.md (superseded by this ADR): kept in session history
