# ui: thin Qt Widgets view layer.
add_library(easyenglish_ui STATIC
    MainWindow.cpp
    MainWindow.hpp
)
add_library(easyenglish::ui ALIAS easyenglish_ui)

target_include_directories(easyenglish_ui
    PUBLIC
        ${CMAKE_SOURCE_DIR}/src
)

target_link_libraries(easyenglish_ui
    PUBLIC
        Qt6::Widgets
        easyenglish::app
    PRIVATE
        easyenglish_warnings
)
