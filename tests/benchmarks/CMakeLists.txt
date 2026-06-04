find_package(benchmark CONFIG REQUIRED)

# Real benchmarks land in iter-002+. Smoke target stays so CI proves wiring
# even when no real bench has been added yet.
add_executable(bench_smoke bench_smoke.cpp)
target_link_libraries(bench_smoke
    PRIVATE
        easyenglish::core
        benchmark::benchmark
        benchmark::benchmark_main
        easyenglish_warnings
)

add_executable(bench_lookup bench_lookup.cpp)
target_compile_definitions(bench_lookup
    PRIVATE
        EASYENGLISH_FIXTURES_DIR="${CMAKE_SOURCE_DIR}/tests/fixtures"
)
target_link_libraries(bench_lookup
    PRIVATE
        easyenglish::core
        benchmark::benchmark
        benchmark::benchmark_main
        easyenglish_warnings
)
