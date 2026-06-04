# The storage module owns its source list. This file is included from
# src/core/CMakeLists.txt which created the easyenglish_core target.

find_package(unofficial-sqlite3 CONFIG REQUIRED)

target_sources(easyenglish_core
    PRIVATE
        ${CMAKE_CURRENT_LIST_DIR}/Database.cpp
        ${CMAKE_CURRENT_LIST_DIR}/Statement.cpp
        ${CMAKE_CURRENT_LIST_DIR}/Database.hpp
        ${CMAKE_CURRENT_LIST_DIR}/Statement.hpp
        ${CMAKE_CURRENT_LIST_DIR}/errors.hpp
)

target_link_libraries(easyenglish_core
    PRIVATE
        unofficial::sqlite3::sqlite3
)
