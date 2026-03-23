    module std_bit_slice_IN_WIDTH_START_IDX_END_IDX_OUT_WIDTH :
        input in : UInt<IN_WIDTH>
        output out : UInt<OUT_WIDTH>

        out <= bits(in, END_IDX, START_IDX)