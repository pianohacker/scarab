cmake_minimum_required(VERSION 2.8)
project(scarab)

find_package(PkgConfig REQUIRED)
pkg_check_modules(PKG libglib)
set(CMAKE_C_FLAGS "-Werror=implicit-function-declaration -std=gnu99 -Wall")
set(CMAKE_C_FLAGS_DEBUG "-ggdb3")
set(CMAKE_C_FLAGS_PROFILE "${CMAKE_C_FLAGS_DEBUG} -pg")

add_executable(scarab
	src/tokenizer.c
	src/parser.c
	src/eval.c
	src/error.c
	src/value.c
	src/list.c
	src/main.c
)
include_directories(${PKG_INCLUDE_DIRS})
target_link_libraries(nube m ${PKG_LDFLAGS})
