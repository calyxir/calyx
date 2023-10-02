/**
 * Core primitives for Calyx.
 * Implements core primitives used by the compiler.
 *
 * Conventions:
 * - All parameter names must be SNAKE_CASE and all caps.
 * - Port names must be snake_case, no caps.
 */
`default_nettype none

module core_top #(
    parameter IN_WIDTH  = 32,
    parameter OUT_WIDTH = 32
) (
    input  wire logic [ IN_WIDTH-1:0] in,
    output logic      [OUT_WIDTH-1:0] out,
    input  wire logic [0:0] b_in,
    output logic      [0:0] b_out,
    input  wire logic [3:0] addr_in,
);
  std_slice #(
      .IN_WIDTH (IN_WIDTH),
      .OUT_WIDTH(OUT_WIDTH)
  ) slice (
      .in (in),
      .out(out)
  );
  std_pad #(
      .IN_WIDTH (IN_WIDTH),
      .OUT_WIDTH(OUT_WIDTH)
  ) pad (
      .in (in),
      .out(out)
  );
  std_cat #(
      .LEFT_WIDTH (IN_WIDTH),
      .RIGHT_WIDTH(OUT_WIDTH),
      .OUT_WIDTH  (IN_WIDTH + OUT_WIDTH)
  ) cat (
      .left (in),
      .right(out),
      .out  (out)
  );
  std_not #(
      .WIDTH(OUT_WIDTH)
  ) _not (
      .in (in),
      .out(out)
  );
  std_and #(
      .WIDTH(OUT_WIDTH)
  ) _and (
      .left (in),
      .right(out),
      .out  (out)
  );
  std_or #(
      .WIDTH(OUT_WIDTH)
  ) _or (
      .left (in),
      .right(out),
      .out  (out)
  );
  std_xor #(
      .WIDTH(OUT_WIDTH)
  ) _xor (
      .left (in),
      .right(out),
      .out  (out)
  );
  std_sub #(
      .WIDTH(OUT_WIDTH)
  ) sub (
      .left (in),
      .right(out),
      .out  (out)
  );
  std_gt #(
      .WIDTH(OUT_WIDTH)
  ) gt (
      .left (in),
      .right(out),
      .out  (b_out)
  );
  std_lt #(
      .WIDTH(OUT_WIDTH)
  ) lt (
      .left (in),
      .right(out),
      .out  (b_out)
  );
  std_eq #(
      .WIDTH(OUT_WIDTH)
  ) eq (
      .left (in),
      .right(out),
      .out  (b_out)
  );
  std_neq #(
      .WIDTH(OUT_WIDTH)
  ) neq (
      .left (in),
      .right(out),
      .out  (b_out)
  );
  std_ge #(
      .WIDTH(OUT_WIDTH)
  ) ge (
      .left (in),
      .right(out),
      .out  (b_out)
  );
  std_le #(
      .WIDTH(OUT_WIDTH)
  ) le (
      .left (in),
      .right(out),
      .out  (b_out)
  );
  std_lsh #(
      .WIDTH(OUT_WIDTH)
  ) lsh (
      .left (in),
      .right(out),
      .out  (out)
  );
  std_rsh #(
      .WIDTH(OUT_WIDTH)
  ) rsh (
      .left (in),
      .right(out),
      .out  (out)
  );
  std_mux #(
      .WIDTH(OUT_WIDTH)
  ) mux (
      .cond(b_in),
      .tru (in),
      .fal (out),
      .out (out)
  );
  std_mem_d1 #(
      .WIDTH   (OUT_WIDTH),
      .SIZE    (16),
      .IDX_SIZE(4)
  ) mem_d1 (
      .addr0     (addr_in),
      .write_data(in),
      .write_en  (1'b0),
      .clk       (1'b0),
      .reset     (1'b0),
      .read_data (out),
      .done      (b_out)
  );
  std_mem_d2 #(
      .WIDTH   (OUT_WIDTH),
      .D0_SIZE (16),
      .D1_SIZE (16),
      .D0_IDX_SIZE(4),
      .D1_IDX_SIZE(4)
  ) mem_d2 (
      .addr0     (addr_in),
      .addr1     (addr_in),
      .write_data(in),
      .write_en  (1'b0),
      .clk       (1'b0),
      .reset     (1'b0),
      .read_data (out),
      .done      (b_out)
  );
  std_mem_d3 #(
      .WIDTH   (OUT_WIDTH),
      .D0_SIZE (16),
      .D1_SIZE (16),
      .D2_SIZE (16),
      .D0_IDX_SIZE(4),
      .D1_IDX_SIZE(4),
      .D2_IDX_SIZE(4)
  ) mem_d3 (
      .addr0     (addr_in),
      .addr1     (addr_in),
      .addr2     (addr_in),
      .write_data(in),
      .write_en  (1'b0),
      .clk       (1'b0),
      .reset     (1'b0),
      .read_data (out),
      .done      (b_out)
  );
  std_mem_d4 #(
      .WIDTH   (OUT_WIDTH),
      .D0_SIZE (16),
      .D1_SIZE (16),
      .D2_SIZE (16),
      .D3_SIZE (16),
      .D0_IDX_SIZE(4),
      .D1_IDX_SIZE(4),
      .D2_IDX_SIZE(4),
      .D3_IDX_SIZE(4)
  ) mem_d4 (
      .addr0     (addr_in),
      .addr1     (addr_in),
      .addr2     (addr_in),
      .addr3     (addr_in),
      .write_data(in),
      .write_en  (1'b0),
      .clk       (1'b0),
      .reset     (1'b0),
      .read_data (out),
      .done      (b_out)
  );
endmodule

module std_slice #(
    parameter IN_WIDTH  = 32,
    parameter OUT_WIDTH = 32
) (
    input  wire logic [ IN_WIDTH-1:0] in,
    output logic      [OUT_WIDTH-1:0] out
);
  assign out = in[OUT_WIDTH-1:0];

`ifdef VERILATOR
  always_comb begin
    if (IN_WIDTH < OUT_WIDTH)
      $error(
          "std_slice: Input width less than output width\n",
          "IN_WIDTH: %0d",
          IN_WIDTH,
          "OUT_WIDTH: %0d",
          OUT_WIDTH
      );
  end
