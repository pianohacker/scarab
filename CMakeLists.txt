cmake_minimum_required(VERSION 2.8)
project(scarab)

find_package(PkgConfig REQUIRED)
pkg_check_modules(PKG glib-2.0)
set(CMAKE_C_FLAGS "-Werror=implicit-function-declaration -std=gnu99 -Wall")
# The leak checker is not usually useful (and is confused by GLib).
set(CMAKE_C_FLAGS_DEBUG "-ggdb3 -fsanitize=address -fno-sanitize=leak -DKH_IS_DEBUG -DGC_DEBUG")
# ASAN is confused by Valgrind, so disable it. Also, we use sprof (gprof being useless for shared
# libraries), so we don't want -pg.
set(CMAKE_C_FLAGS_PROFILE "${CMAKE_C_FLAGS_DEBUG} -UGC_DEBUG -fno-sanitize=address")

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
