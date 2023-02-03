/* verilator lint_off MULTITOP */
/// =================== Unsigned, Fixed Point =========================
module std_fp_add #(
    parameter WIDTH = 32,
    parameter INT_WIDTH = 16,
    parameter FRAC_WIDTH = 16
) (
    input  logic [WIDTH-1:0] left,
    input  logic [WIDTH-1:0] right,
    output logic [WIDTH-1:0] out
);
  assign out = left + right;
endmodule

module std_fp_sub #(
    parameter WIDTH = 32,
    parameter INT_WIDTH = 16,
    parameter FRAC_WIDTH = 16
) (
    input  logic [WIDTH-1:0] left,
    input  logic [WIDTH-1:0] right,
    output logic [WIDTH-1:0] out
);
  assign out = left - right;
endmodule

module std_fp_mult_pipe #(
    parameter WIDTH = 32,
    parameter INT_WIDTH = 16,
    parameter FRAC_WIDTH = 16,
    parameter SIGNED = 0
) (
    input  logic [WIDTH-1:0] left,
    input  logic [WIDTH-1:0] right,
    input  logic             go,
    input  logic             clk,
    input  logic             reset,
    output logic [WIDTH-1:0] out,
    output logic             done
);
  logic [WIDTH-1:0]          rtmp;
  logic [WIDTH-1:0]          ltmp;
  logic [(WIDTH << 1) - 1:0] out_tmp;
  // Buffer used to walk through the 3 cycles of the pipeline.
  logic done_buf[1:0];

  assign done = done_buf[1];

  assign out = out_tmp[(WIDTH << 1) - INT_WIDTH - 1 : WIDTH - INT_WIDTH];

  // If the done buffer is completely empty and go is high then execution
  // just started.
  logic start;
  assign start = go;

  // Start sending the done signal.
  always_ff @(posedge clk) begin
    if (start)
      done_buf[0] <= 1;
    else
      done_buf[0] <= 0;
  end

  // Push the done signal through the pipeline.
  always_ff @(posedge clk) begin
    if (go) begin
      done_buf[1] <= done_buf[0];
    end else begin
      done_buf[1] <= 0;
    end
  end

  // Register the inputs
  always_ff @(posedge clk) begin
    if (reset) begin
      rtmp <= 0;
      ltmp <= 0;
    end else if (go) begin
      if (SIGNED) begin
        rtmp <= $signed(right);
        ltmp <= $signed(left);
      end else begin
        rtmp <= right;
        ltmp <= left;
      end
    end else begin
      rtmp <= 0;
      ltmp <= 0;
    end

  end

  // Compute the output and save it into out_tmp
  always_ff @(posedge clk) begin
    if (reset) begin
      out_tmp <= 0;
    end else if (go) begin
      if (SIGNED) begin
        // In the first cycle, this performs an invalid computation because
        // ltmp and rtmp only get their actual values in cycle 1
        out_tmp <= $signed(
          { {WIDTH{ltmp[WIDTH-1]}}, ltmp} *
          { {WIDTH{rtmp[WIDTH-1]}}, rtmp}
        );
      end else begin
        out_tmp <= ltmp * rtmp;
      end
    end else begin
      out_tmp <= out_tmp;
    end
  end
endmodule

