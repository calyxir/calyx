cmake_minimum_required(VERSION 3.3)

project(btor2tools)

set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib)
set(CMAKE_LIBRARY_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib)
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin)

set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

#-----------------------------------------------------------------------------#

option(ASAN              "Compile with ASAN support" OFF)
option(BUILD_BTOR2AIGER  "Build btor2aiger binary" OFF)
option(CHECK             "Enable assertions for optimized builds" OFF)
option(BUILD_SHARED_LIBS "Build as shared library" ON)

#-----------------------------------------------------------------------------#

include(CheckCCompilerFlag)
include(CheckCXXCompilerFlag)

macro(add_c_flag flag)
  set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} ${flag}")
  message(STATUS "Configuring with C flag '${flag}'")
endmacro()

macro(add_cxx_flag flag)
  set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} ${flag}")
  message(STATUS "Configuring with CXX flag '${flag}'")
endmacro()

macro(add_c_cxx_flag flag)
  add_c_flag(${flag})
  add_cxx_flag(${flag})
endmacro()

macro(add_check_c_flag flag)
  string(REGEX REPLACE "[-=]" "_" flagname ${flag})
  check_c_compiler_flag("${flag}" HAVE_FLAG${flagname})
  if(HAVE_FLAG${flagname})
    add_c_flag(${flag})
  endif()
endmacro()

macro(add_check_cxx_flag flag)
  string(REGEX REPLACE "[-=]" "_" flagname ${flag})
  check_cxx_compiler_flag("${flag}" HAVE_FLAG${flagname})
  if(HAVE_FLAG${flagname})
    add_cxx_flag(${flag})
  endif()
endmacro()

macro(add_check_c_cxx_flag flag)
  add_check_c_flag(${flag})
  add_check_cxx_flag(${flag})
endmacro()

macro(add_required_cxx_flag flag)
  string(REGEX REPLACE "[-=]" "_" flagnamename ${flag})
  check_cxx_compiler_flag("${flag}" HAVE_FLAG${flagname})
  if (NOT HAVE_FLAG${flagname})
    message(FATAL_ERROR "Required compiler flag ${flag} not supported")
  endif()
  add_cxx_flag(${flag})
endmacro()

macro(add_required_c_flag flag)
  string(REGEX REPLACE "[-=]" "_" flagname ${flag})
  check_c_compiler_flag("${flag}" HAVE_FLAG${flagname})
  if (NOT HAVE_FLAG${flagname})
    message(FATAL_ERROR "Required compiler flag ${flag} not supported")
  endif()
  add_c_flag(${flag})
endmacro()

macro(add_required_c_cxx_flag flag)
  add_required_c_flag(${flag})
  add_required_cxx_flag(${flag})
endmacro()

#-----------------------------------------------------------------------------#

add_check_c_cxx_flag("-W")
add_check_c_cxx_flag("-Wall")
add_check_c_cxx_flag("-Wextra")
add_check_c_cxx_flag("-Wredundant-decls")

add_check_c_flag("-std=gnu99")
add_required_cxx_flag("-std=gnu++11")

if(CMAKE_BUILD_TYPE STREQUAL "Debug")
  add_check_c_cxx_flag("-g3")
  add_check_c_cxx_flag("-ggdb")
else()
  add_check_c_cxx_flag("-O3")
  if(NOT CHECK)
    add_check_c_cxx_flag("-DNDEBUG")
  endif()
endif()

if(ASAN)
  # -fsanitize=address requires CMAKE_REQUIRED_FLAGS to be explicitely set,
  # otherwise the -fsanitize=address check will fail while linking.
  set(CMAKE_REQUIRED_FLAGS -fsanitize=address)
  add_required_c_cxx_flag("-fsanitize=address")
  unset(CMAKE_REQUIRED_FLAGS)
  add_check_c_cxx_flag("-fno-omit-frame-pointer")
  add_required_c_cxx_flag("-fsanitize-recover=address")
  set(BUILD_SHARED_LIBS ON)
endif()

if(BUILD_BTOR2AIGER)
  if(NOT EXISTS "${CMAKE_SOURCE_DIR}/deps/aiger/aiger.c")
    message(FATAL_ERROR
            "Could not find AIGER sources. Execute script setup-aiger.sh.")
  endif()
  list(APPEND CMAKE_PREFIX_PATH "${PROJECT_SOURCE_DIR}/deps/install")
  find_package(Boolector)
endif()

add_subdirectory(src)
