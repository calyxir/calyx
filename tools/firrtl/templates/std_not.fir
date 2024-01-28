    module std_not_WIDTH :
        input in : UInt<WIDTH>
        output out : UInt<WIDTH>

        out <= neg(in)
