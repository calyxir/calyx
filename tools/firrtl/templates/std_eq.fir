    module std_eq_WIDTH :
        input left : UInt<WIDTH>
        input right : UInt<WIDTH>
        output out : UInt<1>

        connect out, eq(left, right)
