
module std_fp_mult_pipe #(
    parameter WIDTH = 32,
    parameter INT_WIDTH = 16,
    parameter FRAC_WIDTH = 16
) (
    input  logic [WIDTH-1:0] left,
    input  logic [WIDTH-1:0] right,
    input  logic             go,
    input  logic             clk,
    output logic [WIDTH-1:0] out,
    output logic             done
);
  logic [WIDTH-1:0]          rtmp;
  logic [WIDTH-1:0]          ltmp;
  logic [(WIDTH << 1) - 1:0] out_tmp;
  reg done_buf[1:0];
  always_ff @(posedge clk) begin
    if (go) begin
      rtmp <= right;
      ltmp <= left;
      out_tmp <= ltmp * rtmp;
      out <= out_tmp[(WIDTH << 1) - INT_WIDTH - 1 : WIDTH - INT_WIDTH];

      done <= done_buf[1];
      done_buf[0] <= 1'b1;
      done_buf[1] <= done_buf[0];
    end else begin
      rtmp <= 0;
      ltmp <= 0;
      out_tmp <= 0;
      out <= 0;

      done <= 0;
      done_buf[0] <= 0;
      done_buf[1] <= 0;
    end
  end
endmodule

module std_fp_add #(
    parameter WIDTH = 32,
    parameter INT_WIDTH = 8,
    parameter FRACT_WIDTH = 24
) (
    input  logic [WIDTH-1:0] left,
    input  logic [WIDTH-1:0] right,
    output logic [WIDTH-1:0] out
);
  assign out = left + right;
endmodule

module std_fp_sub #(
    parameter WIDTH = 32,
    parameter INT_WIDTH = 8,
    parameter FRACT_WIDTH = 24
) (
    input  logic [WIDTH-1:0] left,
    input  logic [WIDTH-1:0] right,
    output logic [WIDTH-1:0] out
);
  assign out = left - right;
endmodule

module std_fp_gt #(
    parameter WIDTH = 32,
    parameter INT_WIDTH = 8,
    parameter FRACT_WIDTH = 24
) (
    input  logic [WIDTH-1:0] left,
    input  logic [WIDTH-1:0] right,
    output logic             out
);
  assign out = left > right;
endmodule

module std_fp_add_dwidth #(
    parameter WIDTH1 = 32,
    parameter WIDTH2 = 32,
    parameter INT_WIDTH1 = 8,
    parameter FRACT_WIDTH1 = 24,
    parameter INT_WIDTH2 = 4,
    parameter FRACT_WIDTH2 = 28,
    parameter OUT_WIDTH = 36
) (
    input  logic [   WIDTH1-1:0] left,
    input  logic [   WIDTH2-1:0] right,
    output logic [OUT_WIDTH-1:0] out
);

  localparam BIG_INT = (INT_WIDTH1 >= INT_WIDTH2) ? INT_WIDTH1 : INT_WIDTH2;
  localparam BIG_FRACT = (FRACT_WIDTH1 >= FRACT_WIDTH2) ? FRACT_WIDTH1 : FRACT_WIDTH2;

  if (BIG_INT + BIG_FRACT != OUT_WIDTH)
    $error("std_fp_add_dwidth: Given output width not equal to computed output width");

  logic [INT_WIDTH1-1:0] left_int;
  logic [INT_WIDTH2-1:0] right_int;
  logic [FRACT_WIDTH1-1:0] left_fract;
  logic [FRACT_WIDTH2-1:0] right_fract;

  logic [BIG_INT-1:0] mod_right_int;
  logic [BIG_FRACT-1:0] mod_left_fract;

  logic [BIG_INT-1:0] whole_int;
  logic [BIG_FRACT-1:0] whole_fract;

  assign {left_int, left_fract} = left;
  assign {right_int, right_fract} = right;

  assign mod_left_fract = left_fract * (2 ** (FRACT_WIDTH2 - FRACT_WIDTH1));

  always_comb begin
    if ((mod_left_fract + right_fract) >= 2 ** FRACT_WIDTH2) begin
      whole_int = left_int + right_int + 1;
      whole_fract = mod_left_fract + right_fract - 2 ** FRACT_WIDTH2;
    end else begin
      whole_int = left_int + right_int;
      whole_fract = mod_left_fract + right_fract;
    end
  end

  assign out = {whole_int, whole_fract};
endmodule

/// ========================= Unsynthesizable primitives =====================

module std_fp_mult #(
    parameter WIDTH = 32,
    parameter INT_WIDTH = 8,
    parameter FRACT_WIDTH = 24
) (
    input  logic [WIDTH-1:0] left,
    input  logic [WIDTH-1:0] right,
    output logic [WIDTH-1:0] out
);
  logic [2*WIDTH-2:0] result;

  assign result = left * right;
  assign out = result[WIDTH+FRACT_WIDTH-1:FRACT_WIDTH];
endmodule

module std_fp_div #(
    parameter WIDTH = 32,
    parameter INT_WIDTH = 8,
    parameter FRACT_WIDTH = 24
) (
    input  logic [WIDTH-1:0] left,
    input  logic [WIDTH-1:0] right,
    output logic [WIDTH-1:0] out
);
  logic [2*WIDTH-2:0] result;

  assign result = left / right;
  assign out = result[WIDTH+FRACT_WIDTH-1:FRACT_WIDTH];
endmodule
