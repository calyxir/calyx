    module std_slice_IN_WIDTH_OUT_WIDTH:
        input in : UInt<IN_WIDTH>
        output out : UInt<OUT_WIDTH>

        out <= head(in, UInt(DIFF)) ; DIFF should be IN_WIDTH - OUT_WIDTH
