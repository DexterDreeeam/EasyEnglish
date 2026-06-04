# `platform` Contract

**Source path**: `src/platform/`
**Owner test path**: (integration-only; no unit tests yet — see §4)
**Status**: frozen (since iter-011)

## 1. Public API (FROZEN — change requires ADR)

```cpp
namespace easyenglish::platform {

class IPlatformShell {
public:
    using Callback = std::function<void()>;

    virtual bool acquireSingleInstance() = 0;
    virtual void installTray(Callback on_left_click, Callback on_quit) = 0;
    virtual void installGlobalHotkey(Callback on_hotkey) = 0;
    virtual void pump() = 0;
    virtual void captureForegroundWindow() = 0;
    virtual void restoreForegroundWindow() = 0;
};

std::unique_ptr<IPlatformShell> makePlatformShell();

}
```

## 2. Invariants

- `acquireSingleInstance()` is called exactly once at process start. If it
  returns `false`, the host should exit cleanly with code 0.
- `installTray()` / `installGlobalHotkey()` may be called only once. Callbacks
  are invoked on the platform thread that owns the message loop (on Windows,
  the main thread that called `pump()`).
- `pump()` is non-blocking and called every iteration of the main loop.
- `captureForegroundWindow()` must be called BEFORE the overlay grabs focus;
  `restoreForegroundWindow()` is called when the overlay hides.
- Non-Windows builds get a stub implementation: every method is a no-op,
  `acquireSingleInstance()` returns true. This lets non-Win CI compile.

## 3. Dependencies

- Allowed: `<windows.h>` (only in `win32/Win32PlatformShell.cpp` behind
  `#ifdef _WIN32`), standard library.
- Forbidden: ImGui, GLFW, `core/**`, `app/**` headers. The platform shell is
  oblivious to UI; it only exposes callbacks.

## 4. Testing strategy

- The Win32 implementation depends on shell32 / user32 APIs that can't be
  exercised in a CI Job's headless session without significant effort.
  iter-011 ships **integration tests only**: CI proves the source compiles
  and links; a human runs the installed exe to verify the tray + hotkey.
- A future iteration may add `tests/platform/test_fake_shell.cpp` driving
  an `IPlatformShell` test double through `AppState`, but the production
  shell will remain manually validated.

## 5. Change log

- 2026-06-04 — iter-011: initial implementation + frozen. Windows-only:
  named-mutex single instance, `Shell_NotifyIconW` tray (Left click = show,
  Right click = context menu with Show/Quit), `WH_MOUSE_LL` low-level hook
  for Ctrl+Shift+WheelUp, GetForegroundWindow/SetForegroundWindow for
  focus restoration. Stub shell on non-Win for build portability.
