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
            done_buf[0] <= UInt(1)
        else:
            done_buf[0] <= UInt(0)

        ; Push the done signal through the pipeline.
        when eq(go, UInt(1)):
            done_buf[1] <= done_buf[0]
        else:
            done_buf[1] <= UInt(0)

        ; Register the inputs
        when eq(reset, UInt(1)):
            ltmp <= UInt(0)
            rtmp <= UInt(0)
        else:
            when eq(go, UInt(1)):
                ltmp <= left
                rtmp <= right
            else:
                ltmp <= UInt(0)
                rtmp <= UInt(0)
        
        ; Compute the output and save it to out_tmp
        when eq(reset, UInt(1)):
            out_tmp <= UInt(0)
        else:
            when eq(go, UInt(1)):
                out_tmp <= mul(ltmp, rtmp)
            else:
                out_tmp <= out_tmp

        out <= bits(out_tmp, HIGH, LOW)
        done <= done_buf[1]