`endif
endmodule

module std_pad #(
    parameter IN_WIDTH  = 32,
    parameter OUT_WIDTH = 32
) (
    input  wire logic [ IN_WIDTH-1:0] in,
    output logic      [OUT_WIDTH-1:0] out
);
  localparam EXTEND = OUT_WIDTH - IN_WIDTH;
  assign out = {{EXTEND{1'b0}}, in};

`ifdef VERILATOR
  always_comb begin
    if (IN_WIDTH > OUT_WIDTH)
      $error(
          "std_pad: Output width less than input width\n",
          "IN_WIDTH: %0d",
          IN_WIDTH,
          "OUT_WIDTH: %0d",
          OUT_WIDTH
      );
  end
`endif
endmodule

module std_cat #(
    parameter LEFT_WIDTH  = 32,
    parameter RIGHT_WIDTH = 32,
    parameter OUT_WIDTH   = 64
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
          "LEFT_WIDTH: %0d",
          LEFT_WIDTH,
          "RIGHT_WIDTH: %0d",
          RIGHT_WIDTH,
          "OUT_WIDTH: %0d",
          OUT_WIDTH
      );
  end
`endif
endmodule

module std_not #(
    parameter WIDTH = 32
) (
    input  wire logic [WIDTH-1:0] in,
    output logic      [WIDTH-1:0] out
);
  assign out = ~in;
endmodule

module std_and #(
    parameter WIDTH = 32
) (
    input  wire logic [WIDTH-1:0] left,
    input  wire logic [WIDTH-1:0] right,
    output logic      [WIDTH-1:0] out
);
  assign out = left & right;
endmodule

module std_or #(
    parameter WIDTH = 32
) (
    input  wire logic [WIDTH-1:0] left,
    input  wire logic [WIDTH-1:0] right,
    output logic      [WIDTH-1:0] out
);
  assign out = left | right;
endmodule

module std_xor #(
    parameter WIDTH = 32
) (
    input  wire logic [WIDTH-1:0] left,
    input  wire logic [WIDTH-1:0] right,
    output logic      [WIDTH-1:0] out
);
  assign out = left ^ right;
endmodule

