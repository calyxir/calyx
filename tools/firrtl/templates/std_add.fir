    module std_add_WIDTH:
        input left : UInt<WIDTH>
        input right : UInt<WIDTH>
        output out : UInt<WIDTH>

        out <= add(left, right)
