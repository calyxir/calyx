    module std_pad_IN_WIDTH_OUT_WIDTH:
        input in : UInt<IN_WIDTH>
        output out : UInt<OUT_WIDTH>

        out <= pad(in, DIFF) ; DIFF should be OUT_WIDTH - IN_WIDTH.
