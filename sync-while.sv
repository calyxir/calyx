/**
* Synchronization primitives for Calyx
* Requirements: 
* 1. write_en_* signals should remain 1 until the 
*    corresponding done_* signals are set to 1 once they are set to 1.
* 2. in_* should remain the same value once their corresponding write_en_* signals
*    are set high until their corresponding done_* signals are set to 1.
* 3. read_en_* signals should remain 1 until the read_done_* signal is set to 1. 
* 4. read_en_* signals should be set to 0 once the read_done_* signals are high.
*/

// M-structure: Register primitive that blocks writes until a read happens.
module std_sync_reg #(
    parameter WIDTH = 32
) (
   input wire [ WIDTH-1:0]    in_0,
   input wire [ WIDTH-1:0]    in_1,
   input wire                 read_en_0,
   input wire                 read_en_1,
   input wire                 write_en_0,
   input wire                 write_en_1,
   input wire                 clk,
   input wire                 reset,
    // output
   output logic [WIDTH - 1:0] out_0,
   output logic [WIDTH - 1:0] out_1,
   output logic               write_done_0,
   output logic               write_done_1,
   output logic               read_done_0,
   output logic               read_done_1,
   output logic [WIDTH - 1:0] peek
);

  logic is_full;
  logic [WIDTH - 1:0] state;
  logic arbiter_w;
  logic arbiter_r;

  // States
  logic READ_ONE_HOT, READ_MULT, WRITE_ONE_HOT, WRITE_MULT, WRITE_0, WRITE_1, READ_0, READ_1;

  assign READ_ONE_HOT = is_full && (read_en_0 ^ read_en_1);
  assign READ_MULT = is_full && (read_en_0 && read_en_1);
  assign WRITE_ONE_HOT = !is_full && (write_en_0 ^ write_en_1);
  assign WRITE_MULT = !is_full && (write_en_0 && write_en_1);
  assign WRITE_0 = (WRITE_ONE_HOT && write_en_0 == 1) || (WRITE_MULT && arbiter_w == 0);
  assign WRITE_1 = (WRITE_ONE_HOT && write_en_1 == 1) || (WRITE_MULT && arbiter_w == 1);
  assign READ_0 = (READ_ONE_HOT && read_en_0 == 1) || (READ_MULT && arbiter_r == 0);
  assign READ_1 = (READ_ONE_HOT && read_en_1 == 1) || (READ_MULT && arbiter_r == 1);

  // State transitions
  always_ff @(posedge clk) begin
    if (reset)
      is_full <= 0;
    else if (WRITE_ONE_HOT || WRITE_MULT)
      is_full <= 1;
    else if (READ_ONE_HOT || READ_MULT)
      is_full <= 0;
    else
      is_full <= is_full;
  end

  // Value of writer arbiter.
  // The arbiter is round robin: if in the current cycle it is 0, the next cycle
  // it is set to 1 and vice versa.
  // If the arbiter is not used to decide which value to write in for the current
  // cycle, then in the next cycle its value does not change.
  always_ff @(posedge clk) begin
    if (reset) 
      arbiter_w <= 0;
    else if (WRITE_MULT && arbiter_w == 0)
      arbiter_w <= 1;
    else if (WRITE_MULT && arbiter_w == 1)
      arbiter_w <= 0;
    else 
      arbiter_w <= arbiter_w;
  end

  // Value of reader arbiter.
  // The arbiter is round robin: if in the current cycle it is 0, the next cycle
  // it is set to 1 and vice versa.
  // If the arbiter is not used to decide which reader to give its value to for the current
  // cycle, then in the next cycle its value does not change.
  always_ff @(posedge clk) begin
    if (reset) 
      arbiter_r <= 0;
    else if (READ_MULT && arbiter_r == 0)
      arbiter_r <= 1;
    else if (READ_MULT && arbiter_r == 1)
      arbiter_r <= 0;
    else 
      arbiter_r <= arbiter_r;
  end
      

  // Value of out_0 port.
  // Note that output is only available for one cycle.
  // out_0 has value only when 
  // 1. only read_en_0 is high
  // 2. read_en_0 and read_en_1 are both high and arbiter_r == 0.
  always_ff @(posedge clk) begin
    if (reset)
      out_0 <= 0;
    else if (READ_0)
      out_0 <= state;
    else
      out_0 <= 'x; // This could've been a latch but we explicitly define the output as undefined.
  end

  // Value of out_1 port.
  // Note that output is only available for one cycle.
  // out_1 has value only when 
  // 1. only read_en_1 is high
  // 2. read_en_0 and read_en_1 are both high and arbiter_r == 1.
  always_ff @(posedge clk) begin
    if (reset)
      out_1 <= 0;
    else if (READ_1)
      out_1 <= state;
    else
      out_1 <= 'x; // This could've been a latch but we explicitly define the output as undefined.
  end

  // Writing values
  // If only one writer is active, we write the active value in
  // If multiple writers are active at the same time, the arbiter decides which 
  // writer's input to take
  always_ff @(posedge clk) begin
    if (reset)
      state <= 0;
    else if (WRITE_0)
      state <= in_0;
    else if (WRITE_1)
      state <= in_1;
    else if (READ_ONE_HOT || READ_MULT)
      state <= 'x;  // This could've been a latch but explicitly make it undefined.
    else
      state <= state;
  end

  //Value of the "peek" port.
  // If the register is full, peek holds the same value as the state of the register.
  // If the register is empty, peek holds the most recent valid state of the register.
  always_ff @(posedge clk) begin
    if (reset)
      peek <= 0;
    else if (WRITE_0)
      peek <= in_0;
    else if (WRITE_1)
      peek <= in_1;
    else
      peek <= peek;
  end

  // Done signal for write_0 commital
  // Two scenarios that write_done_0 is set to 1:
  // 1. in_0 is the only writer for the current cycle
  // 2. Two writers are active at the same time, and the arbiter chooses
  //    in_0 to write into the register
  always_ff @(posedge clk) begin
    if (reset)
      write_done_0 <= 0;
    else if (WRITE_0)
      write_done_0 <= 1;
    else
      write_done_0 <= 0;
  end

  //Done signal for write_1 commital
  // Two scenarios that write_done_1 is set to 1:
  // 1. in_1 is the only writer for the current cycle
  // 2. Two writers are active at the same time, and the arbiter chooses
  //    in_1 to write into the register
  always_ff @(posedge clk) begin
    if (reset)
      write_done_1 <= 0;
    else if (WRITE_1)
      write_done_1 <= 1;
    else
      write_done_1 <= 0;
    end

  // Done signal for read_0 commital
  always_ff @(posedge clk) begin
    if (reset)
      read_done_0 <= 0;
    else if (READ_0)
      read_done_0 <= 1;
    else
      read_done_0 <= 0;
  end

  // Done signal for read_1 commital
  always_ff @(posedge clk) begin
    if (reset)
      read_done_1 <= 0;
    else if (READ_1)
      read_done_1 <= 1;
    else
      read_done_1 <= 0;
  end

  //Simulation self test against overlapping of mutually exclusive states
  `ifdef VERILATOR
    always @(posedge clk) begin
      if (READ_0 && READ_1) 
        $error(
          "\nstd_sync_reg: overlapping of mutually exclusive states!\n",
          "can be at only one of READ_0 and READ_1", 
        );
      else if (WRITE_0 && WRITE_1)
        $error(
          "\nstd_sync_reg: overlapping of mutually exclusive states!\n",
          "can be at only one of WRITE_0 and WRITE_1", 
        );
    end
  `endif

endmodule
/**
 * Core primitives for Calyx.
 * Implements core primitives used by the compiler.
 *
 * Conventions:
 * - All parameter names must be SNAKE_CASE and all caps.
 * - Port names must be snake_case, no caps.
 */
