# Unit tests. One subdirectory per src/core/<module>.
# Each module that ships an implementation gets its own CMakeLists added here.

# Placeholder smoke test so `ctest` is never empty before the first real module.
add_executable(test_smoke smoke_test.cpp)
target_link_libraries(test_smoke
    PRIVATE
        easyenglish::core
        GTest::gtest
        GTest::gtest_main
        easyenglish_warnings
)
gtest_discover_tests(test_smoke)
