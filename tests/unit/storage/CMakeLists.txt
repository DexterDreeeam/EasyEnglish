add_executable(test_storage
    test_database.cpp
    test_statement.cpp
    test_fixture_mini_dict.cpp
)

target_compile_definitions(test_storage
    PRIVATE
        EASYENGLISH_FIXTURES_DIR="${CMAKE_SOURCE_DIR}/tests/fixtures"
)

target_link_libraries(test_storage
    PRIVATE
        easyenglish::core
        GTest::gtest
        GTest::gtest_main
        easyenglish_warnings
)

gtest_discover_tests(test_storage)
