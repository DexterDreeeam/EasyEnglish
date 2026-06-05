---
name: rust-quality-gate
description: Run the canonical 4-step quality gate for EasyEnglish — build, nextest, fmt-check, clippy. Use when the user says "run the gate", "check quality", "ready to commit?", or after any non-trivial Rust change.
allowed-tools: shell
---

# Rust Quality Gate

Run the four-step canonical quality gate for EasyEnglish. **Do not invent
shorter variants** (no "I'll skip clippy for now"). Either all four pass or
the gate fails.

## Pre-flight (Windows / PowerShell)

Cargo is not on `$env:PATH` on the dev machine. **Always** prepend:

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
```

Then verify `cargo --version` and `cargo nextest --version` are both available.
If `cargo-nextest` is missing, ask the user before installing.

## The four commands (in order, fail-fast)

```powershell
# 1. Compile the whole workspace (proves everything still type-checks)
cargo build --workspace

# 2. Run the full test suite via nextest (3-10x faster than cargo test)
cargo nextest run --workspace --no-tests=pass

# 3. Format check (do NOT auto-fix; if it fails, run `cargo fmt --all` and re-gate)
cargo fmt --all --check

# 4. Clippy as a hard linter (warnings = failures)
cargo clippy --workspace --all-targets -- -D warnings
```

If a step fails, **stop**. Report the failing step + its full output. Do not
continue to subsequent steps.

## Optional, on user request only

- **Module-scoped tests:** replace step 2 with `cargo nextest run -p ee-<module>`.
- **With `cargo fmt --all` auto-fix:** ONLY if user explicitly asks. Then re-gate from step 1.
- **`cargo doc --workspace --no-deps`:** only when the user asks "does the docs build?".

## Reporting

Print a one-line per step, then a final verdict:

```
✅ build:   workspace, 5 crates, finished in 1.32s
✅ test:    54 tests passed in 0.551s
✅ fmt:     clean
✅ clippy:  no warnings in 4 crates

Gate: PASS
```

Or on failure:

```
✅ build:   workspace, 5 crates, finished in 1.32s
❌ test:    1 failure in ee-core::test_lookup::case_insensitive_in_both_orderings
   <last 20 lines of test output>

Gate: FAIL at step 2. Stopped.
```

## Hard rules

- Never claim PASS without all 4 steps green.
- Never auto-commit, auto-push, or auto-fix without user permission.
- Never silence warnings via `#[allow(...)]` to make clippy pass — that's a
  code-review issue, not a gate issue.
- If `cargo-nextest` is missing, install only with explicit user consent:
  `cargo install cargo-nextest --locked`.
- If the user is on macOS or Linux, swap PowerShell preamble for:
  `export PATH="$HOME/.cargo/bin:$PATH"`. Otherwise commands are identical.