/* verilator lint_off WIDTH */
module std_fp_div_pipe #(
  parameter WIDTH = 32,
  parameter INT_WIDTH = 16,
  parameter FRAC_WIDTH = 16
) (
    input  logic             go,
    input  logic             clk,
    input  logic             reset,
    input  logic [WIDTH-1:0] left,
    input  logic [WIDTH-1:0] right,
    output logic [WIDTH-1:0] out_remainder,
    output logic [WIDTH-1:0] out_quotient,
    output logic             done
);
    localparam ITERATIONS = WIDTH + FRAC_WIDTH;

    logic [WIDTH-1:0] quotient, quotient_next;
    logic [WIDTH:0] acc, acc_next;
    logic [$clog2(ITERATIONS)-1:0] idx;
    logic start, running, finished, dividend_is_zero;

    assign start = go && !running;
    assign dividend_is_zero = start && left == 0;
    assign finished = idx == ITERATIONS - 1 && running;

    always_ff @(posedge clk) begin
      if (reset || finished || dividend_is_zero)
        running <= 0;
      else if (start)
        running <= 1;
      else
        running <= running;
    end

    always_comb begin
      if (acc >= {1'b0, right}) begin
        acc_next = acc - right;
        {acc_next, quotient_next} = {acc_next[WIDTH-1:0], quotient, 1'b1};
      end else begin
        {acc_next, quotient_next} = {acc, quotient} << 1;
      end
    end

    // `done` signaling
    always_ff @(posedge clk) begin
      if (dividend_is_zero || finished)
        done <= 1;
      else
        done <= 0;
    end

    always_ff @(posedge clk) begin
      if (running)
        idx <= idx + 1;
      else
        idx <= 0;
    end

    always_ff @(posedge clk) begin
      if (reset) begin
        out_quotient <= 0;
        out_remainder <= 0;
      end else if (start) begin
        out_quotient <= 0;
        out_remainder <= left;
      end else if (go == 0) begin
        out_quotient <= out_quotient;
        out_remainder <= out_remainder;
      end else if (dividend_is_zero) begin
        out_quotient <= 0;
        out_remainder <= 0;
      end else if (finished) begin
        out_quotient <= quotient_next;
        out_remainder <= out_remainder;
      end else begin
        out_quotient <= out_quotient;
        if (right <= out_remainder)
          out_remainder <= out_remainder - right;
        else
          out_remainder <= out_remainder;
      end
    end

    always_ff @(posedge clk) begin
      if (reset) begin
        acc <= 0;
        quotient <= 0;
      end else if (start) begin
        {acc, quotient} <= {{WIDTH{1'b0}}, left, 1'b0};
      end else begin
        acc <= acc_next;
        quotient <= quotient_next;
      end
    end
endmodule

module std_fp_gt #(
    parameter WIDTH = 32,
    parameter INT_WIDTH = 16,
    parameter FRAC_WIDTH = 16
) (
    input  logic [WIDTH-1:0] left,
    input  logic [WIDTH-1:0] right,
    output logic             out
);
  assign out = left > right;
endmodule

/// =================== Signed, Fixed Point =========================
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

module std_fp_smult_pipe #(
    parameter WIDTH = 32,
    parameter INT_WIDTH = 16,
    parameter FRAC_WIDTH = 16
) (
    input  [WIDTH-1:0]              left,
    input  [WIDTH-1:0]              right,
    input  logic                    reset,
    input  logic                    go,
    input  logic                    clk,
    output logic [WIDTH-1:0]        out,
    output logic                    done
);
  std_fp_mult_pipe #(
    .WIDTH(WIDTH),
    .INT_WIDTH(INT_WIDTH),
    .FRAC_WIDTH(FRAC_WIDTH),
    .SIGNED(1)
  ) comp (
    .clk(clk),
    .done(done),
    .reset(reset),
    .go(go),
    .left(left),
    .right(right),
    .out(out)
  );
endmodule

module std_fp_sdiv_pipe #(
    parameter WIDTH = 32,
    parameter INT_WIDTH = 16,
    parameter FRAC_WIDTH = 16
) (
    input                     clk,
    input                     go,
    input                     reset,
    input  signed [WIDTH-1:0] left,
    input  signed [WIDTH-1:0] right,
    output signed [WIDTH-1:0] out_quotient,
    output signed [WIDTH-1:0] out_remainder,
    output logic              done
);

  logic signed [WIDTH-1:0] left_abs, right_abs, comp_out_q, comp_out_r, right_save, out_rem_intermediate;

  // Registers to figure out how to transform outputs.
  logic different_signs, left_sign, right_sign;

  // Latch the value of control registers so that their available after
  // go signal becomes low.
  always_ff @(posedge clk) begin
    if (go) begin
      right_save <= right_abs;
      left_sign <= left[WIDTH-1];
      right_sign <= right[WIDTH-1];
    end else begin
      left_sign <= left_sign;
      right_save <= right_save;
      right_sign <= right_sign;
    end
  end

  assign right_abs = right[WIDTH-1] ? -right : right;
  assign left_abs = left[WIDTH-1] ? -left : left;

  assign different_signs = left_sign ^ right_sign;
  assign out_quotient = different_signs ? -comp_out_q : comp_out_q;

  // Remainder is computed as:
  //  t0 = |left| % |right|
  //  t1 = if left * right < 0 and t0 != 0 then |right| - t0 else t0
  //  rem = if right < 0 then -t1 else t1
  assign out_rem_intermediate = different_signs & |comp_out_r ? $signed(right_save - comp_out_r) : comp_out_r;
  assign out_remainder = right_sign ? -out_rem_intermediate : out_rem_intermediate;

  std_fp_div_pipe #(
    .WIDTH(WIDTH),
    .INT_WIDTH(INT_WIDTH),
    .FRAC_WIDTH(FRAC_WIDTH)
  ) comp (
    .reset(reset),
    .clk(clk),
    .done(done),
    .go(go),
    .left(left_abs),
    .right(right_abs),
    .out_quotient(comp_out_q),
    .out_remainder(comp_out_r)
  );
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

module std_fp_slt #(
    parameter WIDTH = 32,
    parameter INT_WIDTH = 16,
    parameter FRAC_WIDTH = 16
) (
   input logic signed [WIDTH-1:0] left,
   input logic signed [WIDTH-1:0] right,
   output logic signed            out
);
  assign out = $signed(left < right);
endmodule

/// =================== Unsigned, Bitnum =========================
module std_mult_pipe #(
    parameter WIDTH = 32
) (
    input  logic [WIDTH-1:0] left,
    input  logic [WIDTH-1:0] right,
    input  logic             reset,
    input  logic             go,
    input  logic             clk,
    output logic [WIDTH-1:0] out,
    output logic             done
);
  std_fp_mult_pipe #(
    .WIDTH(WIDTH),
    .INT_WIDTH(WIDTH),
    .FRAC_WIDTH(0),
    .SIGNED(0)
  ) comp (
    .reset(reset),
    .clk(clk),
    .done(done),
    .go(go),
    .left(left),
    .right(right),
    .out(out)
  );
endmodule

