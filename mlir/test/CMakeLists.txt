configure_lit_site_cfg(
        ${CMAKE_CURRENT_SOURCE_DIR}/lit.site.cfg.py.in
        ${CMAKE_CURRENT_BINARY_DIR}/lit.site.cfg.py
        MAIN_CONFIG
        ${CMAKE_CURRENT_SOURCE_DIR}/lit.cfg.py
)

set(FUTIL_TEST_DEPENDS
        FileCheck count not
        futil-opt
        futil-translate
        )

add_lit_testsuite(check-futil "Running the futil MLIR regression tests"
        ${CMAKE_CURRENT_BINARY_DIR}
        DEPENDS ${FUTIL_TEST_DEPENDS}
        )
set_target_properties(check-futil PROPERTIES FOLDER "Tests")

add_lit_testsuites(FUTIL ${CMAKE_CURRENT_SOURCE_DIR} DEPENDS ${FUTIL_TEST_DEPENDS})
