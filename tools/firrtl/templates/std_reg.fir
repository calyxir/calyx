    module MODULE_NAME :
        input in : UInt<WIDTH>
        input write_en : UInt<1>
        input clk : Clock
        input reset : UInt<1>
        output out : UInt<WIDTH>
        output done : UInt<1>

        reg internal_reg : UInt, clk
        when eq(write_en, UInt(1)):
            out <= in
            done <= UInt(1)
        else:
            when eq(reset, UInt(1)):
                done <= UInt(0)
                out <= UInt(0)
            else:
                done <= UInt(0)