module std_div_pipe #(
    parameter WIDTH = 32
) (
    input                    reset,
    input                    clk,
    input                    go,
    input        [WIDTH-1:0] left,
    input        [WIDTH-1:0] right,
    output logic [WIDTH-1:0] out_remainder,
    output logic [WIDTH-1:0] out_quotient,
    output logic             done
);

  logic [WIDTH-1:0] dividend;
  logic [(WIDTH-1)*2:0] divisor;
  logic [WIDTH-1:0] quotient;
  logic [WIDTH-1:0] quotient_msk;
  logic start, running, finished, dividend_is_zero;

  assign start = go && !running;
  assign finished = quotient_msk == 0 && running;
  assign dividend_is_zero = start && left == 0;

  always_ff @(posedge clk) begin
    // Early return if the divisor is zero.
    if (finished || dividend_is_zero)
      done <= 1;
    else
      done <= 0;
  end

  always_ff @(posedge clk) begin
    if (reset || finished || dividend_is_zero)
      running <= 0;
    else if (start)
      running <= 1;
    else
      running <= running;
  end

  // Outputs
  always_ff @(posedge clk) begin
    if (dividend_is_zero || start) begin
      out_quotient <= 0;
      out_remainder <= 0;
    end else if (finished) begin
      out_quotient <= quotient;
      out_remainder <= dividend;
    end else begin
      // Otherwise, explicitly latch the values.
      out_quotient <= out_quotient;
      out_remainder <= out_remainder;
    end
  end

  // Calculate the quotient mask.
  always_ff @(posedge clk) begin
    if (start)
      quotient_msk <= 1 << WIDTH - 1;
    else if (running)
      quotient_msk <= quotient_msk >> 1;
    else
      quotient_msk <= quotient_msk;
  end

  // Calculate the quotient.
  always_ff @(posedge clk) begin
    if (start)
      quotient <= 0;
    else if (divisor <= dividend)
      quotient <= quotient | quotient_msk;
    else
      quotient <= quotient;
  end

  // Calculate the dividend.
  always_ff @(posedge clk) begin
    if (start)
      dividend <= left;
    else if (divisor <= dividend)
      dividend <= dividend - divisor;
    else
      dividend <= dividend;
  end

  always_ff @(posedge clk) begin
    if (start) begin
      divisor <= right << WIDTH - 1;
    end else if (finished) begin
      divisor <= 0;
    end else begin
      divisor <= divisor >> 1;
    end
  end

  // Simulation self test against unsynthesizable implementation.
  `ifdef VERILATOR
    logic [WIDTH-1:0] l, r;
    always_ff @(posedge clk) begin
      if (go) begin
        l <= left;
        r <= right;
      end else begin
        l <= l;
        r <= r;
      end
    end

    always @(posedge clk) begin
      if (done && $unsigned(out_remainder) != $unsigned(l % r))
        $error(
          "\nstd_div_pipe (Remainder): Computed and golden outputs do not match!\n",
          "left: %0d", $unsigned(l),
          "  right: %0d\n", $unsigned(r),
          "expected: %0d", $unsigned(l % r),
          "  computed: %0d", $unsigned(out_remainder)
        );

      if (done && $unsigned(out_quotient) != $unsigned(l / r))
        $error(
          "\nstd_div_pipe (Quotient): Computed and golden outputs do not match!\n",
          "left: %0d", $unsigned(l),
          "  right: %0d\n", $unsigned(r),
          "expected: %0d", $unsigned(l / r),
          "  computed: %0d", $unsigned(out_quotient)
        );
    end
  `endif
endmodule

/// =================== Signed, Bitnum =========================
module std_sadd #(
    parameter WIDTH = 32
) (
    input  signed [WIDTH-1:0] left,
    input  signed [WIDTH-1:0] right,
    output signed [WIDTH-1:0] out
);
  assign out = $signed(left + right);
endmodule

module std_ssub #(
    parameter WIDTH = 32
) (
    input  signed [WIDTH-1:0] left,
    input  signed [WIDTH-1:0] right,
    output signed [WIDTH-1:0] out
);
  assign out = $signed(left - right);
endmodule

module std_smult_pipe #(
    parameter WIDTH = 32
) (
    input  logic                    reset,
    input  logic                    go,
    input  logic                    clk,
    input  signed       [WIDTH-1:0] left,
    input  signed       [WIDTH-1:0] right,
    output logic signed [WIDTH-1:0] out,
    output logic                    done
);
  std_fp_mult_pipe #(
    .WIDTH(WIDTH),
    .INT_WIDTH(WIDTH),
    .FRAC_WIDTH(0),
    .SIGNED(1)
  ) comp (
    .reset(reset),
    .clk(clk),
    .done(done),
    .go(go),
    .left(left),
    .right(right),
    .out(out)
  );
endmodule

/* verilator lint_off WIDTH */
module std_sdiv_pipe #(
    parameter WIDTH = 32
) (
    input                           reset,
    input                           clk,
    input                           go,
    input  logic signed [WIDTH-1:0] left,
    input  logic signed [WIDTH-1:0] right,
    output logic signed [WIDTH-1:0] out_quotient,
    output logic signed [WIDTH-1:0] out_remainder,
    output logic                    done
);

  logic signed [WIDTH-1:0] left_abs, right_abs, comp_out_q, comp_out_r, right_save, out_rem_intermediate;

  // Registers to figure out how to transform outputs.
  logic different_signs, left_sign, right_sign;

  // Latch the value of control registers so that their available after
  // go signal becomes low.
  always_ff @(posedge clk) begin
    if (go) begin
      right_save <= right_abs;
      left_sign <= left[WIDTH-1];
      right_sign <= right[WIDTH-1];
    end else begin
      left_sign <= left_sign;
      right_save <= right_save;
      right_sign <= right_sign;
    end
  end

  assign right_abs = right[WIDTH-1] ? -right : right;
  assign left_abs = left[WIDTH-1] ? -left : left;

  assign different_signs = left_sign ^ right_sign;
  assign out_quotient = different_signs ? -comp_out_q : comp_out_q;

  // Remainder is computed as:
  //  t0 = |left| % |right|
  //  t1 = if left * right < 0 and t0 != 0 then |right| - t0 else t0
  //  rem = if right < 0 then -t1 else t1
  assign out_rem_intermediate = different_signs & |comp_out_r ? $signed(right_save - comp_out_r) : comp_out_r;
  assign out_remainder = right_sign ? -out_rem_intermediate : out_rem_intermediate;

  std_div_pipe #(
    .WIDTH(WIDTH)
  ) comp (
    .reset(reset),
    .clk(clk),
    .done(done),
    .go(go),
    .left(left_abs),
    .right(right_abs),
    .out_quotient(comp_out_q),
    .out_remainder(comp_out_r)
  );

  // Simulation self test against unsynthesizable implementation.
  `ifdef VERILATOR
    logic signed [WIDTH-1:0] l, r;
    always_ff @(posedge clk) begin
      if (go) begin
        l <= left;
        r <= right;
      end else begin
        l <= l;
        r <= r;
      end
    end

    always @(posedge clk) begin
      if (done && out_quotient != $signed(l / r))
        $error(
          "\nstd_sdiv_pipe (Quotient): Computed and golden outputs do not match!\n",
          "left: %0d", l,
          "  right: %0d\n", r,
          "expected: %0d", $signed(l / r),
          "  computed: %0d", $signed(out_quotient),
        );
      if (done && out_remainder != $signed(((l % r) + r) % r))
        $error(
          "\nstd_sdiv_pipe (Remainder): Computed and golden outputs do not match!\n",
          "left: %0d", l,
          "  right: %0d\n", r,
          "expected: %0d", $signed(((l % r) + r) % r),
          "  computed: %0d", $signed(out_remainder),
        );
    end
  `endif
