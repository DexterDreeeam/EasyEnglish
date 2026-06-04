#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <memory>
#include <string>

// We rely on GLFW to pull in the platform's basic OpenGL header (GL 1.1 is
// enough for glClear / glClearColor / glViewport — anything richer is loaded
// by ImGui's bundled GL3 loader inside imgui_impl_opengl3.cpp).
#include <GLFW/glfw3.h>
#include <imgui.h>
#include <imgui_impl_glfw.h>
#include <imgui_impl_opengl3.h>

#include "app/AppState.hpp"
#include "core/dictionary/SqliteDictionary.hpp"
#include "core/favorites/FavoritesStore.hpp"
#include "core/history/HistoryStore.hpp"
#include "core/storage/Database.hpp"
#include "ui/MainView.hpp"

namespace {

void glfwErrorCallback(int code, const char* desc) {
    std::fprintf(stderr, "GLFW error %d: %s\n", code, desc);
}

std::filesystem::path executableDir() {
#ifdef _WIN32
    // GLFW provides a clean cross-platform way later; for now use std::filesystem.
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
        std::filesystem::path p = std::filesystem::path(appdata) / "EasyEnglish";
        return p / filename;
    }
#endif
    if (const char* home = std::getenv("HOME"); home != nullptr) {
        return std::filesystem::path(home) / ".easyenglish" / filename;
    }
    return executableDir() / filename;
}

}  // namespace

int main(int /*argc*/, char* /*argv*/[]) {
    using namespace easyenglish;

    // ---- Initialize GLFW + OpenGL context --------------------------------
    glfwSetErrorCallback(glfwErrorCallback);
    if (glfwInit() == 0) {
        std::fprintf(stderr, "Failed to initialize GLFW\n");
        return 1;
    }

    // Request OpenGL 3.3 core; ImGui's opengl3 backend's bundled loader handles it.
    glfwWindowHint(GLFW_CONTEXT_VERSION_MAJOR, 3);
    glfwWindowHint(GLFW_CONTEXT_VERSION_MINOR, 3);
    glfwWindowHint(GLFW_OPENGL_PROFILE, GLFW_OPENGL_CORE_PROFILE);
    glfwWindowHint(GLFW_OPENGL_FORWARD_COMPAT, GLFW_TRUE);

    GLFWwindow* window = glfwCreateWindow(960, 640, "EasyEnglish", nullptr, nullptr);
    if (window == nullptr) {
        std::fprintf(stderr, "Failed to create GLFW window\n");
        glfwTerminate();
        return 1;
    }
    glfwMakeContextCurrent(window);
    glfwSwapInterval(1);  // vsync

    // ---- Initialize ImGui -------------------------------------------------
    IMGUI_CHECKVERSION();
    ImGui::CreateContext();
    ImGuiIO& io = ImGui::GetIO();
    io.ConfigFlags |= ImGuiConfigFlags_NavEnableKeyboard;
    ImGui::StyleColorsDark();
    ImGui_ImplGlfw_InitForOpenGL(window, true);
    ImGui_ImplOpenGL3_Init("#version 330");

    // ---- Wire core dependencies ------------------------------------------
    std::shared_ptr<core::dictionary::IDictionary> dict;
    if (const auto dict_path = locateDictionary(); !dict_path.empty()) {
        auto db = core::storage::Database::open(dict_path);
        if (db.has_value()) {
            auto sqlite_dict = core::dictionary::SqliteDictionary::open(std::move(db.value()));
            if (sqlite_dict.has_value()) {
                dict = std::make_shared<core::dictionary::SqliteDictionary>(
                    std::move(sqlite_dict.value()));
            }
        }
    }

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

    app::AppState state(dict, history, favorites);
    if (!dict) {
        // Use a status-bar message instead of a modal — ImGui-only stack.
        // Status string is owned by AppState; mutate via a search submission
        // would override it, so we let the default "Ready." stand and rely on
        // any later search to surface "No dictionary configured.".
    }

    // ---- Main loop --------------------------------------------------------
    while (glfwWindowShouldClose(window) == 0) {
        glfwPollEvents();

        ImGui_ImplOpenGL3_NewFrame();
        ImGui_ImplGlfw_NewFrame();
        ImGui::NewFrame();

        ui::MainView::render(state);

        ImGui::Render();
        int display_w = 0;
        int display_h = 0;
        glfwGetFramebufferSize(window, &display_w, &display_h);
        glViewport(0, 0, display_w, display_h);
        glClearColor(0.10f, 0.10f, 0.12f, 1.0f);
        glClear(GL_COLOR_BUFFER_BIT);
        ImGui_ImplOpenGL3_RenderDrawData(ImGui::GetDrawData());

        glfwSwapBuffers(window);
    }

    // ---- Shutdown ---------------------------------------------------------
    ImGui_ImplOpenGL3_Shutdown();
    ImGui_ImplGlfw_Shutdown();
    ImGui::DestroyContext();
    glfwDestroyWindow(window);
    glfwTerminate();
    return 0;
}
