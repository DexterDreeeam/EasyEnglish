// Win32 implementation of IPlatformShell.
//
// Responsibilities:
//   1. Single-instance: named mutex so double-clicking the tray icon doesn't
//      start a second process.
//   2. Tray icon: message-only window owns a Shell_NotifyIconW with a context
//      menu (Show, Quit). Tray click dispatches to caller-supplied callbacks.
//   3. Global hotkey: low-level mouse hook (WH_MOUSE_LL) that fires the
//      "show overlay" callback when Ctrl+Shift+WheelUp is seen. RegisterHotKey
//      can't bind mouse wheel, hence the hook approach.
//   4. Foreground-window save/restore: GetForegroundWindow() before show,
//      SetForegroundWindow() on hide so the overlay never breaks the user's
//      typing flow.
//
// This file is the ONLY place in src/platform/** that pulls in <windows.h>.

#ifdef _WIN32

// clang-format off
#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <shellapi.h>
// clang-format on

#include <atomic>
#include <cstdio>
#include <memory>
#include <utility>

#include "platform/IPlatformShell.hpp"

namespace easyenglish::platform {

namespace {

constexpr wchar_t kSingleInstanceMutex[] = L"Local\\EasyEnglish-SingleInstance";
constexpr wchar_t kTrayWindowClass[] = L"EasyEnglishTrayWindow";

constexpr UINT WM_TRAY_ICON = WM_APP + 1;
constexpr UINT WM_TRAY_SHOW = WM_APP + 2;
constexpr UINT WM_TRAY_QUIT = WM_APP + 3;

constexpr UINT_PTR kTrayIconId = 1;
constexpr UINT_PTR kTrayMenuShow = 1001;
constexpr UINT_PTR kTrayMenuQuit = 1002;

class Win32Shell final : public IPlatformShell {
public:
    Win32Shell() = default;
    ~Win32Shell() override {
        if (tray_added_) {
            NOTIFYICONDATAW nid{};
            nid.cbSize = sizeof(nid);
            nid.hWnd = msg_hwnd_;
            nid.uID = kTrayIconId;
            Shell_NotifyIconW(NIM_DELETE, &nid);
        }
        if (mouse_hook_ != nullptr) {
            UnhookWindowsHookEx(mouse_hook_);
        }
        if (msg_hwnd_ != nullptr) {
            DestroyWindow(msg_hwnd_);
        }
        if (mutex_ != nullptr) {
            CloseHandle(mutex_);
        }
    }

    bool acquireSingleInstance() override {
        mutex_ = CreateMutexW(nullptr, TRUE, kSingleInstanceMutex);
        if (mutex_ == nullptr) {
            return true;  // best-effort: assume single instance
        }
        if (GetLastError() == ERROR_ALREADY_EXISTS) {
            CloseHandle(mutex_);
            mutex_ = nullptr;
            return false;
        }
        return true;
    }

    void installTray(Callback on_left_click, Callback on_quit) override {
        on_left_click_ = std::move(on_left_click);
        on_quit_ = std::move(on_quit);
        ensureMessageWindow();
        addTrayIcon();
    }

    void installGlobalHotkey(Callback on_hotkey) override {
        on_hotkey_ = std::move(on_hotkey);
        // s_instance is consulted by the hook thunk; mouse_hook_ is process-wide
        // but the WH_MOUSE_LL hook actually fires on the thread that installed it.
        s_instance.store(this, std::memory_order_release);
        mouse_hook_ = SetWindowsHookExW(WH_MOUSE_LL, &Win32Shell::lowLevelMouseProc,
                                        GetModuleHandleW(nullptr), 0);
        if (mouse_hook_ == nullptr) {
            std::fprintf(stderr, "[platform] SetWindowsHookEx failed: %lu\n", GetLastError());
        }
    }

