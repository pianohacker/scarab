cmake_minimum_required(VERSION 2.8)
project(scarab)

find_package(PkgConfig REQUIRED)
pkg_check_modules(PKG glib-2.0)
set(CMAKE_C_FLAGS "-Werror=implicit-function-declaration -std=gnu99 -Wall")
set(CMAKE_C_FLAGS_DEBUG "-ggdb3 -fsanitize=address")
# ASAN is confused by Valgrind
set(CMAKE_C_FLAGS_PROFILE "${CMAKE_C_FLAGS_DEBUG} -fno-sanitize=address -pg")

add_executable(scarab
	src/builtins.c
	src/error.c
	src/list.c
	src/parser.c
	src/strfuncs.c
	src/tokenizer.c
	src/value.c
	src/main.c
)
include_directories(${PKG_INCLUDE_DIRS})
target_link_libraries(scarab gc m ${PKG_LDFLAGS})
