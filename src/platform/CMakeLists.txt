# platform: shell integration (tray, global hotkey, focus, single-instance).
# Pure C++ interface + platform-specific implementations. The stub TU is
# compiled on every OS so the interface symbol resolves; the Win32 TU's body
# is guarded behind #ifdef _WIN32 so the same source list builds everywhere.

add_library(easyenglish_platform STATIC
    IPlatformShell.hpp
    StubPlatformShell.cpp
    win32/Win32PlatformShell.cpp
)
add_library(easyenglish::platform ALIAS easyenglish_platform)

target_include_directories(easyenglish_platform
    PUBLIC
        ${CMAKE_SOURCE_DIR}/src
)

target_link_libraries(easyenglish_platform
    PRIVATE
        easyenglish_warnings
)

if(WIN32)
    # Shell_NotifyIcon lives in shell32; user32 supplies the window/hook APIs.
    target_link_libraries(easyenglish_platform PUBLIC shell32 user32)
endif()
