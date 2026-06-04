# core: pure logic. Must NOT depend on any UI library (ImGui / GLFW / Qt).
# The forbidden-include check in CI enforces this — see tools/check_core_no_ui.py.

add_library(easyenglish_core STATIC)
add_library(easyenglish::core ALIAS easyenglish_core)

target_include_directories(easyenglish_core
    PUBLIC
        ${CMAKE_SOURCE_DIR}/src
)

target_link_libraries(easyenglish_core
    PRIVATE
        easyenglish_warnings
)

# Each module contributes its own sources via target_sources().
add_subdirectory(storage)
add_subdirectory(dictionary)
add_subdirectory(history)
add_subdirectory(favorites)
add_subdirectory(network)