module std_sub #(
    parameter WIDTH = 32
) (
    input  wire logic [WIDTH-1:0] left,
    input  wire logic [WIDTH-1:0] right,
    output logic      [WIDTH-1:0] out
);
  assign out = left - right;
endmodule

module std_gt #(
    parameter WIDTH = 32
) (
    input wire logic [WIDTH-1:0] left,
    input wire logic [WIDTH-1:0] right,
    output logic out
);
  assign out = left > right;
endmodule

module std_lt #(
    parameter WIDTH = 32
) (
    input wire logic [WIDTH-1:0] left,
    input wire logic [WIDTH-1:0] right,
    output logic out
);
  assign out = left < right;
endmodule

module std_eq #(
    parameter WIDTH = 32
) (
    input wire logic [WIDTH-1:0] left,
    input wire logic [WIDTH-1:0] right,
    output logic out
);
  assign out = left == right;
endmodule

module std_neq #(
    parameter WIDTH = 32
) (
    input wire logic [WIDTH-1:0] left,
    input wire logic [WIDTH-1:0] right,
    output logic out
);
  assign out = left != right;
endmodule

module std_ge #(
    parameter WIDTH = 32
) (
    input wire logic [WIDTH-1:0] left,
    input wire logic [WIDTH-1:0] right,
    output logic out
);
  assign out = left >= right;
endmodule

module std_le #(
    parameter WIDTH = 32
) (
    input wire logic [WIDTH-1:0] left,
    input wire logic [WIDTH-1:0] right,
    output logic out
);
  assign out = left <= right;
endmodule

module std_lsh #(
    parameter WIDTH = 32
) (
    input  wire logic [WIDTH-1:0] left,
    input  wire logic [WIDTH-1:0] right,
    output logic      [WIDTH-1:0] out
);
  assign out = left << right;
endmodule

module std_rsh #(
    parameter WIDTH = 32
) (
    input  wire logic [WIDTH-1:0] left,
    input  wire logic [WIDTH-1:0] right,
    output logic      [WIDTH-1:0] out
);
  assign out = left >> right;
endmodule

/// this primitive is intended to be used
/// for lowering purposes (not in source programs)
module std_mux #(
    parameter WIDTH = 32
) (
    input  wire logic             cond,
    input  wire logic [WIDTH-1:0] tru,
    input  wire logic [WIDTH-1:0] fal,
    output logic      [WIDTH-1:0] out
);
  assign out = cond ? tru : fal;
endmodule

module std_mem_d1 #(
    parameter WIDTH = 32,
    parameter SIZE = 16,
    parameter IDX_SIZE = 4
) (
    input  wire logic [IDX_SIZE-1:0] addr0,
    input  wire logic [   WIDTH-1:0] write_data,
    input  wire logic                write_en,
    input  wire logic                clk,
    input  wire logic                reset,
    output logic      [   WIDTH-1:0] read_data,
    output logic                     done
);

  logic [WIDTH-1:0] mem[SIZE-1:0];

  /* verilator lint_off WIDTH */
  assign read_data = mem[addr0];

  always_ff @(posedge clk) begin
    if (reset) done <= '0;
    else if (write_en) done <= '1;
    else done <= '0;
  end

  always_ff @(posedge clk) begin
    if (!reset && write_en) mem[addr0] <= write_data;
  end

  // Check for out of bounds access
