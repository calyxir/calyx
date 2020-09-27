add_mlir_dialect_library(MLIRFuTIL
        FuTILDialect.cpp
        FuTILOps.cpp

        ADDITIONAL_HEADER_DIRS
        ${PROJECT_SOURCE_DIR}/include/FuTIL

        DEPENDS
        MLIRFuTILOpsIncGen

	LINK_LIBS PUBLIC
	MLIRIR
	)
