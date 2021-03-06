module fixed_p_std_sadd #(
    parameter width = 32,
    parameter int_width = 8,
    parameter fract_width = 24
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed [width-1:0] out
);
  assign out = $signed(left + right);
endmodule

module fixed_p_std_ssub #(
    parameter width = 32,
    parameter int_width = 8,
    parameter fract_width = 24
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed [width-1:0] out
);

  assign out = $signed(left - right);
endmodule

module sfixed_p_std_add_dbit #(
    parameter width1 = 32,
    parameter width2 = 32,
    parameter int_width1 = 8,
    parameter fract_width1 = 24,
    parameter int_width2 = 4,
    parameter fract_width2 = 28,
    parameter out_width = 36
) (
    input  logic [   width1-1:0] left,
    input  logic [   width2-1:0] right,
    output logic [out_width-1:0] out
);

  logic signed [int_width1-1:0] left_int;
  logic signed [int_width2-1:0] right_int;
  logic [fract_width1-1:0] left_fract;
  logic [fract_width2-1:0] right_fract;

  localparam big_int = (int_width1 >= int_width2) ? int_width1 : int_width2;
  localparam big_fract = (fract_width1 >= fract_width2) ? fract_width1 : fract_width2;

  logic [big_int-1:0] mod_right_int;
  logic [big_fract-1:0] mod_left_fract;

  logic [big_int-1:0] whole_int;
  logic [big_fract-1:0] whole_fract;

  assign {left_int, left_fract} = left;
  assign {right_int, right_fract} = right;

  assign mod_left_fract = left_fract * (2 ** (fract_width2 - fract_width1));

  always_comb begin
    if ((mod_left_fract + right_fract) >= 2 ** fract_width2) begin
      whole_int = $signed(left_int + right_int + 1);
      whole_fract = mod_left_fract + right_fract - 2 ** fract_width2;
    end else begin
      whole_int = $signed(left_int + right_int);
      whole_fract = mod_left_fract + right_fract;
    end
  end

  assign out = {whole_int, whole_fract};
endmodule

module fixed_p_std_sgt #(
    parameter width = 32,
    parameter int_width = 8,
    parameter fract_width = 24
) (
    input  logic signed [width-1:0] left,
    input  logic signed [width-1:0] right,
    output logic signed             out
);
  assign out = $signed(left > right);
endmodule

module fixed_p_std_smult #(
    parameter width = 32,
    parameter int_width = 8,
    parameter fract_width = 24
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed [width-1:0] out
);

  logic [2*width-2:0] result;

  assign result = $signed(left * right);
  assign out = result[width+fract_width-1:fract_width];
endmodule

module fixed_p_std_sdiv #(
    parameter width = 32,
    parameter int_width = 8,
    parameter fract_width = 24
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed [width-1:0] out
);

  logic [2*width-2:0] result;

  assign result = $signed(left / right);
  assign out = result[width+fract_width-1:fract_width];
endmodule