endmodule

module std_sgt #(
    parameter WIDTH = 32
) (
    input  signed [WIDTH-1:0] left,
    input  signed [WIDTH-1:0] right,
    output signed             out
);
  assign out = $signed(left > right);
endmodule

module std_slt #(
    parameter WIDTH = 32
) (
    input  signed [WIDTH-1:0] left,
    input  signed [WIDTH-1:0] right,
    output signed             out
);
  assign out = $signed(left < right);
endmodule

module std_seq #(
    parameter WIDTH = 32
) (
    input  signed [WIDTH-1:0] left,
    input  signed [WIDTH-1:0] right,
    output signed             out
);
  assign out = $signed(left == right);
endmodule

module std_sneq #(
    parameter WIDTH = 32
) (
    input  signed [WIDTH-1:0] left,
    input  signed [WIDTH-1:0] right,
    output signed             out
);
  assign out = $signed(left != right);
endmodule

module std_sge #(
    parameter WIDTH = 32
) (
    input  signed [WIDTH-1:0] left,
    input  signed [WIDTH-1:0] right,
    output signed             out
);
  assign out = $signed(left >= right);
endmodule

module std_sle #(
    parameter WIDTH = 32
) (
    input  signed [WIDTH-1:0] left,
    input  signed [WIDTH-1:0] right,
    output signed             out
);
  assign out = $signed(left <= right);
endmodule

module std_slsh #(
    parameter WIDTH = 32
) (
    input  signed [WIDTH-1:0] left,
    input  signed [WIDTH-1:0] right,
    output signed [WIDTH-1:0] out
);
  assign out = left <<< right;
endmodule

module std_srsh #(
    parameter WIDTH = 32
) (
    input  signed [WIDTH-1:0] left,
    input  signed [WIDTH-1:0] right,
    output signed [WIDTH-1:0] out
);
  assign out = left >>> right;
endmodule

/**
 * Core primitives for Calyx.
 * Implements core primitives used by the compiler.
 *
 * Conventions:
 * - All parameter names must be SNAKE_CASE and all caps.
 * - Port names must be snake_case, no caps.
 */
`default_nettype none

module std_const #(
    parameter WIDTH = 32,
    parameter VALUE = 0
) (
   output logic [WIDTH - 1:0] out
);
  assign out = VALUE;
endmodule

module std_wire #(
  parameter WIDTH = 32
) (
  input wire logic [WIDTH - 1:0] in,
  output logic [WIDTH - 1:0] out
);
  assign out = in;
endmodule

module std_slice #(
    parameter IN_WIDTH  = 32,
    parameter OUT_WIDTH = 32
) (
   input wire                   logic [ IN_WIDTH-1:0] in,
   output logic [OUT_WIDTH-1:0] out
);
  assign out = in[OUT_WIDTH-1:0];

  `ifdef VERILATOR
    always_comb begin
      if (IN_WIDTH < OUT_WIDTH)
        $error(
          "std_slice: Input width less than output width\n",
          "IN_WIDTH: %0d", IN_WIDTH,
          "OUT_WIDTH: %0d", OUT_WIDTH
        );
    end
  `endif
endmodule

module std_pad #(
    parameter IN_WIDTH  = 32,
    parameter OUT_WIDTH = 32
) (
   input wire logic [IN_WIDTH-1:0]  in,
   output logic     [OUT_WIDTH-1:0] out
);
  localparam EXTEND = OUT_WIDTH - IN_WIDTH;
  assign out = { {EXTEND {1'b0}}, in};

  `ifdef VERILATOR
    always_comb begin
      if (IN_WIDTH > OUT_WIDTH)
        $error(
          "std_pad: Output width less than input width\n",
          "IN_WIDTH: %0d", IN_WIDTH,
          "OUT_WIDTH: %0d", OUT_WIDTH
        );
    end
  `endif
endmodule

module std_cat #(
  parameter LEFT_WIDTH  = 32,
  parameter RIGHT_WIDTH = 32,
  parameter OUT_WIDTH = 64
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
          "LEFT_WIDTH: %0d", LEFT_WIDTH,
          "RIGHT_WIDTH: %0d", RIGHT_WIDTH,
          "OUT_WIDTH: %0d", OUT_WIDTH
        );
    end
  `endif
endmodule

module std_not #(
    parameter WIDTH = 32
) (
   input wire               logic [WIDTH-1:0] in,
   output logic [WIDTH-1:0] out
);
  assign out = ~in;
endmodule

module std_and #(
    parameter WIDTH = 32
) (
   input wire               logic [WIDTH-1:0] left,
   input wire               logic [WIDTH-1:0] right,
   output logic [WIDTH-1:0] out
);
  assign out = left & right;
endmodule

module std_or #(
    parameter WIDTH = 32
) (
   input wire               logic [WIDTH-1:0] left,
   input wire               logic [WIDTH-1:0] right,
   output logic [WIDTH-1:0] out
);
  assign out = left | right;
endmodule

module std_xor #(
    parameter WIDTH = 32
) (
   input wire               logic [WIDTH-1:0] left,
   input wire               logic [WIDTH-1:0] right,
   output logic [WIDTH-1:0] out
);
  assign out = left ^ right;
endmodule

module std_add #(
    parameter WIDTH = 32
) (
   input wire               logic [WIDTH-1:0] left,
   input wire               logic [WIDTH-1:0] right,
   output logic [WIDTH-1:0] out
);
  assign out = left + right;
endmodule

module std_sub #(
    parameter WIDTH = 32
) (
   input wire               logic [WIDTH-1:0] left,
   input wire               logic [WIDTH-1:0] right,
   output logic [WIDTH-1:0] out
);
  assign out = left - right;
endmodule

module std_gt #(
    parameter WIDTH = 32
) (
   input wire   logic [WIDTH-1:0] left,
   input wire   logic [WIDTH-1:0] right,
   output logic out
);
  assign out = left > right;
endmodule

module std_lt #(
    parameter WIDTH = 32
) (
   input wire   logic [WIDTH-1:0] left,
   input wire   logic [WIDTH-1:0] right,
   output logic out
);
  assign out = left < right;
