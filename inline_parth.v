module comb_mem_d1 #(
    parameter WIDTH = 32,
    parameter SIZE = 16,
    parameter IDX_SIZE = 4
) (
   input wire                logic [IDX_SIZE-1:0] addr0,
   input wire                logic [ WIDTH-1:0] write_data,
   input wire                logic write_en,
   input wire                logic clk,
   input wire                logic reset,
   output logic [ WIDTH-1:0] read_data,
   output logic              done
);

  logic [WIDTH-1:0] mem[SIZE-1:0];

  /* verilator lint_off WIDTH */
  assign read_data = mem[addr0];

  always_ff @(posedge clk) begin
    if (reset)
      done <= '0;
    else if (write_en)
      done <= '1;
    else
      done <= '0;
  end

  always_ff @(posedge clk) begin
    if (!reset && write_en)
      mem[addr0] <= write_data;
  end

  // Check for out of bounds access
  `ifdef VERILATOR
    always_comb begin
      if (addr0 >= SIZE)
        $error(
          "comb_mem_d1: Out of bounds access\n",
          "addr0: %0d\n", addr0,
          "SIZE: %0d", SIZE
        );
    end
  `endif
endmodule

module comb_mem_d2 #(
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
   input wire                logic reset,
   output logic [ WIDTH-1:0] read_data,
   output logic              done
);

  /* verilator lint_off WIDTH */
  logic [WIDTH-1:0] mem[D0_SIZE-1:0][D1_SIZE-1:0];

  assign read_data = mem[addr0][addr1];

  always_ff @(posedge clk) begin
    if (reset)
      done <= '0;
    else if (write_en)
      done <= '1;
    else
      done <= '0;
  end

  always_ff @(posedge clk) begin
    if (!reset && write_en)
      mem[addr0][addr1] <= write_data;
  end

  // Check for out of bounds access
  `ifdef VERILATOR
    always_comb begin
      if (addr0 >= D0_SIZE)
        $error(
          "comb_mem_d2: Out of bounds access\n",
          "addr0: %0d\n", addr0,
          "D0_SIZE: %0d", D0_SIZE
        );
      if (addr1 >= D1_SIZE)
        $error(
          "comb_mem_d2: Out of bounds access\n",
          "addr1: %0d\n", addr1,
          "D1_SIZE: %0d", D1_SIZE
        );
    end
  `endif
endmodule

module comb_mem_d3 #(
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
   input wire                logic reset,
   output logic [ WIDTH-1:0] read_data,
   output logic              done
);

  /* verilator lint_off WIDTH */
  logic [WIDTH-1:0] mem[D0_SIZE-1:0][D1_SIZE-1:0][D2_SIZE-1:0];

  assign read_data = mem[addr0][addr1][addr2];

  always_ff @(posedge clk) begin
    if (reset)
      done <= '0;
    else if (write_en)
      done <= '1;
    else
      done <= '0;
  end

  always_ff @(posedge clk) begin
    if (!reset && write_en)
      mem[addr0][addr1][addr2] <= write_data;
  end

  // Check for out of bounds access
  `ifdef VERILATOR
    always_comb begin
      if (addr0 >= D0_SIZE)
        $error(
          "comb_mem_d3: Out of bounds access\n",
          "addr0: %0d\n", addr0,
          "D0_SIZE: %0d", D0_SIZE
        );
      if (addr1 >= D1_SIZE)
        $error(
          "comb_mem_d3: Out of bounds access\n",
          "addr1: %0d\n", addr1,
          "D1_SIZE: %0d", D1_SIZE
        );
      if (addr2 >= D2_SIZE)
        $error(
          "comb_mem_d3: Out of bounds access\n",
          "addr2: %0d\n", addr2,
          "D2_SIZE: %0d", D2_SIZE
        );
    end
  `endif
endmodule

