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
            connect internal_reg, UInt(0)
            connect done_reg, UInt(0)
        else:
            when eq(write_en, UInt(1)):
                connect internal_reg, in
                connect done_reg, UInt(1)
            else:
                connect done_reg, UInt(0)

        connect out, internal_reg
        connect done, done_reg
