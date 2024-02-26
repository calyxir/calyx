    module std_div_pipe_WIDTH :
        input left : UInt<WIDTH>
        input right : UInt<WIDTH>
        input reset : UInt<1>
        input go : UInt<1>
        input clk : Clock
        output out_quotient : UInt<WIDTH>
        output out_remainder : UInt<WIDTH>
        output done : UInt<1>

        reg dividend : UInt<WIDTH>, clk
        reg divisor : UInt<W_TIMES_TWO>, clk
        reg quotient : UInt<WIDTH>, clk
        reg quotient_msk : UInt<WIDTH>, clk
        reg start : UInt<1>, clk
        reg running : UInt<1>, clk
        reg finished : UInt<1>, clk
        reg dividend_is_zero : UInt<1>, clk
        reg done_reg : UInt<1>, clk

        ; Early return if the divisor is zero
        when or(finished, dividend_is_zero):
            done_reg <= UInt(1)
        else:
            done_reg <= UInt(0)

        when or(reset, or(finished, dividend_is_zero)):
            running <= UInt(0)
        else:
            when eq(start, UInt(1)):
                running <= UInt(1)
            else:
                running <= running

        out_quotient is invalid
        out_remainder is invalid
        ; Outputs 
        when or(dividend_is_zero, start):
            out_quotient <= UInt(0)
            out_remainder <= UInt(0)
        else:
            ; when eq(finished, UInt(1)):
            out_quotient <= quotient
            out_remainder <= dividend
            ; else:
                ; Otherwise, explicitly latch the values.
                ; out_quotient <= out_quotient
                ; out_remainder <= out_remainder

        ; Calculate the quotient mask.
        when eq(start, UInt(1)):
            quotient_msk <= shl(UInt(1), W_MINUS_ONE)
        else:
            when eq(running, UInt(1)):
                quotient_msk <= shr(quotient_msk, 1)
            else:
                quotient_msk <= quotient_msk
            
        ; Calculate the quotient.
        when eq(start, UInt(1)):
            quotient <= UInt(0)
        else:
            when leq(divisor, dividend):
                quotient <= or(quotient, quotient_msk)
            else:
                quotient <= quotient

        ; Calculate the dividend.
        when eq(start, UInt(1)):
            dividend <= left
        else:
            when leq(divisor, dividend):
                dividend <= sub(dividend, divisor)
            else:
                dividend <= dividend

        when eq(start, UInt(1)):
            divisor <= shl(right, W_MINUS_ONE)
        else:
            when eq(finished, UInt(1)):
                divisor <= UInt(0)
            else:
                divisor <= shr(divisor, 1)

        done <= done_reg