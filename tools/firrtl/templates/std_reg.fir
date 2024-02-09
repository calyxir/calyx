    module std_reg_WIDTH :
        input in : UInt<WIDTH>
        input write_en : UInt<1>
        input clk : Clock
        input reset : UInt<1>
        output out : UInt<WIDTH>
        output done : UInt<1>

        reg internal_reg : UInt<WIDTH>, clk
        reg done_reg : UInt<1>, clk

        when eq(reset, UInt(1)):
            internal_reg <= UInt(0)
            done_reg <= UInt(0)
        else:
            when eq(write_en, UInt(1)):
                internal_reg <= in
                done_reg <= UInt(1)
            else:
                done_reg <= UInt(0)

        out <= internal_reg
        done <= done_reg
