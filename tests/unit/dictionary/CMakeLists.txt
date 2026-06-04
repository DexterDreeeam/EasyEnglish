add_executable(test_dictionary
    test_sqlite_dictionary.cpp
)

target_compile_definitions(test_dictionary
    PRIVATE
        EASYENGLISH_FIXTURES_DIR="${CMAKE_SOURCE_DIR}/tests/fixtures"
)

target_link_libraries(test_dictionary
    PRIVATE
        easyenglish::core
        GTest::gtest
        GTest::gtest_main
        easyenglish_warnings
)

gtest_discover_tests(test_dictionary)
