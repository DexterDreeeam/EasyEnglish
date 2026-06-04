add_subdirectory(core)
add_subdirectory(app)
add_subdirectory(ui)

# The executable target itself: ui depends on app depends on core.
qt_add_executable(EasyEnglishApp WIN32
    main.cpp
)

target_link_libraries(EasyEnglishApp
    PRIVATE
        easyenglish::ui
        easyenglish::app
        easyenglish::core
        easyenglish_warnings
)

set_target_properties(EasyEnglishApp PROPERTIES
    OUTPUT_NAME "EasyEnglish"
)
