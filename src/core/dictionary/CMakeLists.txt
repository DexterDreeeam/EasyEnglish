# dictionary module: contributes sources to easyenglish_core.

target_sources(easyenglish_core
    PRIVATE
        ${CMAKE_CURRENT_LIST_DIR}/SqliteDictionary.cpp
        ${CMAKE_CURRENT_LIST_DIR}/SqliteDictionary.hpp
        ${CMAKE_CURRENT_LIST_DIR}/ApiDictionary.cpp
        ${CMAKE_CURRENT_LIST_DIR}/ApiDictionary.hpp
        ${CMAKE_CURRENT_LIST_DIR}/IDictionary.hpp
        ${CMAKE_CURRENT_LIST_DIR}/Entry.hpp
        ${CMAKE_CURRENT_LIST_DIR}/errors.hpp
)
