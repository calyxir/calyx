    module std_mult_pipe_WIDTH :
        input left : UInt<WIDTH>
        input right : UInt<WIDTH>
        input reset : UInt<1>
        input go : UInt<1>
        input clk : Clock
        output out : UInt<WIDTH>
        output done : UInt<1>

        reg rtmp : UInt<WIDTH>, clk
        reg ltmp : UInt<WIDTH>, clk
        reg out_tmp : UInt<W_SHIFTED_ONE>, clk
        reg done_buf : UInt<1>[2], clk

        ; Start sending the done signal.
        when eq(go, UInt(1)):
            connect done_buf[0], UInt(1)
        else:
            connect done_buf[0], UInt(0)

        ; Push the done signal through the pipeline.
        when eq(go, UInt(1)):
            connect done_buf[1], done_buf[0]
        else:
            connect done_buf[1], UInt(0)

        ; Register the inputs
        when eq(reset, UInt(1)):
            connect ltmp, UInt(0)
            connect rtmp, UInt(0)
        else:
            when eq(go, UInt(1)):
                connect ltmp, left
                connect rtmp, right
            else:
                connect ltmp, UInt(0)
                connect rtmp, UInt(0)
        
        ; Compute the output and save it to out_tmp
        when eq(reset, UInt(1)):
            connect out_tmp, UInt(0)
        else:
            when eq(go, UInt(1)):
                connect out_tmp, mul(ltmp, rtmp)
            else:
                connect out_tmp, out_tmp

        connect out, bits(out_tmp, HIGH, LOW)
        connect done, done_buf[1]