endmodule

module std_eq #(
    parameter WIDTH = 32
) (
   input wire   logic [WIDTH-1:0] left,
   input wire   logic [WIDTH-1:0] right,
   output logic out
);
  assign out = left == right;
endmodule

module std_neq #(
    parameter WIDTH = 32
) (
   input wire   logic [WIDTH-1:0] left,
   input wire   logic [WIDTH-1:0] right,
   output logic out
);
  assign out = left != right;
endmodule

module std_ge #(
    parameter WIDTH = 32
) (
    input wire   logic [WIDTH-1:0] left,
    input wire   logic [WIDTH-1:0] right,
    output logic out
);
  assign out = left >= right;
endmodule

module std_le #(
    parameter WIDTH = 32
) (
   input wire   logic [WIDTH-1:0] left,
   input wire   logic [WIDTH-1:0] right,
   output logic out
);
  assign out = left <= right;
endmodule

module std_lsh #(
    parameter WIDTH = 32
) (
   input wire               logic [WIDTH-1:0] left,
   input wire               logic [WIDTH-1:0] right,
   output logic [WIDTH-1:0] out
);
  assign out = left << right;
endmodule

module std_rsh #(
    parameter WIDTH = 32
) (
   input wire               logic [WIDTH-1:0] left,
   input wire               logic [WIDTH-1:0] right,
   output logic [WIDTH-1:0] out
);
  assign out = left >> right;
endmodule

/// this primitive is intended to be used
/// for lowering purposes (not in source programs)
module std_mux #(
    parameter WIDTH = 32
) (
   input wire               logic cond,
   input wire               logic [WIDTH-1:0] tru,
   input wire               logic [WIDTH-1:0] fal,
   output logic [WIDTH-1:0] out
);
  assign out = cond ? tru : fal;
endmodule

/// Memories
module std_reg #(
    parameter WIDTH = 32
) (
   input wire [ WIDTH-1:0]    in,
   input wire                 write_en,
   input wire                 clk,
   input wire                 reset,
    // output
   output logic [WIDTH - 1:0] out,
   output logic               done
);

  always_ff @(posedge clk) begin
    if (reset) begin
       out <= 0;
       done <= 0;
    end else if (write_en) begin
      out <= in;
      done <= 1'd1;
    end else done <= 1'd0;
  end
endmodule

module std_mem_d1 #(
    parameter WIDTH = 32,
    parameter SIZE = 16,
    parameter IDX_SIZE = 4
) (
   input wire                logic [IDX_SIZE-1:0] addr0,
   input wire                logic [ WIDTH-1:0] write_data,
   input wire                logic write_en,
   input wire                logic clk,
   output logic [ WIDTH-1:0] read_data,
   output logic              done
);

  logic [WIDTH-1:0] mem[SIZE-1:0];

  /* verilator lint_off WIDTH */
  assign read_data = mem[addr0];
  always_ff @(posedge clk) begin
    if (write_en) begin
      mem[addr0] <= write_data;
      done <= 1'd1;
    end else done <= 1'd0;
  end

  // Check for out of bounds access
  `ifdef VERILATOR
    always_comb begin
      if (addr0 >= SIZE)
        $error(
          "std_mem_d1: Out of bounds access\n",
          "addr0: %0d\n", addr0,
          "SIZE: %0d", SIZE
        );
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
   input wire                logic [D0_IDX_SIZE-1:0] addr0,
   input wire                logic [D1_IDX_SIZE-1:0] addr1,
   input wire                logic [ WIDTH-1:0] write_data,
   input wire                logic write_en,
   input wire                logic clk,
   output logic [ WIDTH-1:0] read_data,
   output logic              done
);

  /* verilator lint_off WIDTH */
  logic [WIDTH-1:0] mem[D0_SIZE-1:0][D1_SIZE-1:0];

  assign read_data = mem[addr0][addr1];
  always_ff @(posedge clk) begin
    if (write_en) begin
      mem[addr0][addr1] <= write_data;
      done <= 1'd1;
    end else done <= 1'd0;
  end

  // Check for out of bounds access
  `ifdef VERILATOR
    always_comb begin
      if (addr0 >= D0_SIZE)
        $error(
          "std_mem_d2: Out of bounds access\n",
          "addr0: %0d\n", addr0,
          "D0_SIZE: %0d", D0_SIZE
        );
      if (addr1 >= D1_SIZE)
        $error(
          "std_mem_d2: Out of bounds access\n",
          "addr1: %0d\n", addr1,
          "D1_SIZE: %0d", D1_SIZE
        );
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
   input wire                logic [D0_IDX_SIZE-1:0] addr0,
   input wire                logic [D1_IDX_SIZE-1:0] addr1,
   input wire                logic [D2_IDX_SIZE-1:0] addr2,
   input wire                logic [ WIDTH-1:0] write_data,
   input wire                logic write_en,
   input wire                logic clk,
   output logic [ WIDTH-1:0] read_data,
   output logic              done
);

  /* verilator lint_off WIDTH */
  logic [WIDTH-1:0] mem[D0_SIZE-1:0][D1_SIZE-1:0][D2_SIZE-1:0];

  assign read_data = mem[addr0][addr1][addr2];
  always_ff @(posedge clk) begin
    if (write_en) begin
      mem[addr0][addr1][addr2] <= write_data;
      done <= 1'd1;
    end else done <= 1'd0;
  end

  // Check for out of bounds access
  `ifdef VERILATOR
    always_comb begin
      if (addr0 >= D0_SIZE)
        $error(
          "std_mem_d3: Out of bounds access\n",
          "addr0: %0d\n", addr0,
          "D0_SIZE: %0d", D0_SIZE
        );
      if (addr1 >= D1_SIZE)
        $error(
          "std_mem_d3: Out of bounds access\n",
          "addr1: %0d\n", addr1,
          "D1_SIZE: %0d", D1_SIZE
        );
      if (addr2 >= D2_SIZE)
        $error(
          "std_mem_d3: Out of bounds access\n",
          "addr2: %0d\n", addr2,
          "D2_SIZE: %0d", D2_SIZE
        );
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
   input wire                logic [D0_IDX_SIZE-1:0] addr0,
   input wire                logic [D1_IDX_SIZE-1:0] addr1,
   input wire                logic [D2_IDX_SIZE-1:0] addr2,
   input wire                logic [D3_IDX_SIZE-1:0] addr3,
   input wire                logic [ WIDTH-1:0] write_data,
   input wire                logic write_en,
   input wire                logic clk,
   output logic [ WIDTH-1:0] read_data,
   output logic              done
);

  /* verilator lint_off WIDTH */
  logic [WIDTH-1:0] mem[D0_SIZE-1:0][D1_SIZE-1:0][D2_SIZE-1:0][D3_SIZE-1:0];

  assign read_data = mem[addr0][addr1][addr2][addr3];
  always_ff @(posedge clk) begin
    if (write_en) begin
      mem[addr0][addr1][addr2][addr3] <= write_data;
      done <= 1'd1;
    end else done <= 1'd0;
  end

  // Check for out of bounds access
  `ifdef VERILATOR
    always_comb begin
      if (addr0 >= D0_SIZE)
        $error(
          "std_mem_d4: Out of bounds access\n",
          "addr0: %0d\n", addr0,
          "D0_SIZE: %0d", D0_SIZE
        );
      if (addr1 >= D1_SIZE)
        $error(
          "std_mem_d4: Out of bounds access\n",
          "addr1: %0d\n", addr1,
          "D1_SIZE: %0d", D1_SIZE
        );
      if (addr2 >= D2_SIZE)
        $error(
          "std_mem_d4: Out of bounds access\n",
          "addr2: %0d\n", addr2,
          "D2_SIZE: %0d", D2_SIZE
        );
      if (addr3 >= D3_SIZE)
        $error(
          "std_mem_d4: Out of bounds access\n",
          "addr3: %0d\n", addr3,
          "D3_SIZE: %0d", D3_SIZE
        );
    end
  `endif
endmodule

`default_nettype wire