module comb_mem_d4 #(
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
   input wire                logic reset,
   output logic [ WIDTH-1:0] read_data,
   output logic              done
);

  /* verilator lint_off WIDTH */
  logic [WIDTH-1:0] mem[D0_SIZE-1:0][D1_SIZE-1:0][D2_SIZE-1:0][D3_SIZE-1:0];

  assign read_data = mem[addr0][addr1][addr2][addr3];

  always_ff @(posedge clk) begin
    if (reset)
      done <= '0;
    else if (write_en)
      done <= '1;
    else
      done <= '0;
  end

  always_ff @(posedge clk) begin
    if (!reset && write_en)
      mem[addr0][addr1][addr2][addr3] <= write_data;
  end

  // Check for out of bounds access
  `ifdef VERILATOR
    always_comb begin
      if (addr0 >= D0_SIZE)
        $error(
          "comb_mem_d4: Out of bounds access\n",
          "addr0: %0d\n", addr0,
          "D0_SIZE: %0d", D0_SIZE
        );
      if (addr1 >= D1_SIZE)
        $error(
          "comb_mem_d4: Out of bounds access\n",
          "addr1: %0d\n", addr1,
          "D1_SIZE: %0d", D1_SIZE
        );
      if (addr2 >= D2_SIZE)
        $error(
          "comb_mem_d4: Out of bounds access\n",
          "addr2: %0d\n", addr2,
          "D2_SIZE: %0d", D2_SIZE
        );
      if (addr3 >= D3_SIZE)
        $error(
          "comb_mem_d4: Out of bounds access\n",
          "addr3: %0d\n", addr3,
          "D3_SIZE: %0d", D3_SIZE
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

module std_cat #(
  parameter LEFT_WIDTH  = 32,
  parameter RIGHT_WIDTH = 32,
  parameter OUT_WIDTH = 64
) (
  input wire logic [LEFT_WIDTH-1:0] left,
  input wire logic [RIGHT_WIDTH-1:0] right,
  output logic [OUT_WIDTH-1:0] out
);
  assign out = {left, right};

  `ifdef VERILATOR
    always_comb begin
      if (LEFT_WIDTH + RIGHT_WIDTH != OUT_WIDTH)
        $error(
          "std_cat: Output width must equal sum of input widths\n",
          "LEFT_WIDTH: %0d", LEFT_WIDTH,
          "RIGHT_WIDTH: %0d", RIGHT_WIDTH,
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

module std_bit_slice #(
    parameter IN_WIDTH = 32,
    parameter START_IDX = 0,
    parameter END_IDX = 31,
    parameter OUT_WIDTH = 32
)(
   input wire logic [IN_WIDTH-1:0] in,
   output logic [OUT_WIDTH-1:0] out
);
  assign out = in[END_IDX:START_IDX];

  `ifdef VERILATOR
    always_comb begin
      if (START_IDX < 0 || END_IDX > IN_WIDTH-1)
        $error(
          "std_bit_slice: Slice range out of bounds\n",
          "IN_WIDTH: %0d", IN_WIDTH,
          "START_IDX: %0d", START_IDX,
          "END_IDX: %0d", END_IDX,
        );
    end
  `endif

endmodule

module std_skid_buffer #(
    parameter WIDTH = 32
)(
    input wire logic [WIDTH-1:0] in,
    input wire logic i_valid,
    input wire logic i_ready,
    input wire logic clk,
    input wire logic reset,
    output logic [WIDTH-1:0] out,
    output logic o_valid,
    output logic o_ready
);
  logic [WIDTH-1:0] val;
  logic bypass_rg;
  always @(posedge clk) begin
    // Reset  
    if (reset) begin      
      // Internal Registers
      val <= '0;     
      bypass_rg <= 1'b1;
    end   
    // Out of reset
    else begin      
      // Bypass state      
      if (bypass_rg) begin         
        if (!i_ready && i_valid) begin
          val <= in;          // Data skid happened, store to buffer
          bypass_rg <= 1'b0;  // To skid mode  
        end 
      end 
      // Skid state
      else begin         
        if (i_ready) begin
          bypass_rg <= 1'b1;  // Back to bypass mode           
        end
      end
    end
  end

  assign o_ready = bypass_rg;
  assign out = bypass_rg ? in : val;
  assign o_valid = bypass_rg ? i_valid : 1'b1;
endmodule

module std_bypass_reg #(
    parameter WIDTH = 32
)(
    input wire logic [WIDTH-1:0] in,
    input wire logic write_en,
    input wire logic clk,
    input wire logic reset,
    output logic [WIDTH-1:0] out,
    output logic done
);
  logic [WIDTH-1:0] val;
  assign out = write_en ? in : val;

  always_ff @(posedge clk) begin
    if (reset) begin
      val <= 0;
      done <= 0;
    end else if (write_en) begin
      val <= in;
      done <= 1'd1;
    end else done <= 1'd0;
  end
