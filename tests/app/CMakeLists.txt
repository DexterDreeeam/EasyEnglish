add_executable(test_app_state
    test_app_state.cpp
)

target_link_libraries(test_app_state
    PRIVATE
        easyenglish::app
        easyenglish::core
        GTest::gtest
        GTest::gtest_main
        easyenglish_warnings
)

gtest_discover_tests(test_app_state)
