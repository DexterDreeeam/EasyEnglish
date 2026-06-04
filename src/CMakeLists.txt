add_subdirectory(core)
add_subdirectory(platform)
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
        easyenglish::platform
        glfw
        imgui::imgui
        easyenglish_warnings
)

# Link the system OpenGL 1.1 loader for the few raw GL calls main.cpp makes
# (glClear / glClearColor / glViewport). ImGui's own renderer loads anything
# higher than that via its bundled loader.
if(WIN32)
    target_link_libraries(EasyEnglishApp PRIVATE opengl32)
elseif(APPLE)
    target_link_libraries(EasyEnglishApp PRIVATE "-framework OpenGL")
else()
    find_package(OpenGL REQUIRED)
    target_link_libraries(EasyEnglishApp PRIVATE OpenGL::GL)
endif()

# WIN32 subsystem normally requires WinMain. Redirect to mainCRTStartup so we
# can keep a portable `int main(int, char*[])` entry point.
if(MSVC)
    target_link_options(EasyEnglishApp PRIVATE "/ENTRY:mainCRTStartup")
endif()

set_target_properties(EasyEnglishApp PROPERTIES
    OUTPUT_NAME "EasyEnglish"
)

