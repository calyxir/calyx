module fixed_p_std_const #(
    parameter width = 32,
    parameter int_width = 8,
    parameter fract_width = 24,
    parameter [int_width-1:0] value1 = 0,
    parameter [fract_width-1:0] value2 = 0
) (
    output logic [width-1:0] out
);
  assign out = {value1, value2};
endmodule


module fixed_p_std_add #(
    parameter width = 32,
    parameter int_width = 8,
    parameter fract_width = 24
) (
    input  logic [width-1:0] left,
    input  logic [width-1:0] right,
    output logic [width-1:0] out
);
  assign out = left + right;
endmodule

module fixed_p_std_sub #(
    parameter width = 32,
    parameter int_width = 8,
    parameter fract_width = 24
) (
    input  logic [width-1:0] left,
    input  logic [width-1:0] right,
    output logic [width-1:0] out
);
  assign out = left - right;
endmodule

module fixed_p_std_gt #(
    parameter width = 32,
    parameter int_width = 8,
    parameter fract_width = 24
) (
    input  logic [width-1:0] left,
    input  logic [width-1:0] right,
    output logic             out
);
  assign out = left > right;
endmodule

module fixed_p_std_add_dbit #(
    parameter width1 = 32,
    parameter width2 = 32,
    parameter int_width1 = 8,
    parameter fract_width1 = 24,
    parameter int_width2 = 4,
    parameter fract_width2 = 28,
    parameter outwidth = 36
) (
    input  logic [   width1-1:0] left,
    input  logic [   width2-1:0] right,
    output logic [outwidth-1:0] out
);

  localparam bigINT = (int_width1 >= int_width2) ? int_width1 : int_width2;
  localparam bigfract = (fract_width1 >= fract_width2) ? fract_width1 : fract_width2;

  if (bigINT + bigfract != outwidth)
    $error("fixed_p_std_add_dbit: Given output width not equal to computed output width");

  logic [int_width1-1:0] left_int;
  logic [int_width2-1:0] right_int;
  logic [fract_width1-1:0] left_fract;
  logic [fract_width2-1:0] right_fract;

  logic [bigINT-1:0] mod_right_int;
  logic [bigfract-1:0] mod_left_fract;

  logic [bigINT-1:0] whole_int;
  logic [bigfract-1:0] whole_fract;

  assign {left_int, left_fract} = left;
  assign {right_int, right_fract} = right;

  assign mod_left_fract = left_fract * (2 ** (fract_width2 - fract_width1));

  always_comb begin
    if ((mod_left_fract + right_fract) >= 2 ** fract_width2) begin
      whole_int = left_int + right_int + 1;
      whole_fract = mod_left_fract + right_fract - 2 ** fract_width2;
    end else begin
      whole_int = left_int + right_int;
      whole_fract = mod_left_fract + right_fract;
    end
  end

  assign out = {whole_int, whole_fract};
endmodule

/// ========================= Unsynthesizable primitives =====================

module fixed_p_std_mult #(
    parameter width = 32,
    parameter int_width = 8,
    parameter fract_width = 24
) (
    input  logic [width-1:0] left,
    input  logic [width-1:0] right,
    output logic [width-1:0] out
);
  logic [2*width-2:0] result;

  assign result = left * right;
  assign out = result[width+fract_width-1:fract_width];
endmodule

module fixed_p_std_div #(
    parameter width = 32,
    parameter int_width = 8,
    parameter fract_width = 24
) (
    input  logic [width-1:0] left,
    input  logic [width-1:0] right,
    output logic [width-1:0] out
);
  logic [2*width-2:0] result;

  assign result = left / right;
  assign out = result[width+fract_width-1:fract_width];
endmodule
