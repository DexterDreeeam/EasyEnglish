# network module: contributes sources to easyenglish_core.
# Real HTTP client uses cpp-httplib (header-only) with OpenSSL for HTTPS.

find_package(httplib CONFIG REQUIRED)

target_sources(easyenglish_core
    PRIVATE
        ${CMAKE_CURRENT_LIST_DIR}/INetworkClient.hpp
        ${CMAKE_CURRENT_LIST_DIR}/HttpNetworkClient.hpp
        ${CMAKE_CURRENT_LIST_DIR}/HttpNetworkClient.cpp
)

target_link_libraries(easyenglish_core
    PUBLIC
        httplib::httplib
)