endmodule

module undef #(
    parameter WIDTH = 32
) (
   output logic [WIDTH-1:0] out
);
assign out = 'x;
endmodule

module std_const #(
    parameter WIDTH = 32,
    parameter VALUE = 32
) (
   output logic [WIDTH-1:0] out
);
assign out = VALUE;
endmodule

module std_wire #(
    parameter WIDTH = 32
) (
   input wire logic [WIDTH-1:0] in,
   output logic [WIDTH-1:0] out
);
assign out = in;
endmodule

module std_add #(
    parameter WIDTH = 32
) (
   input wire logic [WIDTH-1:0] left,
   input wire logic [WIDTH-1:0] right,
   output logic [WIDTH-1:0] out
);
assign out = left + right;
endmodule

module std_lsh #(
    parameter WIDTH = 32
) (
   input wire logic [WIDTH-1:0] left,
   input wire logic [WIDTH-1:0] right,
   output logic [WIDTH-1:0] out
);
assign out = left << right;
endmodule

module std_reg #(
    parameter WIDTH = 32
) (
   input wire logic [WIDTH-1:0] in,
   input wire logic write_en,
   input wire logic clk,
   input wire logic reset,
   output logic [WIDTH-1:0] out,
   output logic done
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

module init_one_reg #(
    parameter WIDTH = 32
) (
   input wire logic [WIDTH-1:0] in,
   input wire logic write_en,
   input wire logic clk,
   input wire logic reset,
   output logic [WIDTH-1:0] out,
   output logic done
);
always_ff @(posedge clk) begin
    if (reset) begin
       out <= 1;
       done <= 0;
    end else if (write_en) begin
      out <= in;
      done <= 1'd1;
    end else done <= 1'd0;
  end
endmodule


module fsm_main_def (
  input logic clk,
  input logic reset,

  input logic  fsm_start_out,
  output logic s0_out,
  output logic s1_out,
  output logic s2_out,
  output logic s3_out
);

  parameter s0 = 2'd0;
  parameter s1 = 2'd1;
  parameter s2 = 2'd2;
  parameter s3 = 2'd3;

  logic [1:0] state_reg;
  logic [1:0] state_next;

  always @(posedge clk) begin
if (reset) begin
      state_reg <= s0;
    end
else begin
      state_reg <= state_next;
end
  end

  always @(*) begin
    state_next = s0;
case ( state_reg )
        s0: begin
          s0_out = 1'b1;
          s1_out = 1'b0;
          s2_out = 1'b0;
          s3_out = 1'b0;
          if (fsm_start_out) begin
            state_next = s1;
          end
          else begin
            state_next = s0;
          end
        end
        s1: begin
          s0_out = 1'b0;
          s1_out = 1'b1;
          s2_out = 1'b0;
          s3_out = 1'b0;
          state_next = s2;
        end
        s2: begin
          s0_out = 1'b0;
          s1_out = 1'b0;
          s2_out = 1'b1;
          s3_out = 1'b0;
          state_next = s3;
        end
        s3: begin
          s0_out = 1'b0;
          s1_out = 1'b0;
          s2_out = 1'b0;
          s3_out = 1'b1;
          state_next = s0;
        end
    endcase
  end
endmodule


module fsm0_main_def (
  input logic clk,
  input logic reset,

  input logic [21:0] group_counter_out,
  input logic [21:0] const3999999_22__out,
  input logic  fsm0_start_out,
  output logic s0_out,
  output logic s1_out
);

  parameter s0 = 1'd0;
  parameter s1 = 1'd1;

  logic [0:0] state_reg;
  logic [0:0] state_next;

  always @(posedge clk) begin
if (reset) begin
      state_reg <= s0;
    end
else begin
      state_reg <= state_next;
end
  end

  always @(*) begin
    state_next = s0;
case ( state_reg )
        s0: begin
          s0_out = 1'b1;
          s1_out = 1'b0;
          if ((group_counter_out == const3999999_22__out) & (fsm0_start_out)) begin
            state_next = s1;
          end
          else begin
            state_next = s0;
          end
        end
        s1: begin
          s0_out = 1'b0;
          s1_out = 1'b1;
          state_next = s0;
        end
    endcase
  end
