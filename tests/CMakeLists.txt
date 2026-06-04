find_package(GTest CONFIG REQUIRED)
include(GoogleTest)

add_subdirectory(unit)
add_subdirectory(app)

if(EASYENGLISH_BUILD_BENCHMARKS)
    add_subdirectory(benchmarks)
endif()
