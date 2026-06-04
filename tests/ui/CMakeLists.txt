find_package(Qt6 REQUIRED COMPONENTS Test Widgets)

# UI tests are added per-window starting in iter-003. This file is intentionally
# small until then so CI configures cleanly.

add_executable(test_mainwindow_smoke test_mainwindow_smoke.cpp)
set_target_properties(test_mainwindow_smoke PROPERTIES AUTOMOC ON)
target_link_libraries(test_mainwindow_smoke
    PRIVATE
        easyenglish::ui
        Qt6::Test
        Qt6::Widgets
        easyenglish_warnings
)

add_test(NAME MainWindowSmoke COMMAND test_mainwindow_smoke)
