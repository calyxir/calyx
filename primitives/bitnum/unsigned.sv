module std_mod_pipe #(
    parameter width = 32
) (
    input                  clk,
    reset,
    input                  go,
    input      [width-1:0] left,
    input      [width-1:0] right,
    output reg [width-1:0] out,
    output reg             done
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
      divisor <= right << width - 1;
      quotient <= 0;
      quotient_msk <= 1 << width - 1;
    end else if (!quotient_msk && running) begin
      running <= 0;
      done <= 1;
      out <= dividend;
    end else begin
      if (divisor <= dividend) begin
        dividend <= dividend - divisor;
        quotient <= quotient | quotient_msk;
      end
      divisor <= divisor >> 1;
      quotient_msk <= quotient_msk >> 1;
    end
  end
endmodule

module std_mult_pipe #(
    parameter width = 32
) (
    input  logic [width-1:0] left,
    input  logic [width-1:0] right,
    input  logic             go,
    input  logic             clk,
    output logic [width-1:0] out,
    output logic             done
);
  logic [width-1:0] rtmp;
  logic [width-1:0] ltmp;
  logic [width-1:0] out_tmp;
  reg done_buf[1:0];
  always_ff @(posedge clk) begin
    if (go) begin
      rtmp <= right;
      ltmp <= left;
      out_tmp <= rtmp * ltmp;
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
module std_div_pipe #(
    parameter width = 32
) (
    input                  clk,
    reset,
    input                  go,
    input      [width-1:0] left,
    input      [width-1:0] right,
    output reg [width-1:0] out,
    output reg             done
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
      divisor <= right << width - 1;
      quotient <= 0;
      quotient_msk <= 1 << width - 1;
    end else if (!quotient_msk && running) begin
      running <= 0;
      done <= 1;
      out <= quotient;
    end else begin
      if (divisor <= dividend) begin
        dividend <= dividend - divisor;
        quotient <= quotient | quotient_msk;
      end
      divisor <= divisor >> 1;
      quotient_msk <= quotient_msk >> 1;
    end
  end
endmodule

module std_sqrt (
    input  logic [31:0] in,
    input  logic        go,
    input  logic        clk,
    output logic [31:0] out,
    output logic        done
);
  // declare the variables
  logic [31:0] a;
  logic [15:0] q;
  logic [17:0] left, right, r;
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
        r <= left + right;
      else  //subtract if r is positive
        r <= left - right;
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

//==================== Unsynthesizable primitives =========================
module std_mult #(
    parameter width = 32
) (
    input  logic [width-1:0] left,
    input  logic [width-1:0] right,
    output logic [width-1:0] out
);
  assign out = left * right;
endmodule

module std_div #(
    parameter width = 32
) (
    input  logic [width-1:0] left,
    input  logic [width-1:0] right,
    output logic [width-1:0] out
);
  assign out = left / right;
endmodule

module std_mod #(
    parameter width = 32
) (
    input  logic [width-1:0] left,
    input  logic [width-1:0] right,
    output logic [width-1:0] out
);
  assign out = left % right;
endmodule

module std_exp (
    input  logic [31:0] exponent,
    input  logic        go,
    input  logic        clk,
    output logic [31:0] out,
    output logic        done
);
  always_ff @(posedge clk) begin
    if (go) begin
      /* verilator lint_off REALCVT */
      out <= 2.718281 ** exponent;
      done <= 1;
    end else begin
      out <= 0;
      done <= 0;
    end
  end
endmodule

