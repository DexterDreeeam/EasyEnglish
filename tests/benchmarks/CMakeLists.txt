find_package(benchmark CONFIG REQUIRED)

# Real benchmarks land in iter-002. Until then a noop benchmark proves wiring.
add_executable(bench_smoke bench_smoke.cpp)
target_link_libraries(bench_smoke
    PRIVATE
        easyenglish::core
        benchmark::benchmark
        benchmark::benchmark_main
        easyenglish_warnings
)
