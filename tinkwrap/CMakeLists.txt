cmake_minimum_required(VERSION 3.13)

project(tinkwrap)

set(CMAKE_CXX_STANDARD 14)
set(CMAKE_CXX_STANDARD 14)
include(FetchContent)

# Import Tink as an in-tree dependency.
FetchContent_Declare(
  tink
  URL       https://github.com/tink-crypto/tink-cc/archive/refs/tags/v2.3.0.zip
  URL_HASH  SHA256=363ce671ab5ce0b24f279d3647185597a25f407c3608db007315f79f151f436b
)
FetchContent_GetProperties(tink)

if(NOT googletest_POPULATED)
  FetchContent_Populate(tink)
    add_subdirectory(${tink_SOURCE_DIR} ${tink_BINARY_DIR} EXCLUDE_FROM_ALL)
endif()

add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/3rdParty/a)

add_library(tinkwrap STATIC src/main.cpp)

target_include_directories(tinkwrap PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
target_link_libraries(tinkwrap PUBLIC a tink::static)

install(TARGETS tinkwrap DESTINATION . EXPORT tinkwrap DESTINATION .)
