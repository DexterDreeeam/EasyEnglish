add_executable(test_network
    test_api_dictionary.cpp
)

target_link_libraries(test_network
    PRIVATE
        easyenglish::core
        GTest::gtest
        GTest::gtest_main
        easyenglish_warnings
)

gtest_discover_tests(test_network)
