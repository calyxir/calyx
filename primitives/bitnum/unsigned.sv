/* verilator lint_off WIDTH */
module std_mod_pipe #(
    parameter width = 32
) (
    input                    clk,
    input                    go,
    input        [width-1:0] left,
    input        [width-1:0] right,
    output logic [width-1:0] out,
    output logic             done
);

  logic [width-1:0] dividend;
  logic [(width-1)*2:0] divisor;
  logic [width-1:0] quotient;
  logic [width-1:0] quotient_msk;
  logic start, running, finished;

  assign start = go && !running;
  assign finished = !quotient_msk && running;

  always @(posedge clk) begin
    if (!go) begin
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
    end else if (finished) begin
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

  `ifdef VERILATOR
    // Simulation self test against unsynthesizable implementation.
    always @(posedge clk) begin
      if (finished && dividend != $unsigned(((left % right) + right) % right))
        $error(
          "\nstd_mod_pipe: Computed and golden outputs do not match!\n",
          "left: %0d", $unsigned(left),
          "  right: %0d\n", $unsigned(right),
          "expected: %0d", $unsigned(((left % right) + right) % right),
          "  computed: %0d", $unsigned(out)
        );
    end
  `endif
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
    input                  go,
    input      [width-1:0] left,
    input      [width-1:0] right,
    output reg [width-1:0] out,
    output reg             done
);

  wire start = go && !running;

  reg [width-1:0] dividend;
  reg [(width-1)*2:0] divisor;
  reg [width-1:0] quotient;
  reg [width-1:0] quotient_msk;
  reg running;

  always @(posedge clk) begin
    if (!go) begin
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

// ===============Signed operations that wrap unsigned ones ===============

module std_smod_pipe #(
    parameter width = 32
) (
    input                     clk,
    input                     go,
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output logic  [width-1:0] out,
    output logic              done
);

  logic signed [width-1:0] left_abs;
  logic signed [width-1:0] comp_out;

  assign left_abs = left[width-1] == 1 ? -left : left;
  assign out = left[width-1] == 1 ? $signed(right - comp_out) : comp_out;

  std_mod_pipe #(
    .width(width)
  ) comp (
    .clk(clk),
    .done(done),
    .go(go),
    .left(left_abs),
    .right(right),
    .out(comp_out)
  );

  `ifdef VERILATOR
    // Simulation self test against unsynthesizable implementation.
    always @(posedge clk) begin
      if (done && out != $signed(((left % right) + right) % right))
        $error(
          "\nstd_smod_pipe: Computed and golden outputs do not match!\n",
          "left: %0d", left,
          "  right: %0d\n", right,
          "expected: %0d", $signed(((left % right) + right) % right),
          "  computed: %0d", $signed(out)
        );
    end
  `endif
endmodule

/* verilator lint_off WIDTH */
module std_sdiv_pipe #(
    parameter width = 32
) (
    input                     clk,
    input                     go,
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output logic  [width-1:0] out,
    output logic              done
);

  logic signed [width-1:0] left_abs;
  logic signed [width-1:0] right_abs;
  logic signed [width-1:0] comp_out;

  assign right_abs = right[width-1] == 1 ? -right : right;
  assign left_abs = left[width-1] == 1 ? -left : left;
  assign out =
    (left[width-1] == 1) ^ (right[width-1] == 1) ? -comp_out : comp_out;

  std_div_pipe #(
    .width(width)
  ) comp (
    .clk(clk),
    .done(done),
    .go(go),
    .left(left_abs),
    .right(right_abs),
    .out(comp_out)
  );

  `ifdef VERILATOR
    // Simulation self test against unsynthesizable implementation.
    always @(posedge clk) begin
      if (done && out != $signed(left / right))
        $error(
          "\nstd_sdiv_pipe: Computed and golden outputs do not match!\n",
          "left: %0d", left,
          "  right: %0d\n", right,
          "expected: %0d", $signed(left / right),
          "  computed: %0d", $signed(out)
        );
    end
  `endif
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
      // XXX: This is a hilariously bad approximation
      /* verilator lint_off REALCVT */
      out <= /* 2.718281 */ 3 ** exponent;
      done <= 1;
    end else begin
      out <= 0;
      done <= 0;
    end
  end
endmodule