`default_nettype none

module std_const #(
    parameter WIDTH = 32,
    parameter VALUE = 0
) (
   output logic [WIDTH - 1:0] out
);
  assign out = VALUE;
endmodule

module std_wire #(
  parameter WIDTH = 32
) (
  input wire logic [WIDTH - 1:0] in,
  output logic [WIDTH - 1:0] out
);
  assign out = in;
endmodule

module std_slice #(
    parameter IN_WIDTH  = 32,
    parameter OUT_WIDTH = 32
) (
   input wire                   logic [ IN_WIDTH-1:0] in,
   output logic [OUT_WIDTH-1:0] out
);
  assign out = in[OUT_WIDTH-1:0];

  `ifdef VERILATOR
    always_comb begin
      if (IN_WIDTH < OUT_WIDTH)
        $error(
          "std_slice: Input width less than output width\n",
          "IN_WIDTH: %0d", IN_WIDTH,
          "OUT_WIDTH: %0d", OUT_WIDTH
        );
    end
  `endif
endmodule

module std_pad #(
    parameter IN_WIDTH  = 32,
    parameter OUT_WIDTH = 32
) (
   input wire logic [IN_WIDTH-1:0]  in,
   output logic     [OUT_WIDTH-1:0] out
);
  localparam EXTEND = OUT_WIDTH - IN_WIDTH;
  assign out = { {EXTEND {1'b0}}, in};

  `ifdef VERILATOR
    always_comb begin
      if (IN_WIDTH > OUT_WIDTH)
        $error(
          "std_pad: Output width less than input width\n",
          "IN_WIDTH: %0d", IN_WIDTH,
          "OUT_WIDTH: %0d", OUT_WIDTH
        );
    end
  `endif
endmodule

module std_not #(
    parameter WIDTH = 32
) (
   input wire               logic [WIDTH-1:0] in,
   output logic [WIDTH-1:0] out
);
  assign out = ~in;
endmodule

module std_and #(
    parameter WIDTH = 32
) (
   input wire               logic [WIDTH-1:0] left,
   input wire               logic [WIDTH-1:0] right,
   output logic [WIDTH-1:0] out
);
  assign out = left & right;
endmodule

module std_or #(
    parameter WIDTH = 32
) (
   input wire               logic [WIDTH-1:0] left,
   input wire               logic [WIDTH-1:0] right,
   output logic [WIDTH-1:0] out
);
  assign out = left | right;
endmodule

module std_xor #(
    parameter WIDTH = 32
) (
   input wire               logic [WIDTH-1:0] left,
   input wire               logic [WIDTH-1:0] right,
   output logic [WIDTH-1:0] out
);
  assign out = left ^ right;
endmodule

module std_add #(
    parameter WIDTH = 32
) (
   input wire               logic [WIDTH-1:0] left,
   input wire               logic [WIDTH-1:0] right,
   output logic [WIDTH-1:0] out
);
  assign out = left + right;
endmodule

module std_sub #(
    parameter WIDTH = 32
) (
   input wire               logic [WIDTH-1:0] left,
   input wire               logic [WIDTH-1:0] right,
   output logic [WIDTH-1:0] out
);
  assign out = left - right;
endmodule

module std_gt #(
    parameter WIDTH = 32
) (
   input wire   logic [WIDTH-1:0] left,
   input wire   logic [WIDTH-1:0] right,
   output logic out
);
  assign out = left > right;
endmodule

module std_lt #(
    parameter WIDTH = 32
) (
   input wire   logic [WIDTH-1:0] left,
   input wire   logic [WIDTH-1:0] right,
   output logic out
);
  assign out = left < right;
endmodule

module std_eq #(
    parameter WIDTH = 32
) (
   input wire   logic [WIDTH-1:0] left,
   input wire   logic [WIDTH-1:0] right,
   output logic out
);
  assign out = left == right;
endmodule

module std_neq #(
    parameter WIDTH = 32
) (
   input wire   logic [WIDTH-1:0] left,
   input wire   logic [WIDTH-1:0] right,
   output logic out
);
  assign out = left != right;
endmodule

module std_ge #(
    parameter WIDTH = 32
) (
    input wire   logic [WIDTH-1:0] left,
    input wire   logic [WIDTH-1:0] right,
    output logic out
);
  assign out = left >= right;
endmodule

module std_le #(
    parameter WIDTH = 32
) (
   input wire   logic [WIDTH-1:0] left,
   input wire   logic [WIDTH-1:0] right,
   output logic out
);
  assign out = left <= right;
endmodule

module std_lsh #(
    parameter WIDTH = 32
) (
   input wire               logic [WIDTH-1:0] left,
   input wire               logic [WIDTH-1:0] right,
   output logic [WIDTH-1:0] out
);
  assign out = left << right;
endmodule

module std_rsh #(
    parameter WIDTH = 32
) (
   input wire               logic [WIDTH-1:0] left,
   input wire               logic [WIDTH-1:0] right,
   output logic [WIDTH-1:0] out
);
  assign out = left >> right;
endmodule

/// this primitive is intended to be used
/// for lowering purposes (not in source programs)
module std_mux #(
    parameter WIDTH = 32
) (
   input wire               logic cond,
   input wire               logic [WIDTH-1:0] tru,
   input wire               logic [WIDTH-1:0] fal,
   output logic [WIDTH-1:0] out
);
  assign out = cond ? tru : fal;
endmodule

/// Memories
module std_reg #(
    parameter WIDTH = 32
) (
   input wire [ WIDTH-1:0]    in,
   input wire                 write_en,
   input wire                 clk,
   input wire                 reset,
    // output
   output logic [WIDTH - 1:0] out,
   output logic               done
);

  always_ff @(posedge clk) begin
    if (reset) begin
       out <= 0;
       done <= 0;
    end else if (write_en) begin
      out <= in;
      done <= 1'd1;
    end else done <= 1'd0;
  end
endmodule

module std_mem_d1 #(
    parameter WIDTH = 32,
    parameter SIZE = 16,
    parameter IDX_SIZE = 4
) (
   input wire                logic [IDX_SIZE-1:0] addr0,
   input wire                logic [ WIDTH-1:0] write_data,
   input wire                logic write_en,
   input wire                logic clk,
   output logic [ WIDTH-1:0] read_data,
   output logic              done
);

  logic [WIDTH-1:0] mem[SIZE-1:0];

  /* verilator lint_off WIDTH */
  assign read_data = mem[addr0];
  always_ff @(posedge clk) begin
    if (write_en) begin
      mem[addr0] <= write_data;
      done <= 1'd1;
    end else done <= 1'd0;
  end

  // Check for out of bounds access
  `ifdef VERILATOR
    always_comb begin
      if (addr0 >= SIZE)
        $error(
          "std_mem_d1: Out of bounds access\n",
          "addr0: %0d\n", addr0,
          "SIZE: %0d", SIZE
        );
    end
  `endif
endmodule

module std_mem_d2 #(
    parameter WIDTH = 32,
    parameter D0_SIZE = 16,
    parameter D1_SIZE = 16,
    parameter D0_IDX_SIZE = 4,
    parameter D1_IDX_SIZE = 4
) (
   input wire                logic [D0_IDX_SIZE-1:0] addr0,
   input wire                logic [D1_IDX_SIZE-1:0] addr1,
   input wire                logic [ WIDTH-1:0] write_data,
   input wire                logic write_en,
   input wire                logic clk,
   output logic [ WIDTH-1:0] read_data,
   output logic              done
);

  /* verilator lint_off WIDTH */
  logic [WIDTH-1:0] mem[D0_SIZE-1:0][D1_SIZE-1:0];

  assign read_data = mem[addr0][addr1];
  always_ff @(posedge clk) begin
    if (write_en) begin
      mem[addr0][addr1] <= write_data;
      done <= 1'd1;
    end else done <= 1'd0;
  end

  // Check for out of bounds access
  `ifdef VERILATOR
    always_comb begin
      if (addr0 >= D0_SIZE)
        $error(
          "std_mem_d2: Out of bounds access\n",
          "addr0: %0d\n", addr0,
          "D0_SIZE: %0d", D0_SIZE
        );
      if (addr1 >= D1_SIZE)
        $error(
          "std_mem_d2: Out of bounds access\n",
          "addr1: %0d\n", addr1,
          "D1_SIZE: %0d", D1_SIZE
        );
    end
  `endif
endmodule

module std_mem_d3 #(
    parameter WIDTH = 32,
    parameter D0_SIZE = 16,
    parameter D1_SIZE = 16,
    parameter D2_SIZE = 16,
    parameter D0_IDX_SIZE = 4,
    parameter D1_IDX_SIZE = 4,
    parameter D2_IDX_SIZE = 4
) (
   input wire                logic [D0_IDX_SIZE-1:0] addr0,
   input wire                logic [D1_IDX_SIZE-1:0] addr1,
   input wire                logic [D2_IDX_SIZE-1:0] addr2,
   input wire                logic [ WIDTH-1:0] write_data,
   input wire                logic write_en,
   input wire                logic clk,
   output logic [ WIDTH-1:0] read_data,
   output logic              done
);

  /* verilator lint_off WIDTH */
  logic [WIDTH-1:0] mem[D0_SIZE-1:0][D1_SIZE-1:0][D2_SIZE-1:0];

  assign read_data = mem[addr0][addr1][addr2];
  always_ff @(posedge clk) begin
    if (write_en) begin
      mem[addr0][addr1][addr2] <= write_data;
      done <= 1'd1;
    end else done <= 1'd0;
  end

  // Check for out of bounds access
  `ifdef VERILATOR
    always_comb begin
      if (addr0 >= D0_SIZE)
        $error(
          "std_mem_d3: Out of bounds access\n",
          "addr0: %0d\n", addr0,
          "D0_SIZE: %0d", D0_SIZE
        );
      if (addr1 >= D1_SIZE)
        $error(
          "std_mem_d3: Out of bounds access\n",
          "addr1: %0d\n", addr1,
          "D1_SIZE: %0d", D1_SIZE
        );
      if (addr2 >= D2_SIZE)
        $error(
          "std_mem_d3: Out of bounds access\n",
          "addr2: %0d\n", addr2,
          "D2_SIZE: %0d", D2_SIZE
        );
    end
  `endif
endmodule

module std_mem_d4 #(
    parameter WIDTH = 32,
    parameter D0_SIZE = 16,
    parameter D1_SIZE = 16,
    parameter D2_SIZE = 16,
    parameter D3_SIZE = 16,
    parameter D0_IDX_SIZE = 4,
    parameter D1_IDX_SIZE = 4,
    parameter D2_IDX_SIZE = 4,
    parameter D3_IDX_SIZE = 4
) (
   input wire                logic [D0_IDX_SIZE-1:0] addr0,
   input wire                logic [D1_IDX_SIZE-1:0] addr1,
   input wire                logic [D2_IDX_SIZE-1:0] addr2,
   input wire                logic [D3_IDX_SIZE-1:0] addr3,
   input wire                logic [ WIDTH-1:0] write_data,
   input wire                logic write_en,
   input wire                logic clk,
   output logic [ WIDTH-1:0] read_data,
   output logic              done
);

  /* verilator lint_off WIDTH */
  logic [WIDTH-1:0] mem[D0_SIZE-1:0][D1_SIZE-1:0][D2_SIZE-1:0][D3_SIZE-1:0];

  assign read_data = mem[addr0][addr1][addr2][addr3];
  always_ff @(posedge clk) begin
    if (write_en) begin
      mem[addr0][addr1][addr2][addr3] <= write_data;
      done <= 1'd1;
    end else done <= 1'd0;
  end

  // Check for out of bounds access
  `ifdef VERILATOR
    always_comb begin
      if (addr0 >= D0_SIZE)
        $error(
          "std_mem_d4: Out of bounds access\n",
          "addr0: %0d\n", addr0,
          "D0_SIZE: %0d", D0_SIZE
        );
      if (addr1 >= D1_SIZE)
        $error(
          "std_mem_d4: Out of bounds access\n",
          "addr1: %0d\n", addr1,
          "D1_SIZE: %0d", D1_SIZE
        );
      if (addr2 >= D2_SIZE)
        $error(
          "std_mem_d4: Out of bounds access\n",
          "addr2: %0d\n", addr2,
          "D2_SIZE: %0d", D2_SIZE
        );
      if (addr3 >= D3_SIZE)
        $error(
          "std_mem_d4: Out of bounds access\n",
          "addr3: %0d\n", addr3,
          "D3_SIZE: %0d", D3_SIZE
        );
    end
  `endif
endmodule

`default_nettype wire
module main (
    input logic go,
    input logic clk,
    input logic reset,
    output logic done
);
    string DATA;
    int CODE;
    initial begin
        CODE = $value$plusargs("DATA=%s", DATA);
        $display("DATA (path to meminit files): %s", DATA);
        $readmemh({DATA, "/out.dat"}, out.mem);
    end
    final begin
        $writememh({DATA, "/out.out"}, out.mem);
    end
    logic [2:0] out_addr0;
    logic [31:0] out_write_data;
    logic out_write_en;
    logic out_clk;
    logic [31:0] out_read_data;
    logic out_done;
    logic [31:0] val_in;
    logic val_write_en;
    logic val_clk;
    logic val_reset;
    logic [31:0] val_out;
    logic val_done;
    logic [31:0] add_0_left;
    logic [31:0] add_0_right;
    logic [31:0] add_0_out;
    logic [2:0] addr_in;
    logic addr_write_en;
    logic addr_clk;
    logic addr_reset;
    logic [2:0] addr_out;
    logic addr_done;
    logic [2:0] add_1_left;
    logic [2:0] add_1_right;
    logic [2:0] add_1_out;
    logic [2:0] lt_left;
    logic [2:0] lt_right;
    logic lt_out;
    logic [31:0] no_use_in;
    logic no_use_write_en;
    logic no_use_clk;
    logic no_use_reset;
    logic [31:0] no_use_out;
    logic no_use_done;
    logic [31:0] barrier_1_in_0;
    logic [31:0] barrier_1_in_1;
    logic barrier_1_read_en_0;
    logic barrier_1_read_en_1;
    logic barrier_1_write_en_0;
    logic barrier_1_write_en_1;
    logic barrier_1_clk;
    logic barrier_1_reset;
    logic [31:0] barrier_1_out_0;
    logic [31:0] barrier_1_out_1;
    logic barrier_1_write_done_0;
    logic barrier_1_write_done_1;
    logic barrier_1_read_done_0;
    logic barrier_1_read_done_1;
    logic [31:0] barrier_1_peek;
    logic [31:0] eq_1_left;
    logic [31:0] eq_1_right;
    logic eq_1_out;
    logic once_1_0_in;
    logic once_1_0_write_en;
    logic once_1_0_clk;
    logic once_1_0_reset;
    logic once_1_0_out;
    logic once_1_0_done;
    logic [31:0] save_1_0_in;
    logic save_1_0_write_en;
    logic save_1_0_clk;
    logic save_1_0_reset;
    logic [31:0] save_1_0_out;
    logic save_1_0_done;
    logic [31:0] incr_1_0_left;
    logic [31:0] incr_1_0_right;
    logic [31:0] incr_1_0_out;
    logic once_1_1_in;
    logic once_1_1_write_en;
    logic once_1_1_clk;
    logic once_1_1_reset;
    logic once_1_1_out;
    logic once_1_1_done;
    logic [31:0] save_1_1_in;
    logic save_1_1_write_en;
    logic save_1_1_clk;
    logic save_1_1_reset;
    logic [31:0] save_1_1_out;
    logic save_1_1_done;
    logic [31:0] barrier_2_in_0;
    logic [31:0] barrier_2_in_1;
    logic barrier_2_read_en_0;
    logic barrier_2_read_en_1;
    logic barrier_2_write_en_0;
    logic barrier_2_write_en_1;
    logic barrier_2_clk;
    logic barrier_2_reset;
    logic [31:0] barrier_2_out_0;
    logic [31:0] barrier_2_out_1;
    logic barrier_2_write_done_0;
    logic barrier_2_write_done_1;
    logic barrier_2_read_done_0;
    logic barrier_2_read_done_1;
    logic [31:0] barrier_2_peek;
    logic [31:0] eq_2_left;
    logic [31:0] eq_2_right;
    logic eq_2_out;
    logic once_2_0_in;
    logic once_2_0_write_en;
    logic once_2_0_clk;
    logic once_2_0_reset;
    logic once_2_0_out;
    logic once_2_0_done;
    logic [31:0] save_2_0_in;
    logic save_2_0_write_en;
    logic save_2_0_clk;
    logic save_2_0_reset;
    logic [31:0] save_2_0_out;
    logic save_2_0_done;
    logic once_2_1_in;
    logic once_2_1_write_en;
    logic once_2_1_clk;
    logic once_2_1_reset;
    logic once_2_1_out;
    logic once_2_1_done;
    logic [31:0] save_2_1_in;
    logic save_2_1_write_en;
    logic save_2_1_clk;
    logic save_2_1_reset;
    logic [31:0] save_2_1_out;
    logic save_2_1_done;
    logic comb_reg_in;
    logic comb_reg_write_en;
    logic comb_reg_clk;
    logic comb_reg_reset;
    logic comb_reg_out;
    logic comb_reg_done;
    logic pd_in;
    logic pd_write_en;
    logic pd_clk;
    logic pd_reset;
    logic pd_out;
    logic pd_done;
    logic pd0_in;
    logic pd0_write_en;
    logic pd0_clk;
    logic pd0_reset;
    logic pd0_out;
    logic pd0_done;
    logic [3:0] fsm_in;
    logic fsm_write_en;
    logic fsm_clk;
    logic fsm_reset;
    logic [3:0] fsm_out;
    logic fsm_done;
    logic pd1_in;
    logic pd1_write_en;
    logic pd1_clk;
    logic pd1_reset;
    logic pd1_out;
    logic pd1_done;
    logic [3:0] fsm0_in;
    logic fsm0_write_en;
    logic fsm0_clk;
    logic fsm0_reset;
    logic [3:0] fsm0_out;
    logic fsm0_done;
    logic pd2_in;
    logic pd2_write_en;
    logic pd2_clk;
    logic pd2_reset;
    logic pd2_out;
    logic pd2_done;
    logic [1:0] fsm1_in;
    logic fsm1_write_en;
    logic fsm1_clk;
    logic fsm1_reset;
    logic [1:0] fsm1_out;
    logic fsm1_done;
    logic no_op_go_in;
    logic no_op_go_out;
    logic no_op_done_in;
    logic no_op_done_out;
    logic incr_val_go_in;
    logic incr_val_go_out;
    logic incr_val_done_in;
    logic incr_val_done_out;
    logic reg_to_mem_go_in;
    logic reg_to_mem_go_out;
    logic reg_to_mem_done_in;
    logic reg_to_mem_done_out;
    logic incr_idx_go_in;
    logic incr_idx_go_out;
    logic incr_idx_done_in;
    logic incr_idx_done_out;
    logic incr_barrier_1_0_go_in;
    logic incr_barrier_1_0_go_out;
    logic incr_barrier_1_0_done_in;
    logic incr_barrier_1_0_done_out;
    logic write_barrier_1_0_go_in;
    logic write_barrier_1_0_go_out;
    logic write_barrier_1_0_done_in;
    logic write_barrier_1_0_done_out;
    logic incr_barrier_1_1_go_in;
    logic incr_barrier_1_1_go_out;
    logic incr_barrier_1_1_done_in;
    logic incr_barrier_1_1_done_out;
    logic write_barrier_1_1_go_in;
    logic write_barrier_1_1_go_out;
    logic write_barrier_1_1_done_in;
    logic write_barrier_1_1_done_out;
    logic wait_1_go_in;
    logic wait_1_go_out;
    logic wait_1_done_in;
    logic wait_1_done_out;
    logic restore_1_go_in;
    logic restore_1_go_out;
    logic restore_1_done_in;
    logic restore_1_done_out;
    logic clear_barrier_1_go_in;
    logic clear_barrier_1_go_out;
    logic clear_barrier_1_done_in;
    logic clear_barrier_1_done_out;
    logic wait_restore_1_go_in;
    logic wait_restore_1_go_out;
    logic wait_restore_1_done_in;
    logic wait_restore_1_done_out;
    logic incr_barrier_2_0_go_in;
    logic incr_barrier_2_0_go_out;
    logic incr_barrier_2_0_done_in;
    logic incr_barrier_2_0_done_out;
    logic write_barrier_2_0_go_in;
    logic write_barrier_2_0_go_out;
    logic write_barrier_2_0_done_in;
    logic write_barrier_2_0_done_out;
    logic incr_barrier_2_1_go_in;
    logic incr_barrier_2_1_go_out;
    logic incr_barrier_2_1_done_in;
    logic incr_barrier_2_1_done_out;
    logic write_barrier_2_1_go_in;
    logic write_barrier_2_1_go_out;
    logic write_barrier_2_1_done_in;
    logic write_barrier_2_1_done_out;
    logic wait_2_go_in;
    logic wait_2_go_out;
    logic wait_2_done_in;
    logic wait_2_done_out;
    logic restore_2_go_in;
    logic restore_2_go_out;
    logic restore_2_done_in;
    logic restore_2_done_out;
    logic clear_barrier_2_go_in;
    logic clear_barrier_2_go_out;
    logic clear_barrier_2_done_in;
    logic clear_barrier_2_done_out;
    logic wait_restore_2_go_in;
    logic wait_restore_2_go_out;
    logic wait_restore_2_done_in;
    logic wait_restore_2_done_out;
    logic cmp0_go_in;
    logic cmp0_go_out;
    logic cmp0_done_in;
    logic cmp0_done_out;
    logic par_go_in;
    logic par_go_out;
    logic par_done_in;
    logic par_done_out;
    logic par0_go_in;
    logic par0_go_out;
    logic par0_done_in;
    logic par0_done_out;
    logic tdcc_go_in;
    logic tdcc_go_out;
    logic tdcc_done_in;
    logic tdcc_done_out;
    logic tdcc0_go_in;
    logic tdcc0_go_out;
    logic tdcc0_done_in;
    logic tdcc0_done_out;
    logic tdcc1_go_in;
    logic tdcc1_go_out;
    logic tdcc1_done_in;
    logic tdcc1_done_out;
    initial begin
        out_addr0 = 3'd0;
        out_write_data = 32'd0;
        out_write_en = 1'd0;
        out_clk = 1'd0;
        val_in = 32'd0;
        val_write_en = 1'd0;
        val_clk = 1'd0;
        val_reset = 1'd0;
        add_0_left = 32'd0;
        add_0_right = 32'd0;
        addr_in = 3'd0;
        addr_write_en = 1'd0;
        addr_clk = 1'd0;
        addr_reset = 1'd0;
        add_1_left = 3'd0;
        add_1_right = 3'd0;
        lt_left = 3'd0;
        lt_right = 3'd0;
        no_use_in = 32'd0;
        no_use_write_en = 1'd0;
        no_use_clk = 1'd0;
        no_use_reset = 1'd0;
        barrier_1_in_0 = 32'd0;
        barrier_1_in_1 = 32'd0;
        barrier_1_read_en_0 = 1'd0;
        barrier_1_read_en_1 = 1'd0;
        barrier_1_write_en_0 = 1'd0;
        barrier_1_write_en_1 = 1'd0;
        barrier_1_clk = 1'd0;
        barrier_1_reset = 1'd0;
        eq_1_left = 32'd0;
        eq_1_right = 32'd0;
        once_1_0_in = 1'd0;
        once_1_0_write_en = 1'd0;
        once_1_0_clk = 1'd0;
        once_1_0_reset = 1'd0;
        save_1_0_in = 32'd0;
        save_1_0_write_en = 1'd0;
        save_1_0_clk = 1'd0;
        save_1_0_reset = 1'd0;
        incr_1_0_left = 32'd0;
        incr_1_0_right = 32'd0;
        once_1_1_in = 1'd0;
        once_1_1_write_en = 1'd0;
        once_1_1_clk = 1'd0;
        once_1_1_reset = 1'd0;
        save_1_1_in = 32'd0;
        save_1_1_write_en = 1'd0;
        save_1_1_clk = 1'd0;
        save_1_1_reset = 1'd0;
        barrier_2_in_0 = 32'd0;
        barrier_2_in_1 = 32'd0;
        barrier_2_read_en_0 = 1'd0;
        barrier_2_read_en_1 = 1'd0;
        barrier_2_write_en_0 = 1'd0;
        barrier_2_write_en_1 = 1'd0;
        barrier_2_clk = 1'd0;
        barrier_2_reset = 1'd0;
        eq_2_left = 32'd0;
        eq_2_right = 32'd0;
        once_2_0_in = 1'd0;
        once_2_0_write_en = 1'd0;
        once_2_0_clk = 1'd0;
        once_2_0_reset = 1'd0;
        save_2_0_in = 32'd0;
        save_2_0_write_en = 1'd0;
        save_2_0_clk = 1'd0;
        save_2_0_reset = 1'd0;
        once_2_1_in = 1'd0;
        once_2_1_write_en = 1'd0;
        once_2_1_clk = 1'd0;
        once_2_1_reset = 1'd0;
        save_2_1_in = 32'd0;
        save_2_1_write_en = 1'd0;
        save_2_1_clk = 1'd0;
        save_2_1_reset = 1'd0;
        comb_reg_in = 1'd0;
        comb_reg_write_en = 1'd0;
        comb_reg_clk = 1'd0;
        comb_reg_reset = 1'd0;
        pd_in = 1'd0;
        pd_write_en = 1'd0;
        pd_clk = 1'd0;
        pd_reset = 1'd0;
        pd0_in = 1'd0;
        pd0_write_en = 1'd0;
        pd0_clk = 1'd0;
        pd0_reset = 1'd0;
        fsm_in = 4'd0;
        fsm_write_en = 1'd0;
        fsm_clk = 1'd0;
        fsm_reset = 1'd0;
        pd1_in = 1'd0;
        pd1_write_en = 1'd0;
        pd1_clk = 1'd0;
        pd1_reset = 1'd0;
        fsm0_in = 4'd0;
        fsm0_write_en = 1'd0;
        fsm0_clk = 1'd0;
        fsm0_reset = 1'd0;
        pd2_in = 1'd0;
        pd2_write_en = 1'd0;
        pd2_clk = 1'd0;
        pd2_reset = 1'd0;
        fsm1_in = 2'd0;
        fsm1_write_en = 1'd0;
        fsm1_clk = 1'd0;
        fsm1_reset = 1'd0;
        no_op_go_in = 1'd0;
        no_op_done_in = 1'd0;
        incr_val_go_in = 1'd0;
        incr_val_done_in = 1'd0;
        reg_to_mem_go_in = 1'd0;
        reg_to_mem_done_in = 1'd0;
        incr_idx_go_in = 1'd0;
        incr_idx_done_in = 1'd0;
        incr_barrier_1_0_go_in = 1'd0;
        incr_barrier_1_0_done_in = 1'd0;
        write_barrier_1_0_go_in = 1'd0;
        write_barrier_1_0_done_in = 1'd0;
        incr_barrier_1_1_go_in = 1'd0;
        incr_barrier_1_1_done_in = 1'd0;
        write_barrier_1_1_go_in = 1'd0;
        write_barrier_1_1_done_in = 1'd0;
        wait_1_go_in = 1'd0;
        wait_1_done_in = 1'd0;
        restore_1_go_in = 1'd0;
        restore_1_done_in = 1'd0;
        clear_barrier_1_go_in = 1'd0;
        clear_barrier_1_done_in = 1'd0;
        wait_restore_1_go_in = 1'd0;
        wait_restore_1_done_in = 1'd0;
        incr_barrier_2_0_go_in = 1'd0;
        incr_barrier_2_0_done_in = 1'd0;
        write_barrier_2_0_go_in = 1'd0;
        write_barrier_2_0_done_in = 1'd0;
        incr_barrier_2_1_go_in = 1'd0;
        incr_barrier_2_1_done_in = 1'd0;
        write_barrier_2_1_go_in = 1'd0;
        write_barrier_2_1_done_in = 1'd0;
        wait_2_go_in = 1'd0;
        wait_2_done_in = 1'd0;
        restore_2_go_in = 1'd0;
        restore_2_done_in = 1'd0;
        clear_barrier_2_go_in = 1'd0;
        clear_barrier_2_done_in = 1'd0;
        wait_restore_2_go_in = 1'd0;
        wait_restore_2_done_in = 1'd0;
        cmp0_go_in = 1'd0;
        cmp0_done_in = 1'd0;
        par_go_in = 1'd0;
        par_done_in = 1'd0;
        par0_go_in = 1'd0;
        par0_done_in = 1'd0;
        tdcc_go_in = 1'd0;
        tdcc_done_in = 1'd0;
        tdcc0_go_in = 1'd0;
        tdcc0_done_in = 1'd0;
        tdcc1_go_in = 1'd0;
        tdcc1_done_in = 1'd0;
    end
    std_mem_d1 # (
        .IDX_SIZE(3),
        .SIZE(5),
        .WIDTH(32)
    ) out (
        .addr0(out_addr0),
        .clk(out_clk),
        .done(out_done),
        .read_data(out_read_data),
        .write_data(out_write_data),
        .write_en(out_write_en)
    );
    std_reg # (
        .WIDTH(32)
    ) val (
        .clk(val_clk),
        .done(val_done),
        .in(val_in),
        .out(val_out),
        .reset(val_reset),
        .write_en(val_write_en)
    );
    std_add # (
        .WIDTH(32)
    ) add_0 (
        .left(add_0_left),
        .out(add_0_out),
        .right(add_0_right)
    );
    std_reg # (
        .WIDTH(3)
    ) addr (
        .clk(addr_clk),
        .done(addr_done),
        .in(addr_in),
        .out(addr_out),
        .reset(addr_reset),
        .write_en(addr_write_en)
    );
    std_add # (
        .WIDTH(3)
    ) add_1 (
        .left(add_1_left),
        .out(add_1_out),
        .right(add_1_right)
    );
    std_lt # (
        .WIDTH(3)
    ) lt (
        .left(lt_left),
        .out(lt_out),
        .right(lt_right)
    );
    std_reg # (
        .WIDTH(32)
    ) no_use (
        .clk(no_use_clk),
        .done(no_use_done),
        .in(no_use_in),
        .out(no_use_out),
        .reset(no_use_reset),
        .write_en(no_use_write_en)
    );
    std_sync_reg # (
        .WIDTH(32)
    ) barrier_1 (
        .clk(barrier_1_clk),
        .in_0(barrier_1_in_0),
        .in_1(barrier_1_in_1),
        .out_0(barrier_1_out_0),
        .out_1(barrier_1_out_1),
        .peek(barrier_1_peek),
        .read_done_0(barrier_1_read_done_0),
        .read_done_1(barrier_1_read_done_1),
        .read_en_0(barrier_1_read_en_0),
        .read_en_1(barrier_1_read_en_1),
        .reset(barrier_1_reset),
        .write_done_0(barrier_1_write_done_0),
        .write_done_1(barrier_1_write_done_1),
        .write_en_0(barrier_1_write_en_0),
        .write_en_1(barrier_1_write_en_1)
    );
    std_eq # (
        .WIDTH(32)
    ) eq_1 (
        .left(eq_1_left),
        .out(eq_1_out),
        .right(eq_1_right)
    );
    std_reg # (
        .WIDTH(1)
    ) once_1_0 (
        .clk(once_1_0_clk),
        .done(once_1_0_done),
        .in(once_1_0_in),
        .out(once_1_0_out),
        .reset(once_1_0_reset),
        .write_en(once_1_0_write_en)
    );
    std_reg # (
        .WIDTH(32)
    ) save_1_0 (
        .clk(save_1_0_clk),
        .done(save_1_0_done),
        .in(save_1_0_in),
        .out(save_1_0_out),
        .reset(save_1_0_reset),
        .write_en(save_1_0_write_en)
    );
    std_add # (
        .WIDTH(32)
    ) incr_1_0 (
        .left(incr_1_0_left),
        .out(incr_1_0_out),
        .right(incr_1_0_right)
    );
    std_reg # (
        .WIDTH(1)
    ) once_1_1 (
        .clk(once_1_1_clk),
        .done(once_1_1_done),
        .in(once_1_1_in),
        .out(once_1_1_out),
        .reset(once_1_1_reset),
        .write_en(once_1_1_write_en)
    );
    std_reg # (
        .WIDTH(32)
    ) save_1_1 (
        .clk(save_1_1_clk),
        .done(save_1_1_done),
        .in(save_1_1_in),
        .out(save_1_1_out),
        .reset(save_1_1_reset),
        .write_en(save_1_1_write_en)
    );
    std_sync_reg # (
        .WIDTH(32)
    ) barrier_2 (
        .clk(barrier_2_clk),
        .in_0(barrier_2_in_0),
        .in_1(barrier_2_in_1),
        .out_0(barrier_2_out_0),
        .out_1(barrier_2_out_1),
        .peek(barrier_2_peek),
        .read_done_0(barrier_2_read_done_0),
        .read_done_1(barrier_2_read_done_1),
        .read_en_0(barrier_2_read_en_0),
        .read_en_1(barrier_2_read_en_1),
        .reset(barrier_2_reset),
        .write_done_0(barrier_2_write_done_0),
        .write_done_1(barrier_2_write_done_1),
        .write_en_0(barrier_2_write_en_0),
        .write_en_1(barrier_2_write_en_1)
    );
    std_eq # (
        .WIDTH(32)
    ) eq_2 (
        .left(eq_2_left),
        .out(eq_2_out),
        .right(eq_2_right)
    );
    std_reg # (
        .WIDTH(1)
    ) once_2_0 (
        .clk(once_2_0_clk),
        .done(once_2_0_done),
        .in(once_2_0_in),
        .out(once_2_0_out),
        .reset(once_2_0_reset),
        .write_en(once_2_0_write_en)
    );
    std_reg # (
        .WIDTH(32)
    ) save_2_0 (
        .clk(save_2_0_clk),
        .done(save_2_0_done),
        .in(save_2_0_in),
        .out(save_2_0_out),
        .reset(save_2_0_reset),
        .write_en(save_2_0_write_en)
    );
    std_reg # (
        .WIDTH(1)
    ) once_2_1 (
        .clk(once_2_1_clk),
        .done(once_2_1_done),
        .in(once_2_1_in),
        .out(once_2_1_out),
        .reset(once_2_1_reset),
        .write_en(once_2_1_write_en)
    );
    std_reg # (
        .WIDTH(32)
    ) save_2_1 (
        .clk(save_2_1_clk),
        .done(save_2_1_done),
        .in(save_2_1_in),
        .out(save_2_1_out),
        .reset(save_2_1_reset),
        .write_en(save_2_1_write_en)
    );
    std_reg # (
        .WIDTH(1)
    ) comb_reg (
        .clk(comb_reg_clk),
        .done(comb_reg_done),
        .in(comb_reg_in),
        .out(comb_reg_out),
        .reset(comb_reg_reset),
        .write_en(comb_reg_write_en)
    );
    std_reg # (
        .WIDTH(1)
    ) pd (
        .clk(pd_clk),
        .done(pd_done),
        .in(pd_in),
        .out(pd_out),
        .reset(pd_reset),
        .write_en(pd_write_en)
    );
    std_reg # (
        .WIDTH(1)
    ) pd0 (
        .clk(pd0_clk),
        .done(pd0_done),
        .in(pd0_in),
        .out(pd0_out),
        .reset(pd0_reset),
        .write_en(pd0_write_en)
    );
    std_reg # (
        .WIDTH(4)
    ) fsm (
        .clk(fsm_clk),
        .done(fsm_done),
        .in(fsm_in),
        .out(fsm_out),
        .reset(fsm_reset),
        .write_en(fsm_write_en)
    );
    std_reg # (
        .WIDTH(1)
    ) pd1 (
        .clk(pd1_clk),
        .done(pd1_done),
        .in(pd1_in),
        .out(pd1_out),
        .reset(pd1_reset),
        .write_en(pd1_write_en)
    );
    std_reg # (
        .WIDTH(4)
    ) fsm0 (
        .clk(fsm0_clk),
        .done(fsm0_done),
        .in(fsm0_in),
        .out(fsm0_out),
        .reset(fsm0_reset),
        .write_en(fsm0_write_en)
    );
    std_reg # (
        .WIDTH(1)
    ) pd2 (
        .clk(pd2_clk),
        .done(pd2_done),
        .in(pd2_in),
        .out(pd2_out),
        .reset(pd2_reset),
        .write_en(pd2_write_en)
    );
    std_reg # (
        .WIDTH(2)
    ) fsm1 (
        .clk(fsm1_clk),
        .done(fsm1_done),
        .in(fsm1_in),
        .out(fsm1_out),
        .reset(fsm1_reset),
        .write_en(fsm1_write_en)
    );
    std_wire # (
        .WIDTH(1)
    ) no_op_go (
        .in(no_op_go_in),
        .out(no_op_go_out)
    );
    std_wire # (
        .WIDTH(1)
    ) no_op_done (
        .in(no_op_done_in),
        .out(no_op_done_out)
    );
    std_wire # (
        .WIDTH(1)
    ) incr_val_go (
        .in(incr_val_go_in),
        .out(incr_val_go_out)
    );
    std_wire # (
        .WIDTH(1)
    ) incr_val_done (
        .in(incr_val_done_in),
        .out(incr_val_done_out)
    );
    std_wire # (
        .WIDTH(1)
    ) reg_to_mem_go (
        .in(reg_to_mem_go_in),
        .out(reg_to_mem_go_out)
    );
    std_wire # (
        .WIDTH(1)
    ) reg_to_mem_done (
        .in(reg_to_mem_done_in),
        .out(reg_to_mem_done_out)
    );
    std_wire # (
        .WIDTH(1)
    ) incr_idx_go (
        .in(incr_idx_go_in),
        .out(incr_idx_go_out)
    );
    std_wire # (
        .WIDTH(1)
    ) incr_idx_done (
        .in(incr_idx_done_in),
        .out(incr_idx_done_out)
    );
    std_wire # (
        .WIDTH(1)
    ) incr_barrier_1_0_go (
        .in(incr_barrier_1_0_go_in),
        .out(incr_barrier_1_0_go_out)
    );
    std_wire # (
        .WIDTH(1)
    ) incr_barrier_1_0_done (
        .in(incr_barrier_1_0_done_in),
        .out(incr_barrier_1_0_done_out)
    );
    std_wire # (
        .WIDTH(1)
    ) write_barrier_1_0_go (
        .in(write_barrier_1_0_go_in),
        .out(write_barrier_1_0_go_out)
    );
    std_wire # (
        .WIDTH(1)
    ) write_barrier_1_0_done (
        .in(write_barrier_1_0_done_in),
        .out(write_barrier_1_0_done_out)
    );
    std_wire # (
        .WIDTH(1)
    ) incr_barrier_1_1_go (
        .in(incr_barrier_1_1_go_in),
        .out(incr_barrier_1_1_go_out)
    );
    std_wire # (
        .WIDTH(1)
    ) incr_barrier_1_1_done (
        .in(incr_barrier_1_1_done_in),
        .out(incr_barrier_1_1_done_out)
    );
    std_wire # (
        .WIDTH(1)
    ) write_barrier_1_1_go (
        .in(write_barrier_1_1_go_in),
        .out(write_barrier_1_1_go_out)
    );
    std_wire # (
        .WIDTH(1)
    ) write_barrier_1_1_done (
        .in(write_barrier_1_1_done_in),
        .out(write_barrier_1_1_done_out)
    );
    std_wire # (
        .WIDTH(1)
    ) wait_1_go (
        .in(wait_1_go_in),
        .out(wait_1_go_out)
    );
    std_wire # (
        .WIDTH(1)
    ) wait_1_done (
        .in(wait_1_done_in),
        .out(wait_1_done_out)
    );
    std_wire # (
        .WIDTH(1)
    ) restore_1_go (
        .in(restore_1_go_in),
        .out(restore_1_go_out)
    );
    std_wire # (
        .WIDTH(1)
    ) restore_1_done (
        .in(restore_1_done_in),
        .out(restore_1_done_out)
    );
    std_wire # (
        .WIDTH(1)
    ) clear_barrier_1_go (
        .in(clear_barrier_1_go_in),
        .out(clear_barrier_1_go_out)
    );
    std_wire # (
        .WIDTH(1)
    ) clear_barrier_1_done (
        .in(clear_barrier_1_done_in),
        .out(clear_barrier_1_done_out)
    );
    std_wire # (
        .WIDTH(1)
    ) wait_restore_1_go (
        .in(wait_restore_1_go_in),
        .out(wait_restore_1_go_out)
    );
    std_wire # (
        .WIDTH(1)
    ) wait_restore_1_done (
        .in(wait_restore_1_done_in),
        .out(wait_restore_1_done_out)
    );
    std_wire # (
        .WIDTH(1)
    ) incr_barrier_2_0_go (
        .in(incr_barrier_2_0_go_in),
        .out(incr_barrier_2_0_go_out)
    );
    std_wire # (
        .WIDTH(1)
    ) incr_barrier_2_0_done (
        .in(incr_barrier_2_0_done_in),
        .out(incr_barrier_2_0_done_out)
    );
    std_wire # (
        .WIDTH(1)
    ) write_barrier_2_0_go (
        .in(write_barrier_2_0_go_in),
        .out(write_barrier_2_0_go_out)
    );
    std_wire # (
        .WIDTH(1)
    ) write_barrier_2_0_done (
        .in(write_barrier_2_0_done_in),
        .out(write_barrier_2_0_done_out)
    );
    std_wire # (
        .WIDTH(1)
    ) incr_barrier_2_1_go (
        .in(incr_barrier_2_1_go_in),
        .out(incr_barrier_2_1_go_out)
    );
    std_wire # (
        .WIDTH(1)
    ) incr_barrier_2_1_done (
        .in(incr_barrier_2_1_done_in),
        .out(incr_barrier_2_1_done_out)
    );
    std_wire # (
        .WIDTH(1)
    ) write_barrier_2_1_go (
        .in(write_barrier_2_1_go_in),
        .out(write_barrier_2_1_go_out)
    );
    std_wire # (
        .WIDTH(1)
    ) write_barrier_2_1_done (
        .in(write_barrier_2_1_done_in),
        .out(write_barrier_2_1_done_out)
    );
    std_wire # (
        .WIDTH(1)
    ) wait_2_go (
        .in(wait_2_go_in),
        .out(wait_2_go_out)
    );
    std_wire # (
        .WIDTH(1)
    ) wait_2_done (
        .in(wait_2_done_in),
        .out(wait_2_done_out)
    );
    std_wire # (
        .WIDTH(1)
    ) restore_2_go (
        .in(restore_2_go_in),
        .out(restore_2_go_out)
    );
    std_wire # (
        .WIDTH(1)
    ) restore_2_done (
        .in(restore_2_done_in),
        .out(restore_2_done_out)
    );
    std_wire # (
        .WIDTH(1)
    ) clear_barrier_2_go (
        .in(clear_barrier_2_go_in),
        .out(clear_barrier_2_go_out)
    );
    std_wire # (
        .WIDTH(1)
    ) clear_barrier_2_done (
        .in(clear_barrier_2_done_in),
        .out(clear_barrier_2_done_out)
    );
    std_wire # (
        .WIDTH(1)
    ) wait_restore_2_go (
        .in(wait_restore_2_go_in),
        .out(wait_restore_2_go_out)
    );
    std_wire # (
        .WIDTH(1)
    ) wait_restore_2_done (
        .in(wait_restore_2_done_in),
        .out(wait_restore_2_done_out)
    );
    std_wire # (
        .WIDTH(1)
    ) cmp0_go (
        .in(cmp0_go_in),
        .out(cmp0_go_out)
    );
    std_wire # (
        .WIDTH(1)
    ) cmp0_done (
        .in(cmp0_done_in),
        .out(cmp0_done_out)
    );
    std_wire # (
        .WIDTH(1)
    ) par_go (
        .in(par_go_in),
        .out(par_go_out)
    );
    std_wire # (
        .WIDTH(1)
    ) par_done (
        .in(par_done_in),
        .out(par_done_out)
    );
    std_wire # (
        .WIDTH(1)
    ) par0_go (
        .in(par0_go_in),
        .out(par0_go_out)
    );
    std_wire # (
        .WIDTH(1)
    ) par0_done (
        .in(par0_done_in),
        .out(par0_done_out)
    );
    std_wire # (
        .WIDTH(1)
    ) tdcc_go (
        .in(tdcc_go_in),
        .out(tdcc_go_out)
    );
    std_wire # (
        .WIDTH(1)
    ) tdcc_done (
        .in(tdcc_done_in),
        .out(tdcc_done_out)
    );
    std_wire # (
        .WIDTH(1)
    ) tdcc0_go (
        .in(tdcc0_go_in),
        .out(tdcc0_go_out)
    );
    std_wire # (
        .WIDTH(1)
    ) tdcc0_done (
        .in(tdcc0_done_in),
        .out(tdcc0_done_out)
    );
    std_wire # (
        .WIDTH(1)
    ) tdcc1_go (
        .in(tdcc1_go_in),
        .out(tdcc1_go_out)
    );
    std_wire # (
        .WIDTH(1)
    ) tdcc1_done (
        .in(tdcc1_done_in),
        .out(tdcc1_done_out)
    );
    assign done = tdcc1_done_out;
    assign add_0_left =
     barrier_1_read_done_1 & incr_barrier_1_1_go_out ? barrier_1_out_1 :
     barrier_2_read_done_1 & incr_barrier_2_1_go_out ? barrier_2_out_1 :
     incr_val_go_out ? val_out : 32'd0;
    assign add_0_right =
     incr_val_go_out | barrier_1_read_done_1 & incr_barrier_1_1_go_out | barrier_2_read_done_1 & incr_barrier_2_1_go_out ? 32'd1 : 32'd0;
    assign add_1_left =
     incr_idx_go_out ? addr_out : 3'd0;
    assign add_1_right =
     incr_idx_go_out ? 3'd1 : 3'd0;
    assign addr_clk = clk;
    assign addr_in =
     incr_idx_go_out ? add_1_out : 3'd0;
    assign addr_reset = reset;
    assign addr_write_en = incr_idx_go_out;
    assign barrier_1_clk = clk;
    assign barrier_1_in_0 =
     restore_1_go_out ? 32'd0 :
     write_barrier_1_0_go_out ? save_1_0_out : 32'd0;
    assign barrier_1_in_1 =
     write_barrier_1_1_go_out ? save_1_1_out : 32'd0;
    assign barrier_1_read_en_0 = ~once_1_0_out & incr_barrier_1_0_go_out | clear_barrier_1_go_out;
    assign barrier_1_read_en_1 = ~once_1_1_out & incr_barrier_1_1_go_out;
    assign barrier_1_reset = reset;
    assign barrier_1_write_en_0 = write_barrier_1_0_go_out | restore_1_go_out;
    assign barrier_1_write_en_1 = write_barrier_1_1_go_out;
    assign barrier_2_clk = clk;
    assign barrier_2_in_0 =
     restore_2_go_out ? 32'd0 :
     write_barrier_2_0_go_out ? save_2_0_out : 32'd0;
    assign barrier_2_in_1 =
     write_barrier_2_1_go_out ? save_2_1_out : 32'd0;
    assign barrier_2_read_en_0 = ~once_2_0_out & incr_barrier_2_0_go_out | clear_barrier_2_go_out;
    assign barrier_2_read_en_1 = ~once_2_1_out & incr_barrier_2_1_go_out;
    assign barrier_2_reset = reset;
    assign barrier_2_write_en_0 = write_barrier_2_0_go_out | restore_2_go_out;
    assign barrier_2_write_en_1 = write_barrier_2_1_go_out;
    assign clear_barrier_1_done_in = barrier_1_read_done_0;
    assign clear_barrier_1_go_in = ~clear_barrier_1_done_out & fsm_out == 4'd5 & tdcc_go_out;
    assign clear_barrier_2_done_in = barrier_2_read_done_0;
    assign clear_barrier_2_go_in = ~clear_barrier_2_done_out & fsm_out == 4'd12 & tdcc_go_out;
    assign cmp0_done_in = comb_reg_done;
    assign cmp0_go_in = ~cmp0_done_out & fsm_out == 4'd0 & tdcc_go_out | ~cmp0_done_out & fsm_out == 4'd14 & tdcc_go_out | ~cmp0_done_out & fsm0_out == 4'd0 & tdcc0_go_out | ~cmp0_done_out & fsm0_out == 4'd11 & tdcc0_go_out;
    assign comb_reg_clk = clk;
    assign comb_reg_in =
     cmp0_go_out ? lt_out : 1'd0;
    assign comb_reg_reset = reset;
    assign comb_reg_write_en = cmp0_go_out;
    assign eq_1_left = barrier_1_peek;
    assign eq_1_right = 32'd2;
    assign eq_2_left = barrier_2_peek;
    assign eq_2_right = 32'd2;
    assign fsm_clk = clk;
    assign fsm_in =
     fsm_out == 4'd15 ? 4'd0 :
     fsm_out == 4'd9 & incr_barrier_2_0_done_out & tdcc_go_out ? 4'd10 :
     fsm_out == 4'd10 & write_barrier_2_0_done_out & tdcc_go_out ? 4'd11 :
     fsm_out == 4'd11 & wait_2_done_out & tdcc_go_out ? 4'd12 :
     fsm_out == 4'd12 & clear_barrier_2_done_out & tdcc_go_out ? 4'd13 :
     fsm_out == 4'd13 & restore_2_done_out & tdcc_go_out ? 4'd14 :
     fsm_out == 4'd0 & cmp0_done_out & ~comb_reg_out & tdcc_go_out | fsm_out == 4'd14 & cmp0_done_out & ~comb_reg_out & tdcc_go_out ? 4'd15 :
     fsm_out == 4'd0 & cmp0_done_out & comb_reg_out & tdcc_go_out | fsm_out == 4'd14 & cmp0_done_out & comb_reg_out & tdcc_go_out ? 4'd1 :
     fsm_out == 4'd1 & no_op_done_out & tdcc_go_out ? 4'd2 :
     fsm_out == 4'd2 & incr_barrier_1_0_done_out & tdcc_go_out ? 4'd3 :
     fsm_out == 4'd3 & write_barrier_1_0_done_out & tdcc_go_out ? 4'd4 :
     fsm_out == 4'd4 & wait_1_done_out & tdcc_go_out ? 4'd5 :
     fsm_out == 4'd5 & clear_barrier_1_done_out & tdcc_go_out ? 4'd6 :
     fsm_out == 4'd6 & restore_1_done_out & tdcc_go_out ? 4'd7 :
     fsm_out == 4'd7 & reg_to_mem_done_out & tdcc_go_out ? 4'd8 :
     fsm_out == 4'd8 & incr_idx_done_out & tdcc_go_out ? 4'd9 : 4'd0;
    assign fsm_reset = reset;
    assign fsm_write_en = fsm_out == 4'd15 | fsm_out == 4'd0 & cmp0_done_out & comb_reg_out & tdcc_go_out | fsm_out == 4'd14 & cmp0_done_out & comb_reg_out & tdcc_go_out | fsm_out == 4'd1 & no_op_done_out & tdcc_go_out | fsm_out == 4'd2 & incr_barrier_1_0_done_out & tdcc_go_out | fsm_out == 4'd3 & write_barrier_1_0_done_out & tdcc_go_out | fsm_out == 4'd4 & wait_1_done_out & tdcc_go_out | fsm_out == 4'd5 & clear_barrier_1_done_out & tdcc_go_out | fsm_out == 4'd6 & restore_1_done_out & tdcc_go_out | fsm_out == 4'd7 & reg_to_mem_done_out & tdcc_go_out | fsm_out == 4'd8 & incr_idx_done_out & tdcc_go_out | fsm_out == 4'd9 & incr_barrier_2_0_done_out & tdcc_go_out | fsm_out == 4'd10 & write_barrier_2_0_done_out & tdcc_go_out | fsm_out == 4'd11 & wait_2_done_out & tdcc_go_out | fsm_out == 4'd12 & clear_barrier_2_done_out & tdcc_go_out | fsm_out == 4'd13 & restore_2_done_out & tdcc_go_out | fsm_out == 4'd0 & cmp0_done_out & ~comb_reg_out & tdcc_go_out | fsm_out == 4'd14 & cmp0_done_out & ~comb_reg_out & tdcc_go_out;
    assign fsm0_clk = clk;
    assign fsm0_in =
     fsm0_out == 4'd12 ? 4'd0 :
     fsm0_out == 4'd9 & wait_2_done_out & tdcc0_go_out ? 4'd10 :
     fsm0_out == 4'd10 & wait_restore_2_done_out & tdcc0_go_out ? 4'd11 :
     fsm0_out == 4'd0 & cmp0_done_out & ~comb_reg_out & tdcc0_go_out | fsm0_out == 4'd11 & cmp0_done_out & ~comb_reg_out & tdcc0_go_out ? 4'd12 :
     fsm0_out == 4'd0 & cmp0_done_out & comb_reg_out & tdcc0_go_out | fsm0_out == 4'd11 & cmp0_done_out & comb_reg_out & tdcc0_go_out ? 4'd1 :
     fsm0_out == 4'd1 & incr_val_done_out & tdcc0_go_out ? 4'd2 :
     fsm0_out == 4'd2 & incr_barrier_1_1_done_out & tdcc0_go_out ? 4'd3 :
     fsm0_out == 4'd3 & write_barrier_1_1_done_out & tdcc0_go_out ? 4'd4 :
     fsm0_out == 4'd4 & wait_1_done_out & tdcc0_go_out ? 4'd5 :
     fsm0_out == 4'd5 & wait_restore_1_done_out & tdcc0_go_out ? 4'd6 :
     fsm0_out == 4'd6 & no_op_done_out & tdcc0_go_out ? 4'd7 :
     fsm0_out == 4'd7 & incr_barrier_2_1_done_out & tdcc0_go_out ? 4'd8 :
     fsm0_out == 4'd8 & write_barrier_2_1_done_out & tdcc0_go_out ? 4'd9 : 4'd0;
    assign fsm0_reset = reset;
    assign fsm0_write_en = fsm0_out == 4'd12 | fsm0_out == 4'd0 & cmp0_done_out & comb_reg_out & tdcc0_go_out | fsm0_out == 4'd11 & cmp0_done_out & comb_reg_out & tdcc0_go_out | fsm0_out == 4'd1 & incr_val_done_out & tdcc0_go_out | fsm0_out == 4'd2 & incr_barrier_1_1_done_out & tdcc0_go_out | fsm0_out == 4'd3 & write_barrier_1_1_done_out & tdcc0_go_out | fsm0_out == 4'd4 & wait_1_done_out & tdcc0_go_out | fsm0_out == 4'd5 & wait_restore_1_done_out & tdcc0_go_out | fsm0_out == 4'd6 & no_op_done_out & tdcc0_go_out | fsm0_out == 4'd7 & incr_barrier_2_1_done_out & tdcc0_go_out | fsm0_out == 4'd8 & write_barrier_2_1_done_out & tdcc0_go_out | fsm0_out == 4'd9 & wait_2_done_out & tdcc0_go_out | fsm0_out == 4'd10 & wait_restore_2_done_out & tdcc0_go_out | fsm0_out == 4'd0 & cmp0_done_out & ~comb_reg_out & tdcc0_go_out | fsm0_out == 4'd11 & cmp0_done_out & ~comb_reg_out & tdcc0_go_out;
    assign fsm1_clk = clk;
    assign fsm1_in =
     fsm1_out == 2'd2 ? 2'd0 :
     fsm1_out == 2'd0 & par_done_out & tdcc1_go_out ? 2'd1 :
     fsm1_out == 2'd1 & par0_done_out & tdcc1_go_out ? 2'd2 : 2'd0;
    assign fsm1_reset = reset;
    assign fsm1_write_en = fsm1_out == 2'd2 | fsm1_out == 2'd0 & par_done_out & tdcc1_go_out | fsm1_out == 2'd1 & par0_done_out & tdcc1_go_out;
    assign incr_1_0_left =
     barrier_1_read_done_0 & incr_barrier_1_0_go_out ? barrier_1_out_0 :
     barrier_2_read_done_0 & incr_barrier_2_0_go_out ? barrier_2_out_0 : 32'd0;
    assign incr_1_0_right =
     barrier_1_read_done_0 & incr_barrier_1_0_go_out | barrier_2_read_done_0 & incr_barrier_2_0_go_out ? 32'd1 : 32'd0;
    assign incr_barrier_1_0_done_in = save_1_0_done;
    assign incr_barrier_1_0_go_in = ~incr_barrier_1_0_done_out & fsm_out == 4'd2 & tdcc_go_out;
    assign incr_barrier_1_1_done_in = save_1_1_done;
    assign incr_barrier_1_1_go_in = ~incr_barrier_1_1_done_out & fsm0_out == 4'd2 & tdcc0_go_out;
    assign incr_barrier_2_0_done_in = save_2_0_done;
    assign incr_barrier_2_0_go_in = ~incr_barrier_2_0_done_out & fsm_out == 4'd9 & tdcc_go_out;
    assign incr_barrier_2_1_done_in = save_2_1_done;
    assign incr_barrier_2_1_go_in = ~incr_barrier_2_1_done_out & fsm0_out == 4'd7 & tdcc0_go_out;
    assign incr_idx_done_in = addr_done;
    assign incr_idx_go_in = ~incr_idx_done_out & fsm_out == 4'd8 & tdcc_go_out;
    assign incr_val_done_in = val_done;
    assign incr_val_go_in = ~incr_val_done_out & fsm0_out == 4'd1 & tdcc0_go_out;
    assign lt_left =
     cmp0_go_out ? addr_out : 3'd0;
    assign lt_right =
     cmp0_go_out ? 3'd5 : 3'd0;
    assign no_op_done_in = no_use_done;
    assign no_op_go_in = ~no_op_done_out & fsm_out == 4'd1 & tdcc_go_out | ~no_op_done_out & fsm0_out == 4'd6 & tdcc0_go_out;
    assign no_use_clk = clk;
    assign no_use_in =
     no_op_go_out ? 32'd0 : 32'd0;
    assign no_use_reset = reset;
    assign no_use_write_en = no_op_go_out;
    assign once_1_0_clk = clk;
    assign once_1_0_in =
     incr_barrier_1_0_go_out ? barrier_1_read_done_0 : 1'd0;
    assign once_1_0_reset = reset;
    assign once_1_0_write_en =
     incr_barrier_1_0_go_out ? barrier_1_read_done_0 : 1'd0;
    assign once_1_1_clk = clk;
    assign once_1_1_in =
     incr_barrier_1_1_go_out ? barrier_1_read_done_1 : 1'd0;
    assign once_1_1_reset = reset;
    assign once_1_1_write_en =
     incr_barrier_1_1_go_out ? barrier_1_read_done_1 : 1'd0;
    assign once_2_0_clk = clk;
    assign once_2_0_in =
     incr_barrier_2_0_go_out ? barrier_2_read_done_0 : 1'd0;
    assign once_2_0_reset = reset;
    assign once_2_0_write_en =
     incr_barrier_2_0_go_out ? barrier_2_read_done_0 : 1'd0;
    assign once_2_1_clk = clk;
    assign once_2_1_in =
     incr_barrier_2_1_go_out ? barrier_2_read_done_1 : 1'd0;
    assign once_2_1_reset = reset;
    assign once_2_1_write_en =
     incr_barrier_2_1_go_out ? barrier_2_read_done_1 : 1'd0;
    assign out_addr0 =
     reg_to_mem_go_out ? addr_out : 3'd0;
    assign out_clk = clk;
    assign out_write_data =
     reg_to_mem_go_out ? val_out : 32'd0;
    assign out_write_en = reg_to_mem_go_out;
    assign par0_done_in = pd1_out & pd2_out;
    assign par0_go_in = ~par0_done_out & fsm1_out == 2'd1 & tdcc1_go_out;
    assign par_done_in = pd_out & pd0_out;
    assign par_go_in = ~par_done_out & fsm1_out == 2'd0 & tdcc1_go_out;
    assign pd_clk = clk;
    assign pd_in =
     pd_out & pd0_out ? 1'd0 :
     restore_2_done_out & par_go_out ? 1'd1 : 1'd0;
    assign pd_reset = reset;
    assign pd_write_en = pd_out & pd0_out | restore_2_done_out & par_go_out;
    assign pd0_clk = clk;
    assign pd0_in =
     pd_out & pd0_out ? 1'd0 :
     restore_1_done_out & par_go_out ? 1'd1 : 1'd0;
    assign pd0_reset = reset;
    assign pd0_write_en = pd_out & pd0_out | restore_1_done_out & par_go_out;
    assign pd1_clk = clk;
    assign pd1_in =
     pd1_out & pd2_out ? 1'd0 :
     tdcc_done_out & par0_go_out ? 1'd1 : 1'd0;
    assign pd1_reset = reset;
    assign pd1_write_en = pd1_out & pd2_out | tdcc_done_out & par0_go_out;
    assign pd2_clk = clk;
    assign pd2_in =
     pd1_out & pd2_out ? 1'd0 :
     tdcc0_done_out & par0_go_out ? 1'd1 : 1'd0;
    assign pd2_reset = reset;
    assign pd2_write_en = pd1_out & pd2_out | tdcc0_done_out & par0_go_out;
    assign reg_to_mem_done_in = out_done;
    assign reg_to_mem_go_in = ~reg_to_mem_done_out & fsm_out == 4'd7 & tdcc_go_out;
    assign restore_1_done_in = barrier_1_write_done_0;
    assign restore_1_go_in = ~(pd0_out | restore_1_done_out) & par_go_out | ~restore_1_done_out & fsm_out == 4'd6 & tdcc_go_out;
    assign restore_2_done_in = barrier_2_write_done_0;
    assign restore_2_go_in = ~(pd_out | restore_2_done_out) & par_go_out | ~restore_2_done_out & fsm_out == 4'd13 & tdcc_go_out;
    assign save_1_0_clk = clk;
    assign save_1_0_in =
     wait_1_go_out | wait_restore_1_go_out ? 32'd0 :
     barrier_1_read_done_0 & incr_barrier_1_0_go_out ? incr_1_0_out : 32'd0;
    assign save_1_0_reset = reset;
    assign save_1_0_write_en =
     wait_1_go_out | wait_restore_1_go_out ? 1'd1 :
     incr_barrier_1_0_go_out ? barrier_1_read_done_0 : 1'd0;
    assign save_1_1_clk = clk;
    assign save_1_1_in =
     barrier_1_read_done_1 & incr_barrier_1_1_go_out ? add_0_out : 32'd0;
    assign save_1_1_reset = reset;
    assign save_1_1_write_en =
     incr_barrier_1_1_go_out ? barrier_1_read_done_1 : 1'd0;
    assign save_2_0_clk = clk;
    assign save_2_0_in =
     wait_2_go_out | wait_restore_2_go_out ? 32'd0 :
     barrier_2_read_done_0 & incr_barrier_2_0_go_out ? incr_1_0_out : 32'd0;
    assign save_2_0_reset = reset;
    assign save_2_0_write_en =
     wait_2_go_out | wait_restore_2_go_out ? 1'd1 :
     incr_barrier_2_0_go_out ? barrier_2_read_done_0 : 1'd0;
    assign save_2_1_clk = clk;
    assign save_2_1_in =
     barrier_2_read_done_1 & incr_barrier_2_1_go_out ? add_0_out : 32'd0;
    assign save_2_1_reset = reset;
    assign save_2_1_write_en =
     incr_barrier_2_1_go_out ? barrier_2_read_done_1 : 1'd0;
    assign tdcc0_done_in = fsm0_out == 4'd12;
    assign tdcc0_go_in = ~(pd2_out | tdcc0_done_out) & par0_go_out;
    assign tdcc1_done_in = fsm1_out == 2'd2;
    assign tdcc1_go_in = go;
    assign tdcc_done_in = fsm_out == 4'd15;
    assign tdcc_go_in = ~(pd1_out | tdcc_done_out) & par0_go_out;
    assign val_clk = clk;
    assign val_in =
     incr_val_go_out ? add_0_out : 32'd0;
    assign val_reset = reset;
    assign val_write_en = incr_val_go_out;
    assign wait_1_done_in = eq_1_out;
    assign wait_1_go_in = ~wait_1_done_out & fsm_out == 4'd4 & tdcc_go_out | ~wait_1_done_out & fsm0_out == 4'd4 & tdcc0_go_out;
    assign wait_2_done_in = eq_2_out;
    assign wait_2_go_in = ~wait_2_done_out & fsm_out == 4'd11 & tdcc_go_out | ~wait_2_done_out & fsm0_out == 4'd9 & tdcc0_go_out;
    assign wait_restore_1_done_in = ~eq_1_out;
    assign wait_restore_1_go_in = ~wait_restore_1_done_out & fsm0_out == 4'd5 & tdcc0_go_out;
    assign wait_restore_2_done_in = ~eq_2_out;
    assign wait_restore_2_go_in = ~wait_restore_2_done_out & fsm0_out == 4'd10 & tdcc0_go_out;
    assign write_barrier_1_0_done_in = barrier_1_write_done_0;
    assign write_barrier_1_0_go_in = ~write_barrier_1_0_done_out & fsm_out == 4'd3 & tdcc_go_out;
    assign write_barrier_1_1_done_in = barrier_1_write_done_1;
    assign write_barrier_1_1_go_in = ~write_barrier_1_1_done_out & fsm0_out == 4'd3 & tdcc0_go_out;
    assign write_barrier_2_0_done_in = barrier_2_write_done_0;
    assign write_barrier_2_0_go_in = ~write_barrier_2_0_done_out & fsm_out == 4'd10 & tdcc_go_out;
    assign write_barrier_2_1_done_in = barrier_2_write_done_1;
    assign write_barrier_2_1_go_in = ~write_barrier_2_1_done_out & fsm0_out == 4'd8 & tdcc0_go_out;
    always_comb begin
        if(~$onehot0({incr_val_go_out, barrier_2_read_done_1 & incr_barrier_2_1_go_out, barrier_1_read_done_1 & incr_barrier_1_1_go_out})) begin
            $fatal(2, "Multiple assignment to port `add_0.left'.");
        end
        if(~$onehot0({write_barrier_1_0_go_out, restore_1_go_out})) begin
            $fatal(2, "Multiple assignment to port `barrier_1.in_0'.");
        end
        if(~$onehot0({write_barrier_2_0_go_out, restore_2_go_out})) begin
            $fatal(2, "Multiple assignment to port `barrier_2.in_0'.");
        end
        if(~$onehot0({fsm_out == 4'd8 & incr_idx_done_out & tdcc_go_out, fsm_out == 4'd7 & reg_to_mem_done_out & tdcc_go_out, fsm_out == 4'd6 & restore_1_done_out & tdcc_go_out, fsm_out == 4'd5 & clear_barrier_1_done_out & tdcc_go_out, fsm_out == 4'd4 & wait_1_done_out & tdcc_go_out, fsm_out == 4'd3 & write_barrier_1_0_done_out & tdcc_go_out, fsm_out == 4'd2 & incr_barrier_1_0_done_out & tdcc_go_out, fsm_out == 4'd1 & no_op_done_out & tdcc_go_out, fsm_out == 4'd0 & cmp0_done_out & comb_reg_out & tdcc_go_out | fsm_out == 4'd14 & cmp0_done_out & comb_reg_out & tdcc_go_out, fsm_out == 4'd0 & cmp0_done_out & ~comb_reg_out & tdcc_go_out | fsm_out == 4'd14 & cmp0_done_out & ~comb_reg_out & tdcc_go_out, fsm_out == 4'd13 & restore_2_done_out & tdcc_go_out, fsm_out == 4'd12 & clear_barrier_2_done_out & tdcc_go_out, fsm_out == 4'd11 & wait_2_done_out & tdcc_go_out, fsm_out == 4'd10 & write_barrier_2_0_done_out & tdcc_go_out, fsm_out == 4'd9 & incr_barrier_2_0_done_out & tdcc_go_out, fsm_out == 4'd15})) begin
            $fatal(2, "Multiple assignment to port `fsm.in'.");
        end
        if(~$onehot0({fsm0_out == 4'd8 & write_barrier_2_1_done_out & tdcc0_go_out, fsm0_out == 4'd7 & incr_barrier_2_1_done_out & tdcc0_go_out, fsm0_out == 4'd6 & no_op_done_out & tdcc0_go_out, fsm0_out == 4'd5 & wait_restore_1_done_out & tdcc0_go_out, fsm0_out == 4'd4 & wait_1_done_out & tdcc0_go_out, fsm0_out == 4'd3 & write_barrier_1_1_done_out & tdcc0_go_out, fsm0_out == 4'd2 & incr_barrier_1_1_done_out & tdcc0_go_out, fsm0_out == 4'd1 & incr_val_done_out & tdcc0_go_out, fsm0_out == 4'd0 & cmp0_done_out & comb_reg_out & tdcc0_go_out | fsm0_out == 4'd11 & cmp0_done_out & comb_reg_out & tdcc0_go_out, fsm0_out == 4'd0 & cmp0_done_out & ~comb_reg_out & tdcc0_go_out | fsm0_out == 4'd11 & cmp0_done_out & ~comb_reg_out & tdcc0_go_out, fsm0_out == 4'd10 & wait_restore_2_done_out & tdcc0_go_out, fsm0_out == 4'd9 & wait_2_done_out & tdcc0_go_out, fsm0_out == 4'd12})) begin
            $fatal(2, "Multiple assignment to port `fsm0.in'.");
        end
        if(~$onehot0({fsm1_out == 2'd1 & par0_done_out & tdcc1_go_out, fsm1_out == 2'd0 & par_done_out & tdcc1_go_out, fsm1_out == 2'd2})) begin
            $fatal(2, "Multiple assignment to port `fsm1.in'.");
        end
        if(~$onehot0({barrier_2_read_done_0 & incr_barrier_2_0_go_out, barrier_1_read_done_0 & incr_barrier_1_0_go_out})) begin
            $fatal(2, "Multiple assignment to port `incr_1_0.left'.");
        end
        if(~$onehot0({restore_2_done_out & par_go_out, pd_out & pd0_out})) begin
            $fatal(2, "Multiple assignment to port `pd.in'.");
        end
        if(~$onehot0({restore_1_done_out & par_go_out, pd_out & pd0_out})) begin
            $fatal(2, "Multiple assignment to port `pd0.in'.");
        end
        if(~$onehot0({tdcc_done_out & par0_go_out, pd1_out & pd2_out})) begin
            $fatal(2, "Multiple assignment to port `pd1.in'.");
        end
        if(~$onehot0({tdcc0_done_out & par0_go_out, pd1_out & pd2_out})) begin
            $fatal(2, "Multiple assignment to port `pd2.in'.");
        end
        if(~$onehot0({barrier_1_read_done_0 & incr_barrier_1_0_go_out, wait_1_go_out | wait_restore_1_go_out})) begin
            $fatal(2, "Multiple assignment to port `save_1_0.in'.");
        end
        if(~$onehot0({incr_barrier_1_0_go_out, wait_1_go_out | wait_restore_1_go_out})) begin
            $fatal(2, "Multiple assignment to port `save_1_0.write_en'.");
        end
        if(~$onehot0({barrier_2_read_done_0 & incr_barrier_2_0_go_out, wait_2_go_out | wait_restore_2_go_out})) begin
            $fatal(2, "Multiple assignment to port `save_2_0.in'.");
        end
        if(~$onehot0({incr_barrier_2_0_go_out, wait_2_go_out | wait_restore_2_go_out})) begin
            $fatal(2, "Multiple assignment to port `save_2_0.write_en'.");
        end
    end
endmodule
