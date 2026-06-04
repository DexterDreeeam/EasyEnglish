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

# Iter-003+: behavioral search / history / favorites tests with fake dictionary.
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

# Iter-008: end-to-end test using the real SqliteDictionary against the shipped
# fixture, plus in-memory history/favorites stores. Covers the critical path.
add_executable(test_e2e_main_flow test_e2e_main_flow.cpp)
set_target_properties(test_e2e_main_flow PROPERTIES AUTOMOC ON)
target_compile_definitions(test_e2e_main_flow
    PRIVATE
        EASYENGLISH_FIXTURES_DIR="${CMAKE_SOURCE_DIR}/tests/fixtures"
)
target_link_libraries(test_e2e_main_flow
    PRIVATE
        easyenglish::ui
        easyenglish::core
        Qt6::Test
        Qt6::Widgets
        easyenglish_warnings
)
add_test(NAME MainWindowEndToEnd COMMAND test_e2e_main_flow)

