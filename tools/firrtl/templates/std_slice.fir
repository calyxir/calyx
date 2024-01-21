define(`DIFF', eval(IN_WIDTH - OUT_WIDTH))dnl
    module MODULE_NAME:
        input in : UInt<IN_WIDTH>
        output out : UInt<OUT_WIDTH>

        out <= head(in, UInt(DIFF)) ; DIFF should be IN_WIDTH - OUT_WIDTH