module main(
  input logic go,
  input logic clk,
  input logic reset,
  output logic done,
  output logic [3:0] A0_addr0,
  output logic [31:0] A0_write_data,
  output logic A0_write_en,
  output logic A0_clk,
  input logic [31:0] A0_read_data,
  input logic A0_done,
  output logic [3:0] B0_addr0,
  output logic [31:0] B0_write_data,
  output logic B0_write_en,
  output logic B0_clk,
  input logic [31:0] B0_read_data,
  input logic B0_done,
  output logic v0_addr0,
  output logic [31:0] v0_write_data,
  output logic v0_write_en,
  output logic v0_clk,
  input logic [31:0] v0_read_data,
  input logic v0_done
);
// COMPONENT START: main
logic [31:0] A_read0_0_in;
logic A_read0_0_write_en;
logic A_read0_0_clk;
logic A_read0_0_reset;
logic [31:0] A_read0_0_out;
logic A_read0_0_done;
logic [31:0] B_read0_0_in;
logic B_read0_0_write_en;
logic B_read0_0_clk;
logic B_read0_0_reset;
logic [31:0] B_read0_0_out;
logic B_read0_0_done;
logic [31:0] add0_left;
logic [31:0] add0_right;
logic [31:0] add0_out;
logic [3:0] add1_left;
logic [3:0] add1_right;
logic [3:0] add1_out;
logic [3:0] const0_out;
logic [3:0] const1_out;
logic const2_out;
logic [3:0] const3_out;
logic [3:0] i0_in;
logic i0_write_en;
logic i0_clk;
logic i0_reset;
logic [3:0] i0_out;
logic i0_done;
logic [3:0] le0_left;
logic [3:0] le0_right;
logic le0_out;
logic mult_pipe0_clk;
logic mult_pipe0_reset;
logic mult_pipe0_go;
logic [31:0] mult_pipe0_left;
logic [31:0] mult_pipe0_right;
logic [31:0] mult_pipe0_out;
logic mult_pipe0_done;
logic comb_reg_in;
logic comb_reg_write_en;
logic comb_reg_clk;
logic comb_reg_reset;
logic comb_reg_out;
logic comb_reg_done;
logic [3:0] comb_reg0_in;
logic comb_reg0_write_en;
logic comb_reg0_clk;
logic comb_reg0_reset;
logic [3:0] comb_reg0_out;
logic comb_reg0_done;
logic [3:0] fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic [3:0] fsm_out;
logic fsm_done;
logic upd2_go_in;
logic upd2_go_out;
logic upd2_done_in;
logic upd2_done_out;
logic cond00_go_in;
logic cond00_go_out;
logic cond00_done_in;
logic cond00_done_out;
logic comb_invoke0_go_in;
logic comb_invoke0_go_out;
logic comb_invoke0_done_in;
logic comb_invoke0_done_out;
logic invoke0_go_in;
logic invoke0_go_out;
logic invoke0_done_in;
logic invoke0_done_out;
logic invoke1_go_in;
logic invoke1_go_out;
logic invoke1_done_in;
logic invoke1_done_out;
logic invoke2_go_in;
logic invoke2_go_out;
logic invoke2_done_in;
logic invoke2_done_out;
logic invoke3_go_in;
logic invoke3_go_out;
logic invoke3_done_in;
logic invoke3_done_out;
logic invoke4_go_in;
logic invoke4_go_out;
logic invoke4_done_in;
logic invoke4_done_out;
logic msp_go_in;
logic msp_go_out;
logic msp_done_in;
logic msp_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
std_reg # (
    .WIDTH(32)
) A_read0_0 (
    .clk(A_read0_0_clk),
    .done(A_read0_0_done),
    .in(A_read0_0_in),
    .out(A_read0_0_out),
    .reset(A_read0_0_reset),
    .write_en(A_read0_0_write_en)
);
std_reg # (
    .WIDTH(32)
) B_read0_0 (
    .clk(B_read0_0_clk),
    .done(B_read0_0_done),
    .in(B_read0_0_in),
    .out(B_read0_0_out),
    .reset(B_read0_0_reset),
    .write_en(B_read0_0_write_en)
);
std_add # (
    .WIDTH(32)
) add0 (
    .left(add0_left),
    .out(add0_out),
    .right(add0_right)
);
std_add # (
    .WIDTH(4)
) add1 (
    .left(add1_left),
    .out(add1_out),
    .right(add1_right)
);
std_const # (
    .VALUE(4'd0),
    .WIDTH(4)
) const0 (
    .out(const0_out)
);
std_const # (
    .VALUE(4'd7),
    .WIDTH(4)
) const1 (
    .out(const1_out)
);
std_const # (
    .VALUE(1'd0),
    .WIDTH(1)
) const2 (
    .out(const2_out)
);
std_const # (
    .VALUE(4'd1),
    .WIDTH(4)
) const3 (
    .out(const3_out)
);
std_reg # (
    .WIDTH(4)
) i0 (
    .clk(i0_clk),
    .done(i0_done),
    .in(i0_in),
    .out(i0_out),
    .reset(i0_reset),
    .write_en(i0_write_en)
);
std_le # (
    .WIDTH(4)
) le0 (
    .left(le0_left),
    .out(le0_out),
    .right(le0_right)
);
std_mult_pipe # (
    .WIDTH(32)
) mult_pipe0 (
    .clk(mult_pipe0_clk),
    .done(mult_pipe0_done),
    .go(mult_pipe0_go),
    .left(mult_pipe0_left),
    .out(mult_pipe0_out),
    .reset(mult_pipe0_reset),
    .right(mult_pipe0_right)
);
std_reg # (
    .WIDTH(1)
) comb_reg (
    .clk(comb_reg_clk),
    .done(comb_reg_done),
    .in(comb_reg_in),
    .out(comb_reg_out),
    .reset(comb_reg_reset),
    .write_en(comb_reg_write_en)
);
std_reg # (
    .WIDTH(4)
) comb_reg0 (
    .clk(comb_reg0_clk),
    .done(comb_reg0_done),
    .in(comb_reg0_in),
    .out(comb_reg0_out),
    .reset(comb_reg0_reset),
    .write_en(comb_reg0_write_en)
);
std_reg # (
    .WIDTH(4)
) fsm (
    .clk(fsm_clk),
    .done(fsm_done),
    .in(fsm_in),
    .out(fsm_out),
    .reset(fsm_reset),
    .write_en(fsm_write_en)
);
std_wire # (
    .WIDTH(1)
) upd2_go (
    .in(upd2_go_in),
    .out(upd2_go_out)
);
std_wire # (
    .WIDTH(1)
) upd2_done (
    .in(upd2_done_in),
    .out(upd2_done_out)
);
std_wire # (
    .WIDTH(1)
) cond00_go (
    .in(cond00_go_in),
    .out(cond00_go_out)
);
std_wire # (
    .WIDTH(1)
) cond00_done (
    .in(cond00_done_in),
    .out(cond00_done_out)
);
std_wire # (
    .WIDTH(1)
) comb_invoke0_go (
    .in(comb_invoke0_go_in),
    .out(comb_invoke0_go_out)
);
std_wire # (
    .WIDTH(1)
) comb_invoke0_done (
    .in(comb_invoke0_done_in),
    .out(comb_invoke0_done_out)
);
std_wire # (
    .WIDTH(1)
) invoke0_go (
    .in(invoke0_go_in),
    .out(invoke0_go_out)
);
std_wire # (
    .WIDTH(1)
) invoke0_done (
    .in(invoke0_done_in),
    .out(invoke0_done_out)
);
std_wire # (
    .WIDTH(1)
) invoke1_go (
    .in(invoke1_go_in),
    .out(invoke1_go_out)
);
std_wire # (
    .WIDTH(1)
) invoke1_done (
    .in(invoke1_done_in),
    .out(invoke1_done_out)
);
std_wire # (
    .WIDTH(1)
) invoke2_go (
    .in(invoke2_go_in),
    .out(invoke2_go_out)
);
std_wire # (
    .WIDTH(1)
) invoke2_done (
    .in(invoke2_done_in),
    .out(invoke2_done_out)
);
std_wire # (
    .WIDTH(1)
) invoke3_go (
    .in(invoke3_go_in),
    .out(invoke3_go_out)
);
std_wire # (
    .WIDTH(1)
) invoke3_done (
    .in(invoke3_done_in),
    .out(invoke3_done_out)
);
std_wire # (
    .WIDTH(1)
) invoke4_go (
    .in(invoke4_go_in),
    .out(invoke4_go_out)
);
std_wire # (
    .WIDTH(1)
) invoke4_done (
    .in(invoke4_done_in),
    .out(invoke4_done_out)
);
std_wire # (
    .WIDTH(1)
) msp_go (
    .in(msp_go_in),
    .out(msp_go_out)
);
std_wire # (
    .WIDTH(1)
) msp_done (
    .in(msp_done_in),
    .out(msp_done_out)
);
std_wire # (
    .WIDTH(1)
) tdcc_go (
    .in(tdcc_go_in),
    .out(tdcc_go_out)
);
std_wire # (
    .WIDTH(1)
) tdcc_done (
    .in(tdcc_done_in),
    .out(tdcc_done_out)
);
assign A_read0_0_in =
 msp_go_out ? A0_read_data :
 invoke2_go_out ? mult_pipe0_out : 32'd0;
