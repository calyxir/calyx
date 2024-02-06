    extmodule file_read_mem_WIDTH_SIZE_IDX_SIZE:
        input addr0: UInt<IDX_SIZE>
        input write_data: UInt<WIDTH>
        input write_en: UInt<1>
        input clk: Clock
        input reset: UInt<1>
        output read_data: UInt<32>
        output done: UInt<1>
        defname = file_read_mem
