cmake_minimum_required(VERSION 3.13)

project(tinkwrap)

set(CMAKE_CXX_STANDARD 14)

add_library(tinkwrap STATIC src/main.cpp)

target_include_directories(tinkwrap PUBLIC ${CMAKE_SOURCE_DIR}/include)

set_target_properties(tinkwrap PROPERTIES PUBLIC_HEADER "${CMAKE_SOURCE_DIR}/include/main.h")

install(TARGETS tinkwrap DESTINATION . PUBLIC_HEADER DESTINATION ./lib)
