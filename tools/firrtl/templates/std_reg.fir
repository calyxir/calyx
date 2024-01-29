    module std_reg_WIDTH :
        input in : UInt<WIDTH>
        input write_en : UInt<1>
        input clk : Clock
        input reset : UInt<1>
        output out : UInt<WIDTH>
        output done : UInt<1>

        reg internal_reg : UInt<WIDTH>, clk
        out <= internal_reg
        when eq(write_en, UInt(1)):
            internal_reg <= in
            done <= UInt(1)
        else:
            when eq(reset, UInt(1)):
                done <= UInt(0)
                internal_reg <= UInt(0)
            else:
                done <= UInt(0)