`ifdef VERILATOR
  always_comb begin
    if (addr0 >= SIZE)
      $error("std_mem_d1: Out of bounds access\n", "addr0: %0d\n", addr0, "SIZE: %0d", SIZE);
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
    input  wire logic [D0_IDX_SIZE-1:0] addr0,
    input  wire logic [D1_IDX_SIZE-1:0] addr1,
    input  wire logic [      WIDTH-1:0] write_data,
    input  wire logic                   write_en,
    input  wire logic                   clk,
    input  wire logic                   reset,
    output logic      [      WIDTH-1:0] read_data,
    output logic                        done
);

  /* verilator lint_off WIDTH */
  logic [WIDTH-1:0] mem[D0_SIZE-1:0][D1_SIZE-1:0];

  assign read_data = mem[addr0][addr1];

  always_ff @(posedge clk) begin
    if (reset) done <= '0;
    else if (write_en) done <= '1;
    else done <= '0;
  end

  always_ff @(posedge clk) begin
    if (!reset && write_en) mem[addr0][addr1] <= write_data;
  end

  // Check for out of bounds access
`ifdef VERILATOR
  always_comb begin
    if (addr0 >= D0_SIZE)
      $error("std_mem_d2: Out of bounds access\n", "addr0: %0d\n", addr0, "D0_SIZE: %0d", D0_SIZE);
    if (addr1 >= D1_SIZE)
      $error("std_mem_d2: Out of bounds access\n", "addr1: %0d\n", addr1, "D1_SIZE: %0d", D1_SIZE);
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
    input  wire logic [D0_IDX_SIZE-1:0] addr0,
    input  wire logic [D1_IDX_SIZE-1:0] addr1,
    input  wire logic [D2_IDX_SIZE-1:0] addr2,
    input  wire logic [      WIDTH-1:0] write_data,
    input  wire logic                   write_en,
    input  wire logic                   clk,
    input  wire logic                   reset,
    output logic      [      WIDTH-1:0] read_data,
    output logic                        done
);

  /* verilator lint_off WIDTH */
  logic [WIDTH-1:0] mem[D0_SIZE-1:0][D1_SIZE-1:0][D2_SIZE-1:0];

  assign read_data = mem[addr0][addr1][addr2];

  always_ff @(posedge clk) begin
    if (reset) done <= '0;
    else if (write_en) done <= '1;
    else done <= '0;
  end

  always_ff @(posedge clk) begin
    if (!reset && write_en) mem[addr0][addr1][addr2] <= write_data;
  end

  // Check for out of bounds access
`ifdef VERILATOR
  always_comb begin
    if (addr0 >= D0_SIZE)
      $error("std_mem_d3: Out of bounds access\n", "addr0: %0d\n", addr0, "D0_SIZE: %0d", D0_SIZE);
    if (addr1 >= D1_SIZE)
      $error("std_mem_d3: Out of bounds access\n", "addr1: %0d\n", addr1, "D1_SIZE: %0d", D1_SIZE);
    if (addr2 >= D2_SIZE)
      $error("std_mem_d3: Out of bounds access\n", "addr2: %0d\n", addr2, "D2_SIZE: %0d", D2_SIZE);
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
    input  wire logic [D0_IDX_SIZE-1:0] addr0,
    input  wire logic [D1_IDX_SIZE-1:0] addr1,
    input  wire logic [D2_IDX_SIZE-1:0] addr2,
    input  wire logic [D3_IDX_SIZE-1:0] addr3,
    input  wire logic [      WIDTH-1:0] write_data,
    input  wire logic                   write_en,
    input  wire logic                   clk,
    input  wire logic                   reset,
    output logic      [      WIDTH-1:0] read_data,
    output logic                        done
);

  /* verilator lint_off WIDTH */
  logic [WIDTH-1:0] mem[D0_SIZE-1:0][D1_SIZE-1:0][D2_SIZE-1:0][D3_SIZE-1:0];

  assign read_data = mem[addr0][addr1][addr2][addr3];

  always_ff @(posedge clk) begin
    if (reset) done <= '0;
    else if (write_en) done <= '1;
    else done <= '0;
  end

  always_ff @(posedge clk) begin
    if (!reset && write_en) mem[addr0][addr1][addr2][addr3] <= write_data;
  end

  // Check for out of bounds access
`ifdef VERILATOR
  always_comb begin
    if (addr0 >= D0_SIZE)
      $error("std_mem_d4: Out of bounds access\n", "addr0: %0d\n", addr0, "D0_SIZE: %0d", D0_SIZE);
    if (addr1 >= D1_SIZE)
      $error("std_mem_d4: Out of bounds access\n", "addr1: %0d\n", addr1, "D1_SIZE: %0d", D1_SIZE);
    if (addr2 >= D2_SIZE)
      $error("std_mem_d4: Out of bounds access\n", "addr2: %0d\n", addr2, "D2_SIZE: %0d", D2_SIZE);
    if (addr3 >= D3_SIZE)
      $error("std_mem_d4: Out of bounds access\n", "addr3: %0d\n", addr3, "D3_SIZE: %0d", D3_SIZE);
  end
`endif
endmodule

`default_nettype wire
