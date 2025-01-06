cmake_minimum_required(VERSION 3.13)

project(a)

set(CMAKE_CXX_STANDARD 14)

add_library(a STATIC src/a.cpp)

target_include_directories(a PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)

install(TARGETS a
	EXPORT as 
	INCLUDES DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}")
export(TARGETS a FILE ${CMAKE_CURRENT_BINARY_DIR}/aConfig.cmake)
