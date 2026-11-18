    module std_xor_WIDTH :
        input left : UInt<WIDTH>
        input right : UInt<WIDTH>
        output out : UInt<WIDTH>

        connect out, xor(left, right)