    void pump() override {
        MSG msg;
        while (PeekMessageW(&msg, nullptr, 0, 0, PM_REMOVE)) {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    void captureForegroundWindow() override { prev_foreground_ = GetForegroundWindow(); }

    void restoreForegroundWindow() override {
        if (prev_foreground_ != nullptr && IsWindow(prev_foreground_)) {
            SetForegroundWindow(prev_foreground_);
        }
        prev_foreground_ = nullptr;
    }

private:
    void ensureMessageWindow() {
        if (msg_hwnd_ != nullptr)
            return;

        WNDCLASSEXW wc{};
        wc.cbSize = sizeof(wc);
        wc.lpfnWndProc = &Win32Shell::wndProcThunk;
        wc.hInstance = GetModuleHandleW(nullptr);
        wc.lpszClassName = kTrayWindowClass;
        RegisterClassExW(&wc);

        msg_hwnd_ = CreateWindowExW(0, kTrayWindowClass, L"EasyEnglish", 0, 0, 0, 0, 0,
                                    HWND_MESSAGE, nullptr, GetModuleHandleW(nullptr), this);
        if (msg_hwnd_ == nullptr) {
            std::fprintf(stderr, "[platform] CreateWindowEx failed: %lu\n", GetLastError());
        }
    }

    void addTrayIcon() {
        NOTIFYICONDATAW nid{};
        nid.cbSize = sizeof(nid);
        nid.hWnd = msg_hwnd_;
        nid.uID = kTrayIconId;
        nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        nid.uCallbackMessage = WM_TRAY_ICON;
        // IDI_APPLICATION expands to a `LPCSTR`-typed MAKEINTRESOURCE; the W
        // variant needs the wide form, which the system also provides.
        nid.hIcon = LoadIconW(nullptr, MAKEINTRESOURCEW(32512));  // OIC_SAMPLE / IDI_APPLICATION
        lstrcpynW(nid.szTip, L"EasyEnglish — Ctrl+Shift+WheelUp", ARRAYSIZE(nid.szTip));

        tray_added_ = Shell_NotifyIconW(NIM_ADD, &nid) == TRUE;
        if (!tray_added_) {
            std::fprintf(stderr, "[platform] Shell_NotifyIcon NIM_ADD failed: %lu\n",
                         GetLastError());
        }
    }

    static LRESULT CALLBACK wndProcThunk(HWND hwnd, UINT msg, WPARAM w, LPARAM l) {
        Win32Shell* self = nullptr;
        if (msg == WM_CREATE) {
            auto* cs = reinterpret_cast<CREATESTRUCTW*>(l);
            self = static_cast<Win32Shell*>(cs->lpCreateParams);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, reinterpret_cast<LONG_PTR>(self));
        } else {
            self = reinterpret_cast<Win32Shell*>(GetWindowLongPtrW(hwnd, GWLP_USERDATA));
        }
        if (self != nullptr) {
            return self->wndProc(hwnd, msg, w, l);
        }
        return DefWindowProcW(hwnd, msg, w, l);
    }

    LRESULT wndProc(HWND hwnd, UINT msg, WPARAM w, LPARAM l) {
        switch (msg) {
            case WM_TRAY_ICON: {
                const UINT event = LOWORD(l);
                if (event == WM_LBUTTONUP) {
                    if (on_left_click_)
                        on_left_click_();
                } else if (event == WM_RBUTTONUP) {
                    showContextMenu(hwnd);
                }
                return 0;
            }
            case WM_COMMAND: {
                const UINT id = LOWORD(w);
                if (id == kTrayMenuShow && on_left_click_) {
                    on_left_click_();
                } else if (id == kTrayMenuQuit && on_quit_) {
                    on_quit_();
                }
                return 0;
            }
            default:
                return DefWindowProcW(hwnd, msg, w, l);
        }
    }

    void showContextMenu(HWND hwnd) {
        POINT pt;
        GetCursorPos(&pt);
        HMENU menu = CreatePopupMenu();
        AppendMenuW(menu, MF_STRING, kTrayMenuShow, L"Show overlay");
        AppendMenuW(menu, MF_SEPARATOR, 0, nullptr);
        AppendMenuW(menu, MF_STRING, kTrayMenuQuit, L"Quit");
        SetForegroundWindow(hwnd);  // required so the menu can be dismissed by clicking away
        TrackPopupMenu(menu, TPM_RIGHTBUTTON, pt.x, pt.y, 0, hwnd, nullptr);
        PostMessageW(hwnd, WM_NULL, 0, 0);
        DestroyMenu(menu);
    }

    static LRESULT CALLBACK lowLevelMouseProc(int code, WPARAM wParam, LPARAM lParam) {
        auto* self = s_instance.load(std::memory_order_acquire);
        if (code == HC_ACTION && self != nullptr && wParam == WM_MOUSEWHEEL) {
            const auto* info = reinterpret_cast<MSLLHOOKSTRUCT*>(lParam);
            const short wheel_delta = static_cast<short>(HIWORD(info->mouseData));
            const bool ctrl = (GetAsyncKeyState(VK_CONTROL) & 0x8000) != 0;
            const bool shift = (GetAsyncKeyState(VK_SHIFT) & 0x8000) != 0;
            if (wheel_delta > 0 && ctrl && shift && self->on_hotkey_) {
                self->on_hotkey_();
                // Swallow the event so Ctrl+Wheel doesn't also trigger
                // zoom in whatever app is underneath.
                return 1;
            }
        }
        return CallNextHookEx(nullptr, code, wParam, lParam);
    }

    HANDLE mutex_ = nullptr;
    HWND msg_hwnd_ = nullptr;
    HHOOK mouse_hook_ = nullptr;
    bool tray_added_ = false;
    HWND prev_foreground_ = nullptr;
    Callback on_left_click_;
    Callback on_quit_;
    Callback on_hotkey_;

    static inline std::atomic<Win32Shell*> s_instance{nullptr};
};

}  // namespace

std::unique_ptr<IPlatformShell> makePlatformShell() {
    return std::make_unique<Win32Shell>();
}

}  // namespace easyenglish::platform

#endif  // _WIN32