assign A_read0_0_write_en = invoke2_go_out | msp_go_out;
assign A_read0_0_clk = clk;
assign A_read0_0_reset = reset;
assign B_read0_0_in =
 invoke3_go_out ? A_read0_0_out :
 msp_go_out ? B0_read_data : 32'd0;
assign B_read0_0_write_en = invoke3_go_out | msp_go_out;
assign B_read0_0_clk = clk;
assign B_read0_0_reset = reset;
assign add0_left =
 upd2_go_out ? v0_read_data : 32'd0;
assign add0_right =
 upd2_go_out ? B_read0_0_out : 32'd0;
assign add1_left =
 comb_invoke0_go_out ? i0_out : 4'd0;
assign add1_right =
 comb_invoke0_go_out ? const3_out : 4'd0;
assign i0_in =
 invoke0_go_out ? const0_out :
 invoke4_go_out ? comb_reg0_out : 4'd0;
assign i0_write_en = invoke0_go_out | invoke4_go_out;
assign i0_clk = clk;
assign i0_reset = reset;
assign le0_left =
 cond00_go_out ? i0_out : 4'd0;
assign le0_right =
 cond00_go_out ? const1_out : 4'd0;
assign mult_pipe0_left =
 invoke1_go_out ? A_read0_0_out : 32'd0;
