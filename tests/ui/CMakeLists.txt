find_package(Qt6 REQUIRED COMPONENTS Test Widgets)

# Smoke test from iter-000: minimal construction check (no dict injected).
add_executable(test_mainwindow_smoke test_mainwindow_smoke.cpp)
set_target_properties(test_mainwindow_smoke PROPERTIES AUTOMOC ON)
target_link_libraries(test_mainwindow_smoke
    PRIVATE
        easyenglish::ui
        easyenglish::core
        Qt6::Test
        Qt6::Widgets
        easyenglish_warnings
)
add_test(NAME MainWindowSmoke COMMAND test_mainwindow_smoke)

# Iter-003: behavioral search tests with a fake IDictionary.
add_executable(test_mainwindow_search test_mainwindow_search.cpp)
set_target_properties(test_mainwindow_search PROPERTIES AUTOMOC ON)
target_link_libraries(test_mainwindow_search
    PRIVATE
        easyenglish::ui
        easyenglish::core
        Qt6::Test
        Qt6::Widgets
        easyenglish_warnings
)
add_test(NAME MainWindowSearch COMMAND test_mainwindow_search)
