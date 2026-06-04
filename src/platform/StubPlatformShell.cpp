// Stub PlatformShell for non-Windows builds. Lets the rest of the app
// compile on Linux/macOS CI runners (we currently don't ship there, but
// keeping the build green on these platforms simplifies future cross-port).

#ifndef _WIN32

#include <cstdio>

#include "platform/IPlatformShell.hpp"

namespace easyenglish::platform {

namespace {

class StubShell final : public IPlatformShell {
public:
    bool acquireSingleInstance() override { return true; }
    void installTray(Callback /*on_left*/, Callback /*on_quit*/) override {
        std::fprintf(stderr, "[platform] tray not implemented on this OS — skipped\n");
    }
    void installGlobalHotkey(Callback /*on_hotkey*/) override {
        std::fprintf(stderr, "[platform] global hotkey not implemented on this OS — skipped\n");
    }
    void pump() override {}
    void captureForegroundWindow() override {}
    void restoreForegroundWindow() override {}
};

}  // namespace

std::unique_ptr<IPlatformShell> makePlatformShell() {
    return std::make_unique<StubShell>();
}

}  // namespace easyenglish::platform

#endif  // !_WIN32
