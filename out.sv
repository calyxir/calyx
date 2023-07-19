/**
 * Core primitives for Calyx.
 * Implements core primitives used by the compiler.
 *
 * Conventions:
 * - All parameter names must be SNAKE_CASE and all caps.
 * - Port names must be snake_case, no caps.
 */
`default_nettype none

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

module std_mem_d1 #(
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
   input logic [WIDTH-1:0] in,
   output logic [WIDTH-1:0] out
);

    assign out = in;
endmodule

module std_add #(
    parameter WIDTH = 32
) (
   input logic [WIDTH-1:0] left,
   input logic [WIDTH-1:0] right,
   output logic [WIDTH-1:0] out
);

  assign out = left + right;
endmodule

module std_reg #(
    parameter WIDTH = 32
) (
   input logic [WIDTH-1:0] in,
   input logic write_en,
   input logic clk,
   input logic reset,
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

module drive(
  input logic go,
  input logic clk,
  input logic reset,
  output logic done,
  output logic [31:0] r_in,
  output logic r_write_en,
  input logic [31:0] r_out,
  input logic r_done
);
// COMPONENT START: drive
logic invoke0_go_in;
logic invoke0_go_out;
logic invoke0_done_in;
logic invoke0_done_out;
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
wire _guard0 = 1;
wire _guard1 = invoke0_done_out;
wire _guard2 = invoke0_go_out;
wire _guard3 = invoke0_go_out;
assign done = _guard1;
assign r_in =
  _guard2 ? 32'd5 :
  32'd0;
assign r_write_en = _guard3;
assign invoke0_go_in = go;
assign invoke0_done_in = r_done;
// COMPONENT END: drive
endmodule
module main(
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: main
string DATA;
int CODE;
initial begin
    CODE = $value$plusargs("DATA=%s", DATA);
    $display("DATA (path to meminit files): %s", DATA);
    $readmemh({DATA, "/mem.dat"}, mem.mem);
end
final begin
    $writememh({DATA, "/mem.out"}, mem.mem);
end
logic [31:0] r_in;
logic r_write_en;
logic r_clk;
logic r_reset;
logic [31:0] r_out;
logic r_done;
logic drive_go;
logic drive_clk;
logic drive_reset;
logic drive_done;
logic [31:0] drive_r_in;
logic drive_r_done;
logic [31:0] drive_r_out;
logic drive_r_write_en;
logic mem_addr0;
logic [31:0] mem_write_data;
logic mem_write_en;
logic mem_clk;
logic mem_reset;
logic [31:0] mem_read_data;
logic mem_done;
logic [1:0] fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic [1:0] fsm_out;
logic fsm_done;
logic r_to_mem_go_in;
logic r_to_mem_go_out;
logic r_to_mem_done_in;
logic r_to_mem_done_out;
logic invoke0_go_in;
logic invoke0_go_out;
logic invoke0_done_in;
logic invoke0_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
std_reg # (
    .WIDTH(32)
) r (
    .clk(r_clk),
    .done(r_done),
    .in(r_in),
    .out(r_out),
    .reset(r_reset),
    .write_en(r_write_en)
);
drive drive (
    .clk(drive_clk),
    .done(drive_done),
    .go(drive_go),
    .r_done(drive_r_done),
    .r_in(drive_r_in),
    .r_out(drive_r_out),
    .r_write_en(drive_r_write_en),
    .reset(drive_reset)
);
std_mem_d1 # (
    .IDX_SIZE(1),
    .SIZE(1),
    .WIDTH(32)
) mem (
    .addr0(mem_addr0),
    .clk(mem_clk),
    .done(mem_done),
    .read_data(mem_read_data),
    .reset(mem_reset),
    .write_data(mem_write_data),
    .write_en(mem_write_en)
);
std_reg # (
    .WIDTH(2)
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
) r_to_mem_go (
    .in(r_to_mem_go_in),
    .out(r_to_mem_go_out)
);
std_wire # (
    .WIDTH(1)
) r_to_mem_done (
    .in(r_to_mem_done_in),
    .out(r_to_mem_done_out)
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
wire _guard0 = 1;
wire _guard1 = tdcc_done_out;
wire _guard2 = fsm_out == 2'd2;
wire _guard3 = fsm_out == 2'd0;
wire _guard4 = invoke0_done_out;
wire _guard5 = _guard3 & _guard4;
wire _guard6 = tdcc_go_out;
wire _guard7 = _guard5 & _guard6;
wire _guard8 = _guard2 | _guard7;
wire _guard9 = fsm_out == 2'd1;
wire _guard10 = r_to_mem_done_out;
wire _guard11 = _guard9 & _guard10;
wire _guard12 = tdcc_go_out;
wire _guard13 = _guard11 & _guard12;
wire _guard14 = _guard8 | _guard13;
wire _guard15 = fsm_out == 2'd0;
wire _guard16 = invoke0_done_out;
wire _guard17 = _guard15 & _guard16;
wire _guard18 = tdcc_go_out;
wire _guard19 = _guard17 & _guard18;
wire _guard20 = fsm_out == 2'd2;
wire _guard21 = fsm_out == 2'd1;
wire _guard22 = r_to_mem_done_out;
wire _guard23 = _guard21 & _guard22;
wire _guard24 = tdcc_go_out;
wire _guard25 = _guard23 & _guard24;
wire _guard26 = invoke0_go_out;
wire _guard27 = invoke0_go_out;
wire _guard28 = invoke0_done_out;
wire _guard29 = ~_guard28;
wire _guard30 = fsm_out == 2'd0;
wire _guard31 = _guard29 & _guard30;
wire _guard32 = tdcc_go_out;
wire _guard33 = _guard31 & _guard32;
wire _guard34 = r_to_mem_done_out;
wire _guard35 = ~_guard34;
wire _guard36 = fsm_out == 2'd1;
wire _guard37 = _guard35 & _guard36;
wire _guard38 = tdcc_go_out;
wire _guard39 = _guard37 & _guard38;
wire _guard40 = invoke0_go_out;
wire _guard41 = invoke0_go_out;
wire _guard42 = invoke0_go_out;
wire _guard43 = fsm_out == 2'd2;
wire _guard44 = r_to_mem_go_out;
wire _guard45 = r_to_mem_go_out;
wire _guard46 = r_to_mem_go_out;
assign done = _guard1;
assign fsm_write_en = _guard14;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard19 ? 2'd1 :
  _guard20 ? 2'd0 :
  _guard25 ? 2'd2 :
  2'd0;
always_comb begin
  if(~$onehot0({_guard25, _guard20, _guard19})) begin
    $fatal(2, "Multiple assignment to port `fsm.in'.");
end
end
assign r_to_mem_done_in = mem_done;
assign r_write_en =
  _guard26 ? drive_r_write_en :
  1'd0;
assign r_clk = clk;
assign r_reset = reset;
assign r_in = drive_r_in;
assign invoke0_go_in = _guard33;
assign tdcc_go_in = go;
assign r_to_mem_go_in = _guard39;
assign invoke0_done_in = drive_done;
assign drive_r_out =
  _guard40 ? r_out :
  32'd0;
assign drive_clk = clk;
assign drive_r_done =
  _guard41 ? r_done :
  1'd0;
assign drive_go = _guard42;
assign drive_reset = reset;
assign tdcc_done_in = _guard43;
assign mem_write_en = _guard44;
assign mem_clk = clk;
assign mem_addr0 =
  _guard45 ? 1'd0 :
  1'd0;
assign mem_reset = reset;
assign mem_write_data = r_out;
// COMPONENT END: main
endmodule
