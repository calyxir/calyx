    module std_and_WIDTH :
        input left : UInt<WIDTH>
        input right : UInt<WIDTH>
        output out : UInt<WIDTH>

        connect out, and(left, right)
