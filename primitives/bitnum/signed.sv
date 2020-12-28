/**
* Signed arthimetic primitives for FuTIL.
*/

module std_slsh #(
    parameter width = 32
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed [width-1:0] out
);
  assign out = left <<< right;
endmodule

module std_srsh #(
    parameter width = 32
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed [width-1:0] out
);
  assign out = left >>> right;
endmodule

module std_sadd #(
    parameter width = 32
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed [width-1:0] out
);
  assign out = $signed(left + right);
endmodule

module std_ssub #(
    parameter width = 32
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed [width-1:0] out
);
  assign out = $signed(left - right);
endmodule

module std_smod_pipe #(
    parameter width = 32
) (
    input                     clk,
    reset,
    input                     go,
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output reg    [width-1:0] out,
    output reg                done
);


  always_ff @(posedge clk) begin
    if (go) begin
      done <= 1;
      out <= $signed(((left % right) + right) % right);
    end else begin
      done <= 0;
      out <= 0;
    end
  end

  // TODO(rachit): This implementation is incorrect. It generates something
  // that is not the modulus of the input.
  /*logic [width-1:0] dividend;
  logic [(width-1)*2:0] divisor;
  logic [width-1:0] quotient;
  logic [width-1:0] quotient_msk;
  logic running;
  logic start;

  // TODO(rachit): Initial values are not synthesizable. Remove this.
  assign start = go && !running && !reset;
  always @(posedge clk) begin
    if (reset || !go) begin
      running <= 0;
      done <= 0;
      out <= 0;
    end else if (start && left == 0) begin
      out <= 0;
      done <= 1;
    end

    if (start) begin
      running <= 1;
      dividend <= left;
      divisor <= (right <<< width - 1);
      quotient <= 0;
      quotient_msk <= (1 <<< width - 1);
    end else if (!quotient_msk && running) begin
      running <= 0;
      done <= 1;
      out <= dividend;
    end else begin
      if (divisor <= dividend) begin
        dividend <= (dividend - divisor);
        quotient <= (quotient | quotient_msk);
      end
      divisor <= (divisor >>> 1);
      quotient_msk <= (quotient_msk >>> 1);
    end
  end*/
endmodule

module std_smult_pipe #(
    parameter width = 32
) (
    input  logic              go,
    input  logic              clk,
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed [width-1:0] out,
    output logic              done
);
  logic signed [width-1:0] rtmp;
  logic signed [width-1:0] ltmp;
  logic signed [width-1:0] out_tmp;
  reg done_buf[1:0];
  always_ff @(posedge clk) begin
    if (go) begin
      rtmp <= right;
      ltmp <= left;
      out_tmp <= $signed(rtmp * ltmp);
      out <= out_tmp;

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

/* verilator lint_off WIDTH */
module std_sdiv_pipe #(
    parameter width = 32
) (
    input                     clk,
    reset,
    input                     go,
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output reg    [width-1:0] out,
    output reg                done
);

  wire start = go && !running && !reset;

  reg [width-1:0] dividend;
  reg [(width-1)*2:0] divisor;
  reg [width-1:0] quotient;
  reg [width-1:0] quotient_msk;
  reg running;

  always @(posedge clk) begin
    if (reset || !go) begin
      running <= 0;
      done <= 0;
      out <= 0;
    end else if (start && left == 0) begin
      out <= 0;
      done <= 1;
    end
    if (start) begin
      running <= 1;
      dividend <= left;
      divisor <= $signed(right << width - 1);
      quotient <= 0;
      quotient_msk <= $signed(1 << width - 1);
    end else if (!quotient_msk && running) begin
      running <= 0;
      done <= 1;
      out <= quotient;
    end else begin
      if (divisor <= dividend) begin
        dividend <= $signed(dividend - divisor);
        quotient <= $signed(quotient | quotient_msk);
      end
      divisor <= $signed(divisor >> 1);
      quotient_msk <= $signed(quotient_msk >> 1);
    end
  end
endmodule

module std_sgt #(
    parameter width = 32
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed             out
);
  assign out = $signed(left > right);
endmodule

module std_slt #(
    parameter width = 32
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed             out
);
  assign out = $signed(left < right);
endmodule

module std_seq #(
    parameter width = 32
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed             out
);
  assign out = $signed(left == right);
endmodule

module std_sneq #(
    parameter width = 32
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed             out
);
  assign out = $signed(left != right);
endmodule

module std_sge #(
    parameter width = 32
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed             out
);
  assign out = $signed(left >= right);
endmodule

module std_sle #(
    parameter width = 32
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed             out
);
  assign out = $signed(left <= right);
endmodule

module std_ssqrt (
    input  logic         go,
    input  logic         clk,
    input  signed [31:0] in,
    output signed [31:0] out,
    output logic         done
);
  // declare the variables
  logic signed [31:0] a;
  logic signed [15:0] q;
  logic signed [17:0] left, right, r;
  integer i;

  always_ff @(posedge clk) begin
    if (go && i == 0) begin
      // initialize all the variables.
      a <= in;
      q <= 0;
      i <= 1;
      left <= 0;  // input to adder/sub
      right <= 0;  // input to adder/sub
      r <= 0;  // remainder
      // run the calculations for 16 iterations.
    end else if (go && i <= 16) begin
      right <= {q, r[17], 1'b1};
      left <= {r[15:0], a[31:30]};
      a <= {a[29:0], 2'b00};  //left shift by 2 bits.

      if (r[17] == 1)  //add if r is negative
        r <= $signed(left + right);
      else  //subtract if r is positive
        r <= $signed(left - right);

      q <= {q[14:0], !r[17]};
      if (i == 16) begin
        out <= {16'd0, q};  //final assignment of output.
        i <= 0;
        done <= 1;
      end else i <= i + 1;
    end else begin
      // initialize all the variables.
      a <= in;
      q <= 0;
      i <= 0;
      left <= 0;  // input to adder/sub
      right <= 0;  // input to adder/sub
      r <= 0;  // remainder
      done <= 0;
    end
  end
endmodule

module std_sdiv #(
    parameter width = 32
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed [width-1:0] out
);
  assign out = $signed(left / right);
endmodule

module std_smod #(
    parameter width = 32
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed [width-1:0] out
);
  assign out = $signed(left % right);
endmodule


module std_smult #(
    parameter width = 32
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed [width-1:0] out
);
  assign out = $signed(left * right);
endmodule

