cmake_minimum_required(VERSION 3.25)

project(EasyEnglish
    VERSION 0.2.0
    DESCRIPTION "AI-assisted English word lookup desktop app for Windows (ImGui edition)"
    LANGUAGES CXX
)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

option(EASYENGLISH_BUILD_TESTS        "Build unit/UI tests"                 ON)
option(EASYENGLISH_BUILD_BENCHMARKS   "Build performance benchmarks"        ON)
option(EASYENGLISH_ENABLE_ASAN        "Enable AddressSanitizer (Debug)"     OFF)
option(EASYENGLISH_WARNINGS_AS_ERRORS "Treat warnings as errors"            ON)

# ---- Compile flags ----------------------------------------------------------
add_library(easyenglish_warnings INTERFACE)
if(MSVC)
    target_compile_options(easyenglish_warnings INTERFACE /W4 /permissive- /utf-8)
    if(EASYENGLISH_WARNINGS_AS_ERRORS)
        target_compile_options(easyenglish_warnings INTERFACE /WX)
    endif()
    if(EASYENGLISH_ENABLE_ASAN)
        target_compile_options(easyenglish_warnings INTERFACE /fsanitize=address)
    endif()
else()
    target_compile_options(easyenglish_warnings INTERFACE -Wall -Wextra -Wpedantic)
    if(EASYENGLISH_WARNINGS_AS_ERRORS)
        target_compile_options(easyenglish_warnings INTERFACE -Werror)
    endif()
endif()

# ---- Subdirectories --------------------------------------------------------
add_subdirectory(src)

if(EASYENGLISH_BUILD_TESTS)
    enable_testing()
    add_subdirectory(tests)
endif()

# ---- Status ----------------------------------------------------------------
message(STATUS "EasyEnglish ${PROJECT_VERSION} (ImGui edition)")
message(STATUS "  C++ standard:       ${CMAKE_CXX_STANDARD}")
message(STATUS "  Build type:         ${CMAKE_BUILD_TYPE}")
message(STATUS "  Tests:              ${EASYENGLISH_BUILD_TESTS}")
message(STATUS "  Benchmarks:         ${EASYENGLISH_BUILD_BENCHMARKS}")
message(STATUS "  AddressSanitizer:   ${EASYENGLISH_ENABLE_ASAN}")
message(STATUS "  Warnings as errors: ${EASYENGLISH_WARNINGS_AS_ERRORS}")
