module std_fp_smult_pipe #(
    parameter WIDTH = 32,
    parameter INT_WIDTH = 16,
    parameter FRAC_WIDTH = 16
) (
    input  signed       [WIDTH-1:0] left,
    input  signed       [WIDTH-1:0] right,
    input  logic                    go,
    input  logic                    clk,
    output logic signed [WIDTH-1:0] out,
    output logic                    done
);
  logic signed [WIDTH-1:0] rtmp;
  logic signed [WIDTH-1:0] ltmp;
  logic signed [(WIDTH << 1) - 1:0] out_tmp;
  reg done_buf[1:0];
  always_ff @(posedge clk) begin
    if (go) begin
      ltmp <= left;
      rtmp <= right;
      // Sign extend by the first bit for the operands.
      out_tmp <= $signed(
                   { {WIDTH{ltmp[WIDTH-1]}}, ltmp} *
                   { {WIDTH{rtmp[WIDTH-1]}}, rtmp}
                 );
      out <= out_tmp[(WIDTH << 1) - INT_WIDTH - 1: WIDTH - INT_WIDTH];

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

module std_fp_sadd #(
    parameter WIDTH = 32,
    parameter INT_WIDTH = 16,
    parameter FRAC_WIDTH = 16
) (
    input  signed [WIDTH-1:0] left,
    input  signed [WIDTH-1:0] right,
    output signed [WIDTH-1:0] out
);
  assign out = $signed(left + right);
endmodule

module std_fp_ssub #(
    parameter WIDTH = 32,
    parameter INT_WIDTH = 16,
    parameter FRAC_WIDTH = 16
) (
    input  signed [WIDTH-1:0] left,
    input  signed [WIDTH-1:0] right,
    output signed [WIDTH-1:0] out
);

  assign out = $signed(left - right);
endmodule

module std_fp_sadd_dwidth #(
    parameter WIDTH1 = 32,
    parameter WIDTH2 = 32,
    parameter INT_WIDTH1 = 8,
    parameter FRAC_WIDTH1 = 24,
    parameter INT_WIDTH2 = 4,
    parameter FRAC_WIDTH2 = 28,
    parameter OUT_WIDTH = 36
) (
    input  logic [   WIDTH1-1:0] left,
    input  logic [   WIDTH2-1:0] right,
    output logic [OUT_WIDTH-1:0] out
);

  logic signed [INT_WIDTH1-1:0] left_int;
  logic signed [INT_WIDTH2-1:0] right_int;
  logic [FRAC_WIDTH1-1:0] left_fract;
  logic [FRAC_WIDTH2-1:0] right_fract;

  localparam BIG_INT = (INT_WIDTH1 >= INT_WIDTH2) ? INT_WIDTH1 : INT_WIDTH2;
  localparam BIG_FRACT = (FRAC_WIDTH1 >= FRAC_WIDTH2) ? FRAC_WIDTH1 : FRAC_WIDTH2;

  logic [BIG_INT-1:0] mod_right_int;
  logic [BIG_FRACT-1:0] mod_left_fract;

  logic [BIG_INT-1:0] whole_int;
  logic [BIG_FRACT-1:0] whole_fract;

  assign {left_int, left_fract} = left;
  assign {right_int, right_fract} = right;

  assign mod_left_fract = left_fract * (2 ** (FRAC_WIDTH2 - FRAC_WIDTH1));

  always_comb begin
    if ((mod_left_fract + right_fract) >= 2 ** FRAC_WIDTH2) begin
      whole_int = $signed(left_int + right_int + 1);
      whole_fract = mod_left_fract + right_fract - 2 ** FRAC_WIDTH2;
    end else begin
      whole_int = $signed(left_int + right_int);
      whole_fract = mod_left_fract + right_fract;
    end
  end

  assign out = {whole_int, whole_fract};
endmodule

module std_fp_sgt #(
    parameter WIDTH = 32,
    parameter INT_WIDTH = 16,
    parameter FRAC_WIDTH = 16
) (
    input  logic signed [WIDTH-1:0] left,
    input  logic signed [WIDTH-1:0] right,
    output logic signed             out
);
  assign out = $signed(left > right);
endmodule
