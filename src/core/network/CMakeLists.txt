# network module: contributes sources to easyenglish_core.
# Qt6::Network is required for QtNetworkClient.

find_package(Qt6 REQUIRED COMPONENTS Network)

target_sources(easyenglish_core
    PRIVATE
        ${CMAKE_CURRENT_LIST_DIR}/INetworkClient.hpp
        ${CMAKE_CURRENT_LIST_DIR}/QtNetworkClient.hpp
        ${CMAKE_CURRENT_LIST_DIR}/QtNetworkClient.cpp
)

target_link_libraries(easyenglish_core
    PUBLIC
        Qt6::Network
)
