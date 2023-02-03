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

module main(
  input logic go,
  input logic clk,
  input logic reset,
  output logic done,
  output logic [3:0] A0_addr0,
  output logic [31:0] A0_write_data,
  output logic A0_write_en,
  output logic A0_clk,
  input logic [31:0] A0_read_data,
  input logic A0_done,
  output logic [3:0] B0_addr0,
  output logic [31:0] B0_write_data,
  output logic B0_write_en,
  output logic B0_clk,
  input logic [31:0] B0_read_data,
  input logic B0_done,
  output logic [3:0] Sum0_addr0,
  output logic [31:0] Sum0_write_data,
  output logic Sum0_write_en,
  output logic Sum0_clk,
  input logic [31:0] Sum0_read_data,
  input logic Sum0_done
);
// COMPONENT START: main
logic [31:0] A_read0_0_in;
logic A_read0_0_write_en;
logic A_read0_0_clk;
logic A_read0_0_reset;
logic [31:0] A_read0_0_out;
logic A_read0_0_done;
logic [31:0] B_read0_0_in;
logic B_read0_0_write_en;
logic B_read0_0_clk;
logic B_read0_0_reset;
logic [31:0] B_read0_0_out;
logic B_read0_0_done;
logic [31:0] add0_left;
logic [31:0] add0_right;
logic [31:0] add0_out;
logic [3:0] add1_left;
logic [3:0] add1_right;
logic [3:0] add1_out;
logic [3:0] const0_out;
logic [3:0] const1_out;
logic [3:0] const2_out;
logic [3:0] i0_in;
logic i0_write_en;
logic i0_clk;
logic i0_reset;
logic [3:0] i0_out;
logic i0_done;
logic [3:0] le0_left;
logic [3:0] le0_right;
logic le0_out;
logic comb_reg_in;
logic comb_reg_write_en;
logic comb_reg_clk;
logic comb_reg_reset;
logic comb_reg_out;
logic comb_reg_done;
logic [3:0] comb_reg0_in;
logic comb_reg0_write_en;
logic comb_reg0_clk;
logic comb_reg0_reset;
logic [3:0] comb_reg0_out;
logic comb_reg0_done;
logic [2:0] fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic [2:0] fsm_out;
logic fsm_done;
logic upd2_go_in;
logic upd2_go_out;
logic upd2_done_in;
logic upd2_done_out;
logic cond00_go_in;
logic cond00_go_out;
logic cond00_done_in;
logic cond00_done_out;
logic comb_invoke0_go_in;
logic comb_invoke0_go_out;
logic comb_invoke0_done_in;
logic comb_invoke0_done_out;
logic invoke0_go_in;
logic invoke0_go_out;
logic invoke0_done_in;
logic invoke0_done_out;
logic invoke1_go_in;
logic invoke1_go_out;
logic invoke1_done_in;
logic invoke1_done_out;
logic msp_go_in;
logic msp_go_out;
logic msp_done_in;
logic msp_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
std_reg # (
    .WIDTH(32)
) A_read0_0 (
    .clk(A_read0_0_clk),
    .done(A_read0_0_done),
    .in(A_read0_0_in),
    .out(A_read0_0_out),
    .reset(A_read0_0_reset),
    .write_en(A_read0_0_write_en)
);
std_reg # (
    .WIDTH(32)
) B_read0_0 (
    .clk(B_read0_0_clk),
    .done(B_read0_0_done),
    .in(B_read0_0_in),
    .out(B_read0_0_out),
    .reset(B_read0_0_reset),
    .write_en(B_read0_0_write_en)
);
std_add # (
    .WIDTH(32)
) add0 (
    .left(add0_left),
    .out(add0_out),
    .right(add0_right)
);
std_add # (
    .WIDTH(4)
) add1 (
    .left(add1_left),
    .out(add1_out),
    .right(add1_right)
);
std_const # (
    .VALUE(4'd0),
    .WIDTH(4)
) const0 (
    .out(const0_out)
);
std_const # (
    .VALUE(4'd7),
    .WIDTH(4)
) const1 (
    .out(const1_out)
);
std_const # (
    .VALUE(4'd1),
    .WIDTH(4)
) const2 (
    .out(const2_out)
);
std_reg # (
    .WIDTH(4)
) i0 (
    .clk(i0_clk),
    .done(i0_done),
    .in(i0_in),
    .out(i0_out),
    .reset(i0_reset),
    .write_en(i0_write_en)
);
std_le # (
    .WIDTH(4)
) le0 (
    .left(le0_left),
    .out(le0_out),
    .right(le0_right)
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
    .WIDTH(4)
) comb_reg0 (
    .clk(comb_reg0_clk),
    .done(comb_reg0_done),
    .in(comb_reg0_in),
    .out(comb_reg0_out),
    .reset(comb_reg0_reset),
    .write_en(comb_reg0_write_en)
);
std_reg # (
    .WIDTH(3)
) fsm (
    .clk(fsm_clk),
    .done(fsm_done),
    .in(fsm_in),
    .out(fsm_out),
    .reset(fsm_reset),
    .write_en(fsm_write_en)
);
std_wire # (
    .WIDTH(1)
) upd2_go (
    .in(upd2_go_in),
    .out(upd2_go_out)
);
std_wire # (
    .WIDTH(1)
) upd2_done (
    .in(upd2_done_in),
    .out(upd2_done_out)
);
std_wire # (
    .WIDTH(1)
) cond00_go (
    .in(cond00_go_in),
    .out(cond00_go_out)
);
std_wire # (
    .WIDTH(1)
) cond00_done (
    .in(cond00_done_in),
    .out(cond00_done_out)
);
std_wire # (
    .WIDTH(1)
) comb_invoke0_go (
    .in(comb_invoke0_go_in),
    .out(comb_invoke0_go_out)
);
std_wire # (
    .WIDTH(1)
) comb_invoke0_done (
    .in(comb_invoke0_done_in),
    .out(comb_invoke0_done_out)
);
std_wire # (
    .WIDTH(1)
) invoke0_go (
    .in(invoke0_go_in),
    .out(invoke0_go_out)
);
std_wire # (
    .WIDTH(1)
) invoke0_done (
    .in(invoke0_done_in),
    .out(invoke0_done_out)
);
std_wire # (
    .WIDTH(1)
) invoke1_go (
    .in(invoke1_go_in),
    .out(invoke1_go_out)
);
std_wire # (
    .WIDTH(1)
) invoke1_done (
    .in(invoke1_done_in),
    .out(invoke1_done_out)
);
std_wire # (
    .WIDTH(1)
) msp_go (
    .in(msp_go_in),
    .out(msp_go_out)
);
std_wire # (
    .WIDTH(1)
) msp_done (
    .in(msp_done_in),
    .out(msp_done_out)
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
assign A_read0_0_in =
 msp_go_out ? A0_read_data : 32'd0;
assign A_read0_0_write_en = msp_go_out;
assign A_read0_0_clk = clk;
assign A_read0_0_reset = reset;
assign B_read0_0_in =
 msp_go_out ? B0_read_data : 32'd0;
assign B_read0_0_write_en = msp_go_out;
assign B_read0_0_clk = clk;
assign B_read0_0_reset = reset;
assign add0_left =
 upd2_go_out ? A_read0_0_out : 32'd0;
assign add0_right =
 upd2_go_out ? B_read0_0_out : 32'd0;
assign add1_left =
 comb_invoke0_go_out ? i0_out : 4'd0;
assign add1_right =
 comb_invoke0_go_out ? const2_out : 4'd0;
assign i0_in =
 invoke0_go_out ? const0_out :
 invoke1_go_out ? comb_reg0_out : 4'd0;
assign i0_write_en = invoke0_go_out | invoke1_go_out;
assign i0_clk = clk;
assign i0_reset = reset;
assign le0_left =
 cond00_go_out ? i0_out : 4'd0;
assign le0_right =
 cond00_go_out ? const1_out : 4'd0;
assign done = tdcc_done_out;
assign A0_addr0 =
 msp_go_out ? i0_out : 4'd0;
assign A0_clk = clk;
assign B0_addr0 =
 msp_go_out ? i0_out : 4'd0;
assign B0_clk = clk;
assign Sum0_addr0 =
 upd2_go_out ? i0_out : 4'd0;
assign Sum0_write_data =
 upd2_go_out ? add0_out : 32'd0;
assign Sum0_write_en = upd2_go_out;
assign Sum0_clk = clk;
assign comb_reg_in =
 cond00_go_out ? le0_out : 1'd0;
assign comb_reg_write_en = cond00_go_out;
assign comb_reg_clk = clk;
assign comb_reg_reset = reset;
assign comb_reg0_in =
 comb_invoke0_go_out ? add1_out : 4'd0;
assign comb_reg0_write_en = comb_invoke0_go_out;
assign comb_reg0_clk = clk;
assign comb_reg0_reset = reset;
assign fsm_in =
 fsm_out == 3'd1 & cond00_done_out & ~comb_reg_out & tdcc_go_out | fsm_out == 3'd6 & cond00_done_out & ~comb_reg_out & tdcc_go_out ? 3'd7 :
 fsm_out == 3'd7 ? 3'd0 :
 fsm_out == 3'd0 & invoke0_done_out & tdcc_go_out ? 3'd1 :
 fsm_out == 3'd1 & cond00_done_out & comb_reg_out & tdcc_go_out | fsm_out == 3'd6 & cond00_done_out & comb_reg_out & tdcc_go_out ? 3'd2 :
 fsm_out == 3'd2 & msp_done_out & tdcc_go_out ? 3'd3 :
 fsm_out == 3'd3 & upd2_done_out & tdcc_go_out ? 3'd4 :
 fsm_out == 3'd4 & comb_invoke0_done_out & tdcc_go_out ? 3'd5 :
 fsm_out == 3'd5 & invoke1_done_out & tdcc_go_out ? 3'd6 : 3'd0;
assign fsm_write_en = fsm_out == 3'd7 | fsm_out == 3'd0 & invoke0_done_out & tdcc_go_out | fsm_out == 3'd1 & cond00_done_out & comb_reg_out & tdcc_go_out | fsm_out == 3'd6 & cond00_done_out & comb_reg_out & tdcc_go_out | fsm_out == 3'd2 & msp_done_out & tdcc_go_out | fsm_out == 3'd3 & upd2_done_out & tdcc_go_out | fsm_out == 3'd4 & comb_invoke0_done_out & tdcc_go_out | fsm_out == 3'd5 & invoke1_done_out & tdcc_go_out | fsm_out == 3'd1 & cond00_done_out & ~comb_reg_out & tdcc_go_out | fsm_out == 3'd6 & cond00_done_out & ~comb_reg_out & tdcc_go_out;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign upd2_go_in = ~upd2_done_out & fsm_out == 3'd3 & tdcc_go_out;
assign upd2_done_in = Sum0_done;
assign cond00_go_in = ~cond00_done_out & fsm_out == 3'd1 & tdcc_go_out | ~cond00_done_out & fsm_out == 3'd6 & tdcc_go_out;
assign cond00_done_in = comb_reg_done;
assign comb_invoke0_go_in = ~comb_invoke0_done_out & fsm_out == 3'd4 & tdcc_go_out;
assign comb_invoke0_done_in = comb_reg0_done;
assign invoke0_go_in = ~invoke0_done_out & fsm_out == 3'd0 & tdcc_go_out;
assign invoke0_done_in = i0_done;
assign invoke1_go_in = ~invoke1_done_out & fsm_out == 3'd5 & tdcc_go_out;
assign invoke1_done_in = i0_done;
assign msp_go_in = ~msp_done_out & fsm_out == 3'd2 & tdcc_go_out;
assign msp_done_in = A_read0_0_done & B_read0_0_done;
assign tdcc_go_in = go;
assign tdcc_done_in = fsm_out == 3'd7;
// COMPONENT END: main
endmodule