endmodule

module main(
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: main
logic [1:0] a_in;
logic a_write_en;
logic a_clk;
logic a_reset;
logic [1:0] a_out;
logic a_done;
logic [1:0] b_in;
logic b_write_en;
logic b_clk;
logic b_reset;
logic [1:0] b_out;
logic b_done;
logic fsm_0_in;
logic fsm_0_out;
logic fsm_1_in;
logic fsm_1_out;
logic fsm_2_in;
logic fsm_2_out;
logic fsm_3_in;
logic fsm_3_out;
logic [21:0] group_counter_in;
logic group_counter_write_en;
logic group_counter_clk;
logic group_counter_reset;
logic [21:0] group_counter_out;
logic group_counter_done;
logic [21:0] const3999999_22__in;
logic [21:0] const3999999_22__out;
logic [21:0] adder_left;
logic [21:0] adder_right;
logic [21:0] adder_out;
logic fsm0_0_in;
logic fsm0_0_out;
logic fsm0_1_in;
logic fsm0_1_out;
logic looped_once_in;
logic looped_once_write_en;
logic looped_once_clk;
logic looped_once_reset;
logic looped_once_out;
logic looped_once_done;
logic fsm_start_in;
logic fsm_start_out;
logic fsm_done_in;
logic fsm_done_out;
logic fsm0_start_in;
logic fsm0_start_out;
logic fsm0_done_in;
logic fsm0_done_out;
std_reg # (
    .WIDTH(2)
) a (
    .clk(a_clk),
    .done(a_done),
    .in(a_in),
    .out(a_out),
    .reset(a_reset),
    .write_en(a_write_en)
);
std_reg # (
    .WIDTH(2)
) b (
    .clk(b_clk),
    .done(b_done),
    .in(b_in),
    .out(b_out),
    .reset(b_reset),
    .write_en(b_write_en)
);
std_wire # (
    .WIDTH(1)
) fsm_0 (
    .in(fsm_0_in),
    .out(fsm_0_out)
);
std_wire # (
    .WIDTH(1)
) fsm_1 (
    .in(fsm_1_in),
    .out(fsm_1_out)
);
std_wire # (
    .WIDTH(1)
) fsm_2 (
    .in(fsm_2_in),
    .out(fsm_2_out)
);
std_wire # (
    .WIDTH(1)
) fsm_3 (
    .in(fsm_3_in),
    .out(fsm_3_out)
);
std_reg # (
    .WIDTH(22)
) group_counter (
    .clk(group_counter_clk),
    .done(group_counter_done),
    .in(group_counter_in),
    .out(group_counter_out),
    .reset(group_counter_reset),
    .write_en(group_counter_write_en)
);
std_wire # (
    .WIDTH(22)
) const3999999_22_ (
    .in(const3999999_22__in),
    .out(const3999999_22__out)
);
std_add # (
    .WIDTH(22)
) adder (
    .left(adder_left),
    .out(adder_out),
    .right(adder_right)
);
std_wire # (
    .WIDTH(1)
) fsm0_0 (
    .in(fsm0_0_in),
    .out(fsm0_0_out)
);
std_wire # (
    .WIDTH(1)
) fsm0_1 (
    .in(fsm0_1_in),
    .out(fsm0_1_out)
);
std_reg # (
    .WIDTH(1)
) looped_once (
    .clk(looped_once_clk),
    .done(looped_once_done),
    .in(looped_once_in),
    .out(looped_once_out),
    .reset(looped_once_reset),
    .write_en(looped_once_write_en)
);
std_wire # (
    .WIDTH(1)
) fsm_start (
    .in(fsm_start_in),
    .out(fsm_start_out)
);
std_wire # (
    .WIDTH(1)
) fsm_done (
    .in(fsm_done_in),
    .out(fsm_done_out)
);
std_wire # (
    .WIDTH(1)
) fsm0_start (
    .in(fsm0_start_in),
    .out(fsm0_start_out)
);
std_wire # (
    .WIDTH(1)
) fsm0_done (
    .in(fsm0_done_in),
    .out(fsm0_done_out)
);
logic fsm_s0_out;
logic fsm_s1_out;
logic fsm_s2_out;
logic fsm_s3_out;
fsm_main_def fsm (
  .s0_out(fsm_s0_out),
  .s1_out(fsm_s1_out),
  .s2_out(fsm_s2_out),
  .s3_out(fsm_s3_out),
  .*
);
logic fsm0_s0_out;
logic fsm0_s1_out;
fsm0_main_def fsm0 (
  .s0_out(fsm0_s0_out),
  .s1_out(fsm0_s1_out),
  .*
);
assign fsm_3_in =
 fsm_s3_out ? 1'd1 :
 1'd0;
