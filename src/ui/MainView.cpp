#include "ui/MainView.hpp"

#include <cstddef>

#include <imgui.h>

#include "app/AppState.hpp"

namespace easyenglish::ui {

bool MainView::render(app::AppState& state, bool just_shown) {
    // Borderless, edge-to-edge ImGui window matching the GLFW window's size.
    const ImGuiViewport* vp = ImGui::GetMainViewport();
    ImGui::SetNextWindowPos(vp->WorkPos);
    ImGui::SetNextWindowSize(vp->WorkSize);

    constexpr ImGuiWindowFlags kFlags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoMove |
                                        ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoCollapse |
                                        ImGuiWindowFlags_NoScrollbar |
                                        ImGuiWindowFlags_NoBringToFrontOnFocus;
    ImGui::PushStyleVar(ImGuiStyleVar_WindowRounding, 6.0f);
    ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(12, 10));
    ImGui::Begin("##overlay", nullptr, kFlags);

    bool dismiss = false;

    // Single-line input. Auto-focus on (re-)show and on every frame the user
    // re-enters via the hotkey, since GLFW restores focus to the window but
    // not into a specific widget.
    if (just_shown) {
        ImGui::SetKeyboardFocusHere();
    }

    ImGui::PushItemWidth(-FLT_MIN);
    const bool submitted =
        ImGui::InputText("##search", state.input_buffer.data(), state.input_buffer.size(),
                         ImGuiInputTextFlags_EnterReturnsTrue | ImGuiInputTextFlags_AutoSelectAll);
    ImGui::PopItemWidth();
    if (submitted) {
        state.submitSearch();
    }

    // Esc dismisses overlay. Handle here so we don't have to plumb the key
    // event through GLFW->AppState.
    if (ImGui::IsKeyPressed(ImGuiKey_Escape, /*repeat=*/false)) {
        dismiss = true;
    }

    // Headword + phonetic line. Drawn even when empty so the layout doesn't
    // jump on first keypress.
    if (!state.currentHeadword().empty()) {
        ImGui::Spacing();
        ImGui::Text("%s", state.currentHeadword().c_str());
        if (!state.currentPhonetic().empty()) {
            ImGui::SameLine();
            ImGui::TextDisabled("  %s", state.currentPhonetic().c_str());
        }
        ImGui::Separator();
    }

    // Dropdown of Chinese translations as selectables. Clicking any one
    // dismisses the overlay (the user got what they came for).
    const auto& items = state.currentTranslations();
    for (std::size_t i = 0; i < items.size(); ++i) {
        ImGui::PushID(static_cast<int>(i));
        if (ImGui::Selectable(items[i].c_str())) {
            dismiss = true;
        }
        ImGui::PopID();
    }

    // Status line only when there's something to say (e.g. "Not found").
    if (!state.status().empty() && items.empty()) {
        ImGui::TextDisabled("%s", state.status().c_str());
    }

    ImGui::End();
    ImGui::PopStyleVar(2);
    return dismiss;
}

}  // namespace easyenglish::ui
