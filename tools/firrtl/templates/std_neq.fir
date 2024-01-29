    module std_neq_WIDTH :
        input left : UInt<WIDTH>
        input right : UInt<WIDTH>
        output out : UInt<1>

        out <= neq(left, right)
