module fixed_p_std_const #(
    parameter WIDTH = 32,
    parameter INT_WIDTH = 8,
    parameter FRACT_WIDTH = 24,
    parameter [INT_WIDTH-1:0] VALUE1 = 0,
    parameter [FRACT_WIDTH-1:0] VALUE2 = 0
) (
    output logic [WIDTH-1:0] out
);
  assign out = {VALUE1, VALUE2};
endmodule


module fixed_p_std_add #(
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

module fixed_p_std_sub #(
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

module fixed_p_std_gt #(
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

module fixed_p_std_add_dbit #(
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

  localparam big_int = (INT_WIDTH1 >= INT_WIDTH2) ? INT_WIDTH1 : INT_WIDTH2;
  localparam big_fract = (FRACT_WIDTH1 >= FRACT_WIDTH2) ? FRACT_WIDTH1 : FRACT_WIDTH2;

  if (big_int + big_fract != OUT_WIDTH)
    $error("fixed_p_std_add_dbit: Given output width not equal to computed output width");

  logic [INT_WIDTH1-1:0] left_int;
  logic [INT_WIDTH2-1:0] right_int;
  logic [FRACT_WIDTH1-1:0] left_fract;
  logic [FRACT_WIDTH2-1:0] right_fract;

  logic [big_int-1:0] mod_right_int;
  logic [big_fract-1:0] mod_left_fract;

  logic [big_int-1:0] whole_int;
  logic [big_fract-1:0] whole_fract;

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

module fixed_p_std_mult #(
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

module fixed_p_std_div #(
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
