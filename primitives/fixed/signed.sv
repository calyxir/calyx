module std_fp_sadd #(
    parameter WIDTH = 32,
    parameter INT_WIDTH = 8,
    parameter FRACT_WIDTH = 24
) (
    input  signed [WIDTH-1:0] left,
    input  signed [WIDTH-1:0] right,
    output signed [WIDTH-1:0] out
);
  assign out = $signed(left + right);
endmodule

module std_fp_ssub #(
    parameter WIDTH = 32,
    parameter INT_WIDTH = 8,
    parameter FRACT_WIDTH = 24
) (
    input  signed [WIDTH-1:0] left,
    input  signed [WIDTH-1:0] right,
    output signed [WIDTH-1:0] out
);

  assign out = $signed(left - right);
endmodule

module sstd_fp_add_dwidth #(
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

  logic signed [INT_WIDTH1-1:0] left_int;
  logic signed [INT_WIDTH2-1:0] right_int;
  logic [FRACT_WIDTH1-1:0] left_fract;
  logic [FRACT_WIDTH2-1:0] right_fract;

  localparam BIG_INT = (INT_WIDTH1 >= INT_WIDTH2) ? INT_WIDTH1 : INT_WIDTH2;
  localparam BIG_FRACT = (FRACT_WIDTH1 >= FRACT_WIDTH2) ? FRACT_WIDTH1 : FRACT_WIDTH2;

  logic [BIG_INT-1:0] mod_right_int;
  logic [BIG_FRACT-1:0] mod_left_fract;

  logic [BIG_INT-1:0] whole_int;
  logic [BIG_FRACT-1:0] whole_fract;

  assign {left_int, left_fract} = left;
  assign {right_int, right_fract} = right;

  assign mod_left_fract = left_fract * (2 ** (FRACT_WIDTH2 - FRACT_WIDTH1));

  always_comb begin
    if ((mod_left_fract + right_fract) >= 2 ** FRACT_WIDTH2) begin
      whole_int = $signed(left_int + right_int + 1);
      whole_fract = mod_left_fract + right_fract - 2 ** FRACT_WIDTH2;
    end else begin
      whole_int = $signed(left_int + right_int);
      whole_fract = mod_left_fract + right_fract;
    end
  end

  assign out = {whole_int, whole_fract};
endmodule

module std_fp_sgt #(
    parameter WIDTH = 32,
    parameter INT_WIDTH = 8,
    parameter FRACT_WIDTH = 24
) (
    input  logic signed [WIDTH-1:0] left,
    input  logic signed [WIDTH-1:0] right,
    output logic signed             out
);
  assign out = $signed(left > right);
endmodule

module std_fp_smult #(
    parameter WIDTH = 32,
    parameter INT_WIDTH = 8,
    parameter FRACT_WIDTH = 24
) (
    input  signed [WIDTH-1:0] left,
    input  signed [WIDTH-1:0] right,
    output signed [WIDTH-1:0] out
);

  logic [2*WIDTH-2:0] result;

  assign result = $signed(left * right);
  assign out = result[WIDTH+FRACT_WIDTH-1:FRACT_WIDTH];
endmodule

module std_fp_sdiv #(
    parameter WIDTH = 32,
    parameter INT_WIDTH = 8,
    parameter FRACT_WIDTH = 24
) (
    input  signed [WIDTH-1:0] left,
    input  signed [WIDTH-1:0] right,
    output signed [WIDTH-1:0] out
);

  logic [2*WIDTH-2:0] result;

  assign result = $signed(left / right);
  assign out = result[WIDTH+FRACT_WIDTH-1:FRACT_WIDTH];
endmodule

