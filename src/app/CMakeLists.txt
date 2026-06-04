# app: orchestration layer between core and ui.
add_library(easyenglish_app STATIC
    placeholder.cpp
)
add_library(easyenglish::app ALIAS easyenglish_app)

target_include_directories(easyenglish_app
    PUBLIC
        ${CMAKE_SOURCE_DIR}/src
)

target_link_libraries(easyenglish_app
    PUBLIC
        easyenglish::core
    PRIVATE
        easyenglish_warnings
)
