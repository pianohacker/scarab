cmake_minimum_required(VERSION 2.8)
project(scarab)

find_package(PkgConfig REQUIRED)
pkg_check_modules(PKG glib-2.0)
set(CMAKE_C_FLAGS "-Werror=implicit-function-declaration -std=gnu99 -Wall")
set(CMAKE_C_FLAGS_DEBUG "-ggdb3 -fsanitize=address")
# ASAN is confused by Valgrind
set(CMAKE_C_FLAGS_PROFILE "${CMAKE_C_FLAGS_DEBUG} -fno-sanitize=address -pg")

add_library(libscarab SHARED
	libscarab/builtins.c
	libscarab/error.c
	libscarab/eval.c
	libscarab/list.c
	libscarab/parser.c
	libscarab/record.c
	libscarab/strfuncs.c
	libscarab/tokenizer.c
	libscarab/value.c
)
set_target_properties(libscarab PROPERTIES
	OUTPUT_NAME scarab
	SOVERSION 1
	VERSION 1.1
)
target_link_libraries(libscarab gc m ${PKG_LDFLAGS})

add_executable(scarab
	cli/main.c
)
include_directories(${PKG_INCLUDE_DIRS})
target_link_libraries(scarab libscarab ${PKG_LDFLAGS})

enable_testing()
file(GLOB TESTS test/test-*.c)
include_directories(libscarab ${GLIB_INCLUDE_DIRS})

foreach(TESTFILE ${TESTS})
	get_filename_component(TESTPROG ${TESTFILE} NAME_WE)
	add_executable(${TESTPROG} ${TESTFILE})
	target_link_libraries(${TESTPROG} libscarab gc m ${GLIB_LIBRARIES})
	add_test(${TESTPROG} ${TESTPROG})
endforeach(TESTFILE)
