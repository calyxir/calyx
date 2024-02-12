    module comb_mem_d1_WIDTH_SIZE_IDX_SIZE :
        input add0 : UInt<IDX_SIZE>
        input write_data : UInt<WIDTH>
        input write_en : UInt<1>
        input clk : Clock
        input reset : UInt<1>
        output read_data : UInt<WIDTH>
        output done : UInt<1>

        mem internal_mem :
            data-type => UInt<WIDTH>
            depth => SIZE
            read-latency => 1
            write-latency => 1
            reader => internal_read
            writer => internal_write
            read-under-write => undefined

        ; read from memory
        internal_mem.internal_read.addr <= add0
        internal_mem.internal_read.en <= UInt(1)
        internal_mem.internal_read.clk <= clk
        read_data <= internal_mem.internal_read.data

        ; write to memory
        internal_mem.internal_write.addr <= add0
        internal_mem.internal_write.en <= write_en
        internal_mem.internal_write.clk <= clk
        internal_mem.internal_write.data <= write_data
        internal_mem.internal_write.mask <= UInt(1) ; unclear
        when eq(write_en, UInt(1)):
            done <= UInt(1)
        else:
            done <= UInt(0)
