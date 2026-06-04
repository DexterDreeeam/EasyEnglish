add_executable(test_history
    test_history_store.cpp
)

target_link_libraries(test_history
    PRIVATE
        easyenglish::core
        GTest::gtest
        GTest::gtest_main
        easyenglish_warnings
)

gtest_discover_tests(test_history)
