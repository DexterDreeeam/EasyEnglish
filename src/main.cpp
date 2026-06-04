// `std::getenv` triggers MSVC's secure-CRT deprecation warning, which we treat
// as an error globally. The standard function is the right tool here.
#define _CRT_SECURE_NO_WARNINGS

#ifdef _WIN32
#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#endif

#include <atomic>
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <memory>
#include <string>

#include <GLFW/glfw3.h>
#include <imgui.h>
#include <imgui_impl_glfw.h>
#include <imgui_impl_opengl3.h>

#include "app/AppState.hpp"
#include "core/dictionary/ApiDictionary.hpp"
#include "core/dictionary/SqliteDictionary.hpp"
#include "core/favorites/FavoritesStore.hpp"
#include "core/history/HistoryStore.hpp"
#include "core/network/HttpNetworkClient.hpp"
#include "core/storage/Database.hpp"
#include "platform/IPlatformShell.hpp"
#include "ui/MainView.hpp"

namespace {

constexpr int kOverlayWidth = 520;
constexpr int kOverlayHeight = 320;

void glfwErrorCallback(int code, const char* desc) {
    std::fprintf(stderr, "GLFW error %d: %s\n", code, desc);
}

std::filesystem::path executableDir() {
#ifdef _WIN32
    wchar_t buf[1024]{};
    if (GetModuleFileNameW(nullptr, buf, ARRAYSIZE(buf)) > 0) {
        return std::filesystem::path(buf).parent_path();
    }
#endif
    return std::filesystem::current_path();
}

std::filesystem::path locateDictionary() {
    const auto next_to_exe = executableDir() / "mini_dict.sqlite";
    if (std::filesystem::exists(next_to_exe)) {
        return next_to_exe;
    }
#ifdef EASYENGLISH_FIXTURES_DIR
    const auto fixture = std::filesystem::path(EASYENGLISH_FIXTURES_DIR) / "mini_dict.sqlite";
    if (std::filesystem::exists(fixture)) {
        return fixture;
    }
#endif
    return {};
}

std::filesystem::path userDataPath(const std::string& filename) {
#ifdef _WIN32
    if (const char* appdata = std::getenv("APPDATA"); appdata != nullptr) {
        return std::filesystem::path(appdata) / "EasyEnglish" / filename;
    }
#endif
    if (const char* home = std::getenv("HOME"); home != nullptr) {
        return std::filesystem::path(home) / ".easyenglish" / filename;
    }
    return executableDir() / filename;
}

void loadFonts(ImGuiIO& io) {
    const auto fonts_dir = executableDir() / "fonts";
    const auto latin = fonts_dir / "NotoSans-Regular.ttf";
    const auto cjk = fonts_dir / "NotoSansSC-Regular.otf";

    bool any_loaded = false;
    if (std::filesystem::exists(latin)) {
        io.Fonts->AddFontFromFileTTF(latin.string().c_str(), 18.0f);
        any_loaded = true;
    }
    if (std::filesystem::exists(cjk)) {
        ImFontConfig cfg;
        cfg.MergeMode = any_loaded;
        cfg.PixelSnapH = true;
        // Use ImGui's built-in CJK range so we don't ship a font subset; the
        // tradeoff is a larger atlas (~30 MB texture) but startup stays fast.
        io.Fonts->AddFontFromFileTTF(cjk.string().c_str(), 18.0f, &cfg,
                                     io.Fonts->GetGlyphRangesChineseFull());
        any_loaded = true;
    }
    if (!any_loaded) {
        std::fprintf(stderr,
                     "[main] No bundled font found under %s — falling back to "
                     "ImGui default font (CJK characters will render as boxes).\n",
                     fonts_dir.string().c_str());
    }
}

void centerOnCursorMonitor(GLFWwindow* window, int width, int height) {
    int monitor_count = 0;
    GLFWmonitor** monitors = glfwGetMonitors(&monitor_count);
    if (monitors == nullptr || monitor_count == 0) {
        return;
    }
    GLFWmonitor* target = monitors[0];
    int tx = 0;
    int ty = 0;
    int tw = 0;
    int th = 0;
    glfwGetMonitorPos(target, &tx, &ty);
    if (const GLFWvidmode* mode = glfwGetVideoMode(target); mode != nullptr) {
        tw = mode->width;
        th = mode->height;
    }
#ifdef _WIN32
    POINT pt;
    if (GetCursorPos(&pt) != 0) {
        for (int i = 0; i < monitor_count; ++i) {
            int mx = 0;
            int my = 0;
            glfwGetMonitorPos(monitors[i], &mx, &my);
            const GLFWvidmode* mode = glfwGetVideoMode(monitors[i]);
            if (mode == nullptr)
                continue;
            if (pt.x >= mx && pt.x < mx + mode->width && pt.y >= my && pt.y < my + mode->height) {
                target = monitors[i];
                tx = mx;
                ty = my;
                tw = mode->width;
                th = mode->height;
                break;
            }
        }
    }
#endif
    if (tw == 0 || th == 0)
        return;
    glfwSetWindowPos(window, tx + (tw - width) / 2, ty + (th - height) / 3);
}

}  // namespace

