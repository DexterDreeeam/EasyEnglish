#pragma once

#include <functional>
#include <memory>

namespace easyenglish::platform {

/// Cross-platform shell-integration interface. Concrete impl on Windows:
/// `Win32PlatformShell` (tray icon + low-level mouse hook + foreground-
/// window save/restore + single-instance mutex).
///
/// The main loop owns one IPlatformShell and pumps `pump()` each frame so
/// shell events (e.g. tray icon click, global hotkey) can be delivered as
/// callbacks without giving the platform thread access to AppState directly.
class IPlatformShell {
public:
    using Callback = std::function<void()>;

    IPlatformShell() = default;
    IPlatformShell(const IPlatformShell&) = delete;
    IPlatformShell& operator=(const IPlatformShell&) = delete;
    IPlatformShell(IPlatformShell&&) = delete;
    IPlatformShell& operator=(IPlatformShell&&) = delete;
    virtual ~IPlatformShell() = default;

    /// Returns false if another instance of the app is already running and
    /// this one should exit immediately. Implementations may take a named
    /// system mutex here.
    [[nodiscard]] virtual bool acquireSingleInstance() = 0;

    /// Install tray icon. `on_left_click` typically maps to "show overlay";
    /// `on_quit` to "shut down app".
    virtual void installTray(Callback on_left_click, Callback on_quit) = 0;

    /// Install a global mouse hook that invokes `on_hotkey` whenever the
    /// configured Ctrl+Shift+WheelUp combination fires.
    virtual void installGlobalHotkey(Callback on_hotkey) = 0;

    /// Drain platform messages once per frame. Must be cheap and non-blocking.
    virtual void pump() = 0;

    /// Cache the foreground window so it can later be restored when the
    /// overlay hides. Implementations should grab it BEFORE the overlay
    /// requests focus.
    virtual void captureForegroundWindow() = 0;

    /// Restore focus to whatever `captureForegroundWindow()` last captured.
    virtual void restoreForegroundWindow() = 0;
};

/// Construct the platform-specific shell. On non-Windows builds this
/// returns a stub that no-ops everything; on Windows it returns the real
/// Win32 implementation.
std::unique_ptr<IPlatformShell> makePlatformShell();

}  // namespace easyenglish::platform
