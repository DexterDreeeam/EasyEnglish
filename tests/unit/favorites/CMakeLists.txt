add_executable(test_favorites
    test_favorites_store.cpp
)

target_link_libraries(test_favorites
    PRIVATE
        easyenglish::core
        GTest::gtest
        GTest::gtest_main
        easyenglish_warnings
)

gtest_discover_tests(test_favorites)
