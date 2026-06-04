# ui: Dear ImGui-based view layer. Stateless render() that draws an AppState.
find_package(imgui CONFIG REQUIRED)

add_library(easyenglish_ui STATIC
    MainView.cpp
    MainView.hpp
)
add_library(easyenglish::ui ALIAS easyenglish_ui)

target_include_directories(easyenglish_ui
    PUBLIC
        ${CMAKE_SOURCE_DIR}/src
)

target_link_libraries(easyenglish_ui
    PUBLIC
        imgui::imgui
        easyenglish::app
    PRIVATE
        easyenglish_warnings
)
