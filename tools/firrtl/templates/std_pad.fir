define(`DIFF', eval(OUT_WIDTH - IN_WIDTH))dnl
    module MODULE_NAME:
        input in : UInt<IN_WIDTH>
        output out : UInt<OUT_WIDTH>

        out <= pad(in, DIFF) ; DIFF should be OUT_WIDTH - IN_WIDTH.
