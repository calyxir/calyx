add_library(btor2parser
  btor2parser/btor2parser.c
)
target_include_directories(btor2parser PRIVATE .)

add_executable(btorsim
  btorsim/btorsim.c
  btorsim/btorsimbv.c
  btorsim/btorsimrng.c
)
target_include_directories(btorsim PRIVATE .)
target_link_libraries(btorsim btor2parser)

add_executable(catbtor catbtor.c)
target_link_libraries(catbtor btor2parser)

if(BUILD_BTOR2AIGER)
  add_executable(btor2aiger
    btor2aiger.cpp
    ${CMAKE_SOURCE_DIR}/deps/aiger/aiger.c
  )
  target_include_directories(btor2aiger PRIVATE . ${CMAKE_SOURCE_DIR}/deps/aiger)
  target_compile_options(btor2aiger PRIVATE -std=c++11 -Wall -g -g3 -ggdb)
  target_link_libraries(btor2aiger btor2parser Boolector::boolector)
endif()

add_executable(btorsplit btorsplit.cpp)
