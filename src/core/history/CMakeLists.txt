# history module: contributes sources to easyenglish_core.

target_sources(easyenglish_core
    PRIVATE
        ${CMAKE_CURRENT_LIST_DIR}/HistoryStore.cpp
        ${CMAKE_CURRENT_LIST_DIR}/HistoryStore.hpp
)
