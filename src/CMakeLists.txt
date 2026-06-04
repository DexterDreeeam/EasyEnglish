add_subdirectory(core)
add_subdirectory(app)
add_subdirectory(ui)

# The executable target: GLFW + ImGui-GL3 main loop.
find_package(glfw3 CONFIG REQUIRED)
find_package(imgui CONFIG REQUIRED)

add_executable(EasyEnglishApp WIN32
    main.cpp
)

target_compile_definitions(EasyEnglishApp
    PRIVATE
        EASYENGLISH_FIXTURES_DIR="${CMAKE_SOURCE_DIR}/tests/fixtures"
)

target_link_libraries(EasyEnglishApp
    PRIVATE
        easyenglish::ui
        easyenglish::app
        easyenglish::core
        glfw
        imgui::imgui
        easyenglish_warnings
)

set_target_properties(EasyEnglishApp PROPERTIES
    OUTPUT_NAME "EasyEnglish"
)

