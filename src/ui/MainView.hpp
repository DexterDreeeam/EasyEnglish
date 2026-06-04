#pragma once

namespace easyenglish::app {
class AppState;
}

namespace easyenglish::ui {

/// Stateless ImGui render function for the frameless overlay.
class MainView {
public:
    /// Called once per ImGui frame. Returns true if the overlay should be
    /// dismissed (user pressed Esc or activated a translation). The host main
    /// loop should hide the GLFW window and call shell.restoreForegroundWindow().
    [[nodiscard]] static bool render(app::AppState& state, bool just_shown);
};

}  // namespace easyenglish::ui
