cmake_minimum_required(VERSION 3.13)

project(tinkwrap)

set(CMAKE_CXX_STANDARD 14)

add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/3rdParty/a)

add_library(tinkwrap STATIC src/main.cpp)

target_include_directories(tinkwrap PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
target_link_libraries(tinkwrap PUBLIC a)

install(TARGETS tinkwrap DESTINATION . EXPORT tinkwrap DESTINATION .)