int main(int /*argc*/, char* /*argv*/[]) {
    using namespace easyenglish;

    // ---- Platform shell (tray, hotkey, single-instance, focus) -----------
    auto shell = platform::makePlatformShell();
    if (!shell->acquireSingleInstance()) {
        std::fprintf(stderr, "Another instance of EasyEnglish is already running.\n");
        return 0;
    }

    // ---- GLFW + ImGui initialization -------------------------------------
    glfwSetErrorCallback(glfwErrorCallback);
    if (glfwInit() == 0) {
        std::fprintf(stderr, "Failed to initialize GLFW\n");
        return 1;
    }
    glfwWindowHint(GLFW_CONTEXT_VERSION_MAJOR, 3);
    glfwWindowHint(GLFW_CONTEXT_VERSION_MINOR, 3);
    glfwWindowHint(GLFW_OPENGL_PROFILE, GLFW_OPENGL_CORE_PROFILE);
    glfwWindowHint(GLFW_OPENGL_FORWARD_COMPAT, GLFW_TRUE);
    // Overlay-style window: undecorated, always-on-top, hidden by default —
    // only shown when the global hotkey fires.
    glfwWindowHint(GLFW_DECORATED, GLFW_FALSE);
    glfwWindowHint(GLFW_RESIZABLE, GLFW_FALSE);
    glfwWindowHint(GLFW_FLOATING, GLFW_TRUE);
    glfwWindowHint(GLFW_VISIBLE, GLFW_FALSE);
    glfwWindowHint(GLFW_FOCUS_ON_SHOW, GLFW_TRUE);

    GLFWwindow* window =
        glfwCreateWindow(kOverlayWidth, kOverlayHeight, "EasyEnglish", nullptr, nullptr);
    if (window == nullptr) {
        std::fprintf(stderr, "Failed to create GLFW window\n");
        glfwTerminate();
        return 1;
    }
    glfwMakeContextCurrent(window);
    glfwSwapInterval(1);

    IMGUI_CHECKVERSION();
    ImGui::CreateContext();
    ImGuiIO& io = ImGui::GetIO();
    io.IniFilename = nullptr;  // overlay layout is ephemeral; don't persist
    io.ConfigFlags |= ImGuiConfigFlags_NavEnableKeyboard;
    loadFonts(io);
    ImGui::StyleColorsDark();
    ImGui_ImplGlfw_InitForOpenGL(window, true);
    ImGui_ImplOpenGL3_Init("#version 330");

    // ---- Wire core dependencies -----------------------------------------
    std::shared_ptr<core::dictionary::IDictionary> local;
    if (const auto dict_path = locateDictionary(); !dict_path.empty()) {
        auto db = core::storage::Database::open(dict_path);
        if (db.has_value()) {
            auto sqlite_dict = core::dictionary::SqliteDictionary::open(std::move(db.value()));
            if (sqlite_dict.has_value()) {
                local = std::make_shared<core::dictionary::SqliteDictionary>(
                    std::move(sqlite_dict.value()));
            }
        }
    }

    auto net = std::make_shared<core::network::HttpNetworkClient>(5000);
    auto online = std::make_shared<core::dictionary::ApiDictionary>(net);

    std::shared_ptr<core::history::HistoryStore> history;
    if (auto path = userDataPath("history.sqlite"); !path.empty()) {
        auto db = core::storage::Database::createOrOpen(path);
        if (db.has_value()) {
            auto store = core::history::HistoryStore::open(std::move(db.value()));
            if (store.has_value()) {
                history = std::make_shared<core::history::HistoryStore>(std::move(store.value()));
            }
        }
    }

    std::shared_ptr<core::favorites::FavoritesStore> favorites;
    if (auto path = userDataPath("favorites.sqlite"); !path.empty()) {
        auto db = core::storage::Database::createOrOpen(path);
        if (db.has_value()) {
            auto store = core::favorites::FavoritesStore::open(std::move(db.value()));
            if (store.has_value()) {
                favorites =
                    std::make_shared<core::favorites::FavoritesStore>(std::move(store.value()));
            }
        }
    }

    app::AppState state(local, online, history, favorites);

    // ---- Shell callbacks -------------------------------------------------
    std::atomic<bool> show_request{false};
    std::atomic<bool> quit_request{false};
    shell->installTray(/*on_left_click=*/[&] { show_request.store(true); },
                       /*on_quit=*/[&] { quit_request.store(true); });
    shell->installGlobalHotkey([&] { show_request.store(true); });

    bool visible = false;
    bool just_shown = false;
    auto showOverlay = [&]() {
        shell->captureForegroundWindow();
        state.reset();
        centerOnCursorMonitor(window, kOverlayWidth, kOverlayHeight);
        glfwShowWindow(window);
        glfwFocusWindow(window);
        visible = true;
        just_shown = true;
    };
    auto hideOverlay = [&]() {
        glfwHideWindow(window);
        visible = false;
        shell->restoreForegroundWindow();
    };

    // ---- Main loop -------------------------------------------------------
    while (!quit_request.load() && glfwWindowShouldClose(window) == 0) {
        shell->pump();

        if (show_request.exchange(false)) {
            if (!visible) {
                showOverlay();
            } else {
                just_shown = true;
                glfwFocusWindow(window);
            }
        }

        if (visible) {
            glfwPollEvents();

            ImGui_ImplOpenGL3_NewFrame();
            ImGui_ImplGlfw_NewFrame();
            ImGui::NewFrame();

            const bool dismiss = ui::MainView::render(state, just_shown);
            just_shown = false;

            ImGui::Render();
            int display_w = 0;
            int display_h = 0;
            glfwGetFramebufferSize(window, &display_w, &display_h);
            glViewport(0, 0, display_w, display_h);
            glClearColor(0.08f, 0.08f, 0.10f, 1.0f);
            glClear(GL_COLOR_BUFFER_BIT);
            ImGui_ImplOpenGL3_RenderDrawData(ImGui::GetDrawData());
            glfwSwapBuffers(window);

            if (dismiss) {
                hideOverlay();
            }
        } else {
            // Tray/hotkey-only state. Wait briefly so we don't spin the CPU
            // while the user isn't looking at us.
            glfwWaitEventsTimeout(0.05);
        }
    }

    // ---- Shutdown --------------------------------------------------------
    if (visible) {
        hideOverlay();
    }
    ImGui_ImplOpenGL3_Shutdown();
    ImGui_ImplGlfw_Shutdown();
    ImGui::DestroyContext();
    glfwDestroyWindow(window);
    glfwTerminate();
    return 0;
}
