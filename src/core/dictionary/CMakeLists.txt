# dictionary module: contributes sources to easyenglish_core.
# JSON parsing uses nlohmann/json (header-only library).

find_package(nlohmann_json CONFIG REQUIRED)

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

target_link_libraries(easyenglish_core
    PUBLIC
        nlohmann_json::nlohmann_json
)
