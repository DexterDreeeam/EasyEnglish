# favorites module: contributes sources to easyenglish_core.

target_sources(easyenglish_core
    PRIVATE
        ${CMAKE_CURRENT_LIST_DIR}/FavoritesStore.cpp
        ${CMAKE_CURRENT_LIST_DIR}/FavoritesStore.hpp
)
