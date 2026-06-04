#pragma once

namespace easyenglish::app {
class AppState;
}

namespace easyenglish::ui {

/// Stateless ImGui render function. Call once per frame with the current
/// `AppState`; AppState exposes both the read-only data to render and the
/// mutators the view calls in response to user input.
class MainView {
public:
    /// Renders the entire main window (search box + result panel + side tabs
    /// + status row) into the current ImGui frame. Must be called between
    /// `ImGui::NewFrame()` and `ImGui::Render()`.
    static void render(app::AppState& state);
};

}  // namespace easyenglish::ui