assign looped_once_in =
 (fsm0_s0_out & ((fsm0_start_out) & (group_counter_out == const3999999_22__out))) ? 1'd1 :
 1'd0;
assign fsm_1_in =
 fsm_s1_out ? 1'd1 :
 1'd0;
assign fsm0_done_in =
 fsm0_s0_out ? looped_once_out :
 1'd0;
assign fsm_2_in =
 fsm_s2_out ? 1'd1 :
 1'd0;
assign looped_once_write_en =
 fsm0_s0_out ? 1'd1 :
 1'd0;
assign fsm0_1_in =
 fsm0_s1_out ? 1'd1 :
 1'd0;
assign fsm0_0_in =
 (fsm0_s0_out & (fsm0_start_out)) ? 1'd1 :
 1'd0;
assign fsm_0_in =
 (fsm_s0_out & (fsm_start_out)) ? 1'd1 :
 1'd0;
wire _guard0 = 1;
wire _guard1 = fsm0_done_out;
wire _guard2 = fsm0_0_out;
wire _guard3 = fsm0_0_out;
wire _guard4 = fsm_2_out;
wire _guard5 = fsm_0_out;
wire _guard6 = _guard4 | _guard5;
wire _guard7 = fsm_1_out;
wire _guard8 = _guard6 | _guard7;
wire _guard9 = fsm_3_out;
wire _guard10 = _guard8 | _guard9;
wire _guard11 = fsm0_1_out;
wire _guard12 = _guard10 | _guard11;
wire _guard13 = fsm_1_out;
wire _guard14 = fsm_3_out;
wire _guard15 = _guard13 | _guard14;
wire _guard16 = fsm0_1_out;
wire _guard17 = _guard15 | _guard16;
wire _guard18 = fsm_2_out;
wire _guard19 = fsm_0_out;
wire _guard20 = _guard18 | _guard19;
wire _guard21 = fsm0_0_out;
wire _guard22 = group_counter_out != const3999999_22__out;
wire _guard23 = fsm0_0_out;
wire _guard24 = _guard22 & _guard23;
wire _guard25 = group_counter_out == const3999999_22__out;
wire _guard26 = fsm0_0_out;
wire _guard27 = _guard25 & _guard26;
wire _guard28 = fsm0_0_out;
wire _guard29 = fsm0_0_out;
assign done = _guard1;
assign adder_left =
  _guard2 ? group_counter_out :
  22'd0;
assign adder_right =
  _guard3 ? 22'd1 :
  22'd0;
assign looped_once_clk = clk;
assign looped_once_reset = reset;
assign fsm0_start_in = go;
assign b_write_en = 1'd0;
assign b_clk = clk;
assign b_reset = reset;
assign a_write_en = _guard12;
assign a_clk = clk;
assign a_reset = reset;
assign a_in =
  _guard17 ? 2'd1 :
  _guard20 ? 2'd0 :
  'x;
always_ff @(posedge clk) begin
  if(~$onehot0({_guard20, _guard17})) begin
    $fatal(2, "Multiple assignment to port `a.in'.");
end
end
assign group_counter_write_en = _guard21;
assign group_counter_clk = clk;
assign group_counter_reset = reset;
assign group_counter_in =
  _guard24 ? adder_out :
  _guard27 ? 22'd0 :
  22'd0;
always_ff @(posedge clk) begin
  if(~$onehot0({_guard27, _guard24})) begin
    $fatal(2, "Multiple assignment to port `group_counter.in'.");
end
end
assign const3999999_22__in =
  _guard28 ? 22'd3999999 :
  22'd0;
assign fsm_start_in = _guard29;
// COMPONENT END: main
endmodule
