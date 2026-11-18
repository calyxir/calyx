    module std_lsh_WIDTH :
        input left : UInt<WIDTH>
        input right : UInt<WIDTH>
        output out : UInt<WIDTH>

        connect out, dshl(left, bits(right, BITS, 0))