assign mult_pipe0_right =
 invoke1_go_out ? B_read0_0_out : 32'd0;
assign mult_pipe0_go = invoke1_go_out;
assign mult_pipe0_clk = clk;
assign mult_pipe0_reset = reset;
assign done = tdcc_done_out;
assign A0_addr0 =
 msp_go_out ? i0_out : 4'd0;
assign A0_clk = clk;
assign B0_addr0 =
 msp_go_out ? i0_out : 4'd0;
assign B0_clk = clk;
assign v0_addr0 =
 upd2_go_out ? const2_out : 1'd0;
assign v0_write_data =
 upd2_go_out ? add0_out : 32'd0;
assign v0_write_en = upd2_go_out;
assign v0_clk = clk;
assign comb_reg_in =
 cond00_go_out ? le0_out : 1'd0;
assign comb_reg_write_en = cond00_go_out;
assign comb_reg_clk = clk;
assign comb_reg_reset = reset;
assign comb_reg0_in =
 comb_invoke0_go_out ? add1_out : 4'd0;
assign comb_reg0_write_en = comb_invoke0_go_out;
assign comb_reg0_clk = clk;
assign comb_reg0_reset = reset;
assign fsm_in =
 fsm_out == 4'd1 & cond00_done_out & ~comb_reg_out & tdcc_go_out | fsm_out == 4'd9 & cond00_done_out & ~comb_reg_out & tdcc_go_out ? 4'd10 :
 fsm_out == 4'd10 ? 4'd0 :
 fsm_out == 4'd0 & invoke0_done_out & tdcc_go_out ? 4'd1 :
 fsm_out == 4'd1 & cond00_done_out & comb_reg_out & tdcc_go_out | fsm_out == 4'd9 & cond00_done_out & comb_reg_out & tdcc_go_out ? 4'd2 :
 fsm_out == 4'd2 & msp_done_out & tdcc_go_out ? 4'd3 :
 fsm_out == 4'd3 & invoke1_done_out & tdcc_go_out ? 4'd4 :
 fsm_out == 4'd4 & invoke2_done_out & tdcc_go_out ? 4'd5 :
 fsm_out == 4'd5 & invoke3_done_out & tdcc_go_out ? 4'd6 :
 fsm_out == 4'd6 & upd2_done_out & tdcc_go_out ? 4'd7 :
 fsm_out == 4'd7 & comb_invoke0_done_out & tdcc_go_out ? 4'd8 :
 fsm_out == 4'd8 & invoke4_done_out & tdcc_go_out ? 4'd9 : 4'd0;
assign fsm_write_en = fsm_out == 4'd10 | fsm_out == 4'd0 & invoke0_done_out & tdcc_go_out | fsm_out == 4'd1 & cond00_done_out & comb_reg_out & tdcc_go_out | fsm_out == 4'd9 & cond00_done_out & comb_reg_out & tdcc_go_out | fsm_out == 4'd2 & msp_done_out & tdcc_go_out | fsm_out == 4'd3 & invoke1_done_out & tdcc_go_out | fsm_out == 4'd4 & invoke2_done_out & tdcc_go_out | fsm_out == 4'd5 & invoke3_done_out & tdcc_go_out | fsm_out == 4'd6 & upd2_done_out & tdcc_go_out | fsm_out == 4'd7 & comb_invoke0_done_out & tdcc_go_out | fsm_out == 4'd8 & invoke4_done_out & tdcc_go_out | fsm_out == 4'd1 & cond00_done_out & ~comb_reg_out & tdcc_go_out | fsm_out == 4'd9 & cond00_done_out & ~comb_reg_out & tdcc_go_out;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign upd2_go_in = ~upd2_done_out & fsm_out == 4'd6 & tdcc_go_out;
assign upd2_done_in = v0_done;
assign cond00_go_in = ~cond00_done_out & fsm_out == 4'd1 & tdcc_go_out | ~cond00_done_out & fsm_out == 4'd9 & tdcc_go_out;
assign cond00_done_in = comb_reg_done;
assign comb_invoke0_go_in = ~comb_invoke0_done_out & fsm_out == 4'd7 & tdcc_go_out;
assign comb_invoke0_done_in = comb_reg0_done;
assign invoke0_go_in = ~invoke0_done_out & fsm_out == 4'd0 & tdcc_go_out;
assign invoke0_done_in = i0_done;
assign invoke1_go_in = ~invoke1_done_out & fsm_out == 4'd3 & tdcc_go_out;
assign invoke1_done_in = mult_pipe0_done;
assign invoke2_go_in = ~invoke2_done_out & fsm_out == 4'd4 & tdcc_go_out;
assign invoke2_done_in = A_read0_0_done;
assign invoke3_go_in = ~invoke3_done_out & fsm_out == 4'd5 & tdcc_go_out;
assign invoke3_done_in = B_read0_0_done;
assign invoke4_go_in = ~invoke4_done_out & fsm_out == 4'd8 & tdcc_go_out;
assign invoke4_done_in = i0_done;
assign msp_go_in = ~msp_done_out & fsm_out == 4'd2 & tdcc_go_out;
assign msp_done_in = A_read0_0_done & B_read0_0_done;
assign tdcc_go_in = go;
assign tdcc_done_in = fsm_out == 4'd10;
// COMPONENT END: main
endmodule
