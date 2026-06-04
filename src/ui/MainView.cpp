#include "ui/MainView.hpp"

#include <cstddef>

#include <imgui.h>

#include "app/AppState.hpp"

namespace easyenglish::ui {

void MainView::render(app::AppState& state) {
    const ImGuiViewport* vp = ImGui::GetMainViewport();
    ImGui::SetNextWindowPos(vp->WorkPos);
    ImGui::SetNextWindowSize(vp->WorkSize);

    constexpr ImGuiWindowFlags kFlags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoMove |
                                        ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoCollapse |
                                        ImGuiWindowFlags_NoBringToFrontOnFocus;
    ImGui::Begin("EasyEnglishMain", nullptr, kFlags);

    // ---- Search row -------------------------------------------------------
    const float buttons_width = 200.0f;
    ImGui::PushItemWidth(-buttons_width);
    const bool submitted =
        ImGui::InputText("##search", state.input_buffer.data(), state.input_buffer.size(),
                         ImGuiInputTextFlags_EnterReturnsTrue);
    ImGui::PopItemWidth();
    if (submitted) {
        state.submitSearch();
    }

    ImGui::SameLine();
    ImGui::BeginDisabled(!state.inputIsNonEmpty());
    if (ImGui::Button("Search", ImVec2(80, 0))) {
        state.submitSearch();
    }
    ImGui::EndDisabled();

    ImGui::SameLine();
    const bool can_fav = state.hasFavorites() && state.currentEntry().has_value();
    ImGui::BeginDisabled(!can_fav);
    const char* star_label = state.currentIsFavorite() ? "Unstar" : "Star";
    if (ImGui::Button(star_label, ImVec2(80, 0))) {
        state.toggleFavorite();
    }
    ImGui::EndDisabled();

    // ---- Body: result panel | side tabs -----------------------------------
    const float status_h = ImGui::GetFrameHeightWithSpacing();
    const float side_w = 240.0f;

    ImGui::BeginChild("ResultPanel", ImVec2(-side_w - ImGui::GetStyle().ItemSpacing.x, -status_h),
                      /*border=*/true);
    if (state.currentEntry().has_value()) {
        const auto& e = state.currentEntry().value();
        ImGui::PushFont(nullptr);
        ImGui::TextWrapped("%s", e.headword.c_str());
        ImGui::PopFont();
        if (!e.phonetic.empty()) {
            ImGui::TextDisabled("%s", e.phonetic.c_str());
        }
        ImGui::Separator();
        for (std::size_t i = 0; i < e.definitions.size(); ++i) {
            ImGui::TextWrapped("%zu. %s", i + 1, e.definitions[i].c_str());
        }
    } else {
        ImGui::TextDisabled("No entry. Type a word above and press Enter.");
    }
    ImGui::EndChild();

    ImGui::SameLine();

    ImGui::BeginChild("SidePanel", ImVec2(0, -status_h), /*border=*/true);
    if (ImGui::BeginTabBar("##SideTabs")) {
        if (ImGui::BeginTabItem("History")) {
            const auto& items = state.recent();
            if (items.empty()) {
                ImGui::TextDisabled("(empty)");
            }
            for (std::size_t i = 0; i < items.size(); ++i) {
                if (ImGui::Selectable(items[i].headword.c_str())) {
                    state.activateRecent(i);
                }
            }
            ImGui::EndTabItem();
        }
        if (ImGui::BeginTabItem("Favorites")) {
            const auto& items = state.favorites();
            if (items.empty()) {
                ImGui::TextDisabled("(empty)");
            }
            for (std::size_t i = 0; i < items.size(); ++i) {
                if (ImGui::Selectable(items[i].headword.c_str())) {
                    state.activateFavorite(i);
                }
            }
            ImGui::EndTabItem();
        }
        ImGui::EndTabBar();
    }
    ImGui::EndChild();

    // ---- Status row -------------------------------------------------------
    ImGui::TextWrapped("%s", state.status().c_str());

    ImGui::End();
}

}  // namespace easyenglish::ui
