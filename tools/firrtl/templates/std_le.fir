    module std_le_WIDTH :
        input left : UInt<WIDTH>
        input right : UInt<WIDTH>
        output out : UInt<1>

        connect out, leq(left, right)
