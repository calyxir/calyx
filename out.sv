t0_move0
static_group: comb_invoke1
static_group: comb_invoke00
static_group: invoke00
static_group: invoke100
static_group: invoke20
static_group: invoke30
static_group: t0_move0
static_group: l0_move0
static_group: invoke40
static_group: invoke50
static_group: invoke60
invoke5: invoke50
invoke0: invoke00
t0_move: t0_move0
invoke3: invoke30
invoke6: invoke60
invoke2: invoke20
l0_move: l0_move0
invoke4: invoke40
invoke1: invoke100
l0_move0
static_group: comb_invoke1
static_group: comb_invoke00
static_group: invoke00
static_group: invoke100
static_group: invoke20
static_group: invoke30
static_group: t0_move0
static_group: l0_move0
static_group: invoke40
static_group: invoke50
static_group: invoke60
invoke5: invoke50
invoke0: invoke00
t0_move: t0_move0
invoke3: invoke30
invoke6: invoke60
invoke2: invoke20
l0_move: l0_move0
invoke4: invoke40
invoke1: invoke100
t0_move0
static_group: comb_invoke1
static_group: comb_invoke00
static_group: invoke00
static_group: invoke100
static_group: invoke20
static_group: invoke30
static_group: t0_move0
static_group: l0_move0
static_group: invoke40
static_group: invoke50
static_group: invoke60
static_group: invoke70
static_group: invoke80
static_group: invoke90
invoke5: invoke50
invoke0: invoke00
t0_move: t0_move0
invoke7: invoke70
invoke3: invoke30
invoke6: invoke60
invoke2: invoke20
l0_move: l0_move0
invoke4: invoke40
invoke1: invoke100
invoke8: invoke80
invoke9: invoke90
l0_move0
static_group: comb_invoke1
static_group: comb_invoke00
static_group: invoke00
static_group: invoke100
static_group: invoke20
static_group: invoke30
static_group: t0_move0
static_group: l0_move0
static_group: invoke40
static_group: invoke50
static_group: invoke60
static_group: invoke70
static_group: invoke80
static_group: invoke90
invoke5: invoke50
invoke0: invoke00
t0_move: t0_move0
invoke7: invoke70
invoke3: invoke30
invoke6: invoke60
invoke2: invoke20
l0_move: l0_move0
invoke4: invoke40
invoke1: invoke100
invoke8: invoke80
invoke9: invoke90
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

module std_mem_d1 #(
    parameter WIDTH = 32,
    parameter SIZE = 16,
    parameter IDX_SIZE = 4
) (
   input wire                logic [IDX_SIZE-1:0] addr0,
   input wire                logic [ WIDTH-1:0] write_data,
   input wire                logic write_en,
   input wire                logic clk,
   input wire                logic reset,
   output logic [ WIDTH-1:0] read_data,
   output logic              done
);

  logic [WIDTH-1:0] mem[SIZE-1:0];

  /* verilator lint_off WIDTH */
  assign read_data = mem[addr0];

  always_ff @(posedge clk) begin
    if (reset)
      done <= '0;
    else if (write_en)
      done <= '1;
    else
      done <= '0;
  end

  always_ff @(posedge clk) begin
    if (!reset && write_en)
      mem[addr0] <= write_data;
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
   input wire                logic reset,
   output logic [ WIDTH-1:0] read_data,
   output logic              done
);

  /* verilator lint_off WIDTH */
  logic [WIDTH-1:0] mem[D0_SIZE-1:0][D1_SIZE-1:0];

  assign read_data = mem[addr0][addr1];

  always_ff @(posedge clk) begin
    if (reset)
      done <= '0;
    else if (write_en)
      done <= '1;
    else
      done <= '0;
  end

  always_ff @(posedge clk) begin
    if (!reset && write_en)
      mem[addr0][addr1] <= write_data;
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
   input wire                logic reset,
   output logic [ WIDTH-1:0] read_data,
   output logic              done
);

  /* verilator lint_off WIDTH */
  logic [WIDTH-1:0] mem[D0_SIZE-1:0][D1_SIZE-1:0][D2_SIZE-1:0];

  assign read_data = mem[addr0][addr1][addr2];

  always_ff @(posedge clk) begin
    if (reset)
      done <= '0;
    else if (write_en)
      done <= '1;
    else
      done <= '0;
  end

  always_ff @(posedge clk) begin
    if (!reset && write_en)
      mem[addr0][addr1][addr2] <= write_data;
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
   input wire                logic reset,
   output logic [ WIDTH-1:0] read_data,
   output logic              done
);

  /* verilator lint_off WIDTH */
  logic [WIDTH-1:0] mem[D0_SIZE-1:0][D1_SIZE-1:0][D2_SIZE-1:0][D3_SIZE-1:0];

  assign read_data = mem[addr0][addr1][addr2][addr3];

  always_ff @(posedge clk) begin
    if (reset)
      done <= '0;
    else if (write_en)
      done <= '1;
    else
      done <= '0;
  end

  always_ff @(posedge clk) begin
    if (!reset && write_en)
      mem[addr0][addr1][addr2][addr3] <= write_data;
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

module undef #(
    parameter WIDTH = 32
) (
   output logic [WIDTH-1:0] out
);

    assign out = 'x;
endmodule

module std_const #(
    parameter WIDTH = 32,
    parameter VALUE = 32
) (
   output logic [WIDTH-1:0] out
);

    assign out = VALUE;
endmodule

module std_wire #(
    parameter WIDTH = 32
) (
   input logic [WIDTH-1:0] in,
   output logic [WIDTH-1:0] out
);

    assign out = in;
endmodule

module std_add #(
    parameter WIDTH = 32
) (
   input logic [WIDTH-1:0] left,
   input logic [WIDTH-1:0] right,
   output logic [WIDTH-1:0] out
);

  assign out = left + right;
endmodule

module std_reg #(
    parameter WIDTH = 32
) (
   input logic [WIDTH-1:0] in,
   input logic write_en,
   input logic clk,
   input logic reset,
   output logic [WIDTH-1:0] out,
   output logic done
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

module mac_pe(
  input logic [31:0] top,
  input logic [31:0] left,
  output logic [31:0] out,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: mac_pe
logic [31:0] acc_in;
logic acc_write_en;
logic acc_clk;
logic acc_reset;
logic [31:0] acc_out;
logic acc_done;
logic [31:0] add_left;
logic [31:0] add_right;
logic [31:0] add_out;
logic mul_clk;
logic mul_reset;
logic mul_go;
logic [31:0] mul_left;
logic [31:0] mul_right;
logic [31:0] mul_out;
logic mul_done;
logic [2:0] fsm1_in;
logic fsm1_write_en;
logic fsm1_clk;
logic fsm1_reset;
logic [2:0] fsm1_out;
logic fsm1_done;
logic ud1_out;
logic [2:0] adder1_left;
logic [2:0] adder1_right;
logic [2:0] adder1_out;
logic signal_reg_in;
logic signal_reg_write_en;
logic signal_reg_clk;
logic signal_reg_reset;
logic signal_reg_out;
logic signal_reg_done;
logic early_reset_static_seq_go_in;
logic early_reset_static_seq_go_out;
logic early_reset_static_seq_done_in;
logic early_reset_static_seq_done_out;
logic wrapper_early_reset_static_seq_go_in;
logic wrapper_early_reset_static_seq_go_out;
logic wrapper_early_reset_static_seq_done_in;
logic wrapper_early_reset_static_seq_done_out;
std_reg # (
    .WIDTH(32)
) acc (
    .clk(acc_clk),
    .done(acc_done),
    .in(acc_in),
    .out(acc_out),
    .reset(acc_reset),
    .write_en(acc_write_en)
);
std_add # (
    .WIDTH(32)
) add (
    .left(add_left),
    .out(add_out),
    .right(add_right)
);
std_mult_pipe # (
    .WIDTH(32)
) mul (
    .clk(mul_clk),
    .done(mul_done),
    .go(mul_go),
    .left(mul_left),
    .out(mul_out),
    .reset(mul_reset),
    .right(mul_right)
);
std_reg # (
    .WIDTH(3)
) fsm1 (
    .clk(fsm1_clk),
    .done(fsm1_done),
    .in(fsm1_in),
    .out(fsm1_out),
    .reset(fsm1_reset),
    .write_en(fsm1_write_en)
);
undef # (
    .WIDTH(1)
) ud1 (
    .out(ud1_out)
);
std_add # (
    .WIDTH(3)
) adder1 (
    .left(adder1_left),
    .out(adder1_out),
    .right(adder1_right)
);
std_reg # (
    .WIDTH(1)
) signal_reg (
    .clk(signal_reg_clk),
    .done(signal_reg_done),
    .in(signal_reg_in),
    .out(signal_reg_out),
    .reset(signal_reg_reset),
    .write_en(signal_reg_write_en)
);
std_wire # (
    .WIDTH(1)
) early_reset_static_seq_go (
    .in(early_reset_static_seq_go_in),
    .out(early_reset_static_seq_go_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_static_seq_done (
    .in(early_reset_static_seq_done_in),
    .out(early_reset_static_seq_done_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_static_seq_go (
    .in(wrapper_early_reset_static_seq_go_in),
    .out(wrapper_early_reset_static_seq_go_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_static_seq_done (
    .in(wrapper_early_reset_static_seq_done_in),
    .out(wrapper_early_reset_static_seq_done_out)
);
wire _guard0 = 1;
wire _guard1 = early_reset_static_seq_go_out;
wire _guard2 = early_reset_static_seq_go_out;
wire _guard3 = fsm1_out == 3'd3;
wire _guard4 = early_reset_static_seq_go_out;
wire _guard5 = _guard3 & _guard4;
wire _guard6 = fsm1_out == 3'd3;
wire _guard7 = early_reset_static_seq_go_out;
wire _guard8 = _guard6 & _guard7;
wire _guard9 = wrapper_early_reset_static_seq_done_out;
wire _guard10 = early_reset_static_seq_go_out;
wire _guard11 = fsm1_out != 3'd3;
wire _guard12 = early_reset_static_seq_go_out;
wire _guard13 = _guard11 & _guard12;
wire _guard14 = fsm1_out == 3'd3;
wire _guard15 = early_reset_static_seq_go_out;
wire _guard16 = _guard14 & _guard15;
wire _guard17 = fsm1_out == 3'd0;
wire _guard18 = signal_reg_out;
wire _guard19 = _guard17 & _guard18;
wire _guard20 = wrapper_early_reset_static_seq_go_out;
wire _guard21 = fsm1_out == 3'd0;
wire _guard22 = signal_reg_out;
wire _guard23 = _guard21 & _guard22;
wire _guard24 = fsm1_out == 3'd0;
wire _guard25 = signal_reg_out;
wire _guard26 = ~_guard25;
wire _guard27 = _guard24 & _guard26;
wire _guard28 = wrapper_early_reset_static_seq_go_out;
wire _guard29 = _guard27 & _guard28;
wire _guard30 = _guard23 | _guard29;
wire _guard31 = fsm1_out == 3'd0;
wire _guard32 = signal_reg_out;
wire _guard33 = ~_guard32;
wire _guard34 = _guard31 & _guard33;
wire _guard35 = wrapper_early_reset_static_seq_go_out;
wire _guard36 = _guard34 & _guard35;
wire _guard37 = fsm1_out == 3'd0;
wire _guard38 = signal_reg_out;
wire _guard39 = _guard37 & _guard38;
wire _guard40 = fsm1_out == 3'd3;
wire _guard41 = early_reset_static_seq_go_out;
wire _guard42 = _guard40 & _guard41;
wire _guard43 = fsm1_out == 3'd3;
wire _guard44 = early_reset_static_seq_go_out;
wire _guard45 = _guard43 & _guard44;
wire _guard46 = fsm1_out < 3'd3;
wire _guard47 = early_reset_static_seq_go_out;
wire _guard48 = _guard46 & _guard47;
wire _guard49 = fsm1_out < 3'd3;
wire _guard50 = early_reset_static_seq_go_out;
wire _guard51 = _guard49 & _guard50;
wire _guard52 = fsm1_out < 3'd3;
wire _guard53 = early_reset_static_seq_go_out;
wire _guard54 = _guard52 & _guard53;
assign adder1_left =
  _guard1 ? fsm1_out :
  3'd0;
assign adder1_right =
  _guard2 ? 3'd1 :
  3'd0;
assign acc_write_en = _guard5;
assign acc_clk = clk;
assign acc_reset = reset;
assign acc_in = add_out;
assign done = _guard9;
assign out = acc_out;
assign fsm1_write_en = _guard10;
assign fsm1_clk = clk;
assign fsm1_reset = reset;
assign fsm1_in =
  _guard13 ? adder1_out :
  _guard16 ? 3'd0 :
  3'd0;
always_comb begin
  if(~$onehot0({_guard16, _guard13})) begin
    $fatal(2, "Multiple assignment to port `fsm1.in'.");
end
end
assign wrapper_early_reset_static_seq_done_in = _guard19;
assign early_reset_static_seq_go_in = _guard20;
assign signal_reg_write_en = _guard30;
assign signal_reg_clk = clk;
assign signal_reg_reset = reset;
assign signal_reg_in =
  _guard36 ? 1'd1 :
  _guard39 ? 1'd0 :
  1'd0;
always_comb begin
  if(~$onehot0({_guard39, _guard36})) begin
    $fatal(2, "Multiple assignment to port `signal_reg.in'.");
end
end
assign add_left = acc_out;
assign add_right = mul_out;
assign early_reset_static_seq_done_in = ud1_out;
assign mul_clk = clk;
assign mul_left = top;
assign mul_reset = reset;
assign mul_go = _guard51;
assign mul_right = left;
assign wrapper_early_reset_static_seq_go_in = go;
// COMPONENT END: mac_pe
endmodule
module main(
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: main
string DATA;
int CODE;
initial begin
    CODE = $value$plusargs("DATA=%s", DATA);
    $display("DATA (path to meminit files): %s", DATA);
    $readmemh({DATA, "/t0.dat"}, t0.mem);
    $readmemh({DATA, "/l0.dat"}, l0.mem);
    $readmemh({DATA, "/out_mem.dat"}, out_mem.mem);
end
final begin
    $writememh({DATA, "/t0.out"}, t0.mem);
    $writememh({DATA, "/l0.out"}, l0.mem);
    $writememh({DATA, "/out_mem.out"}, out_mem.mem);
end
logic [31:0] pe_0_0_top;
logic [31:0] pe_0_0_left;
logic [31:0] pe_0_0_out;
logic pe_0_0_go;
logic pe_0_0_clk;
logic pe_0_0_reset;
logic pe_0_0_done;
logic [31:0] top_0_0_in;
logic top_0_0_write_en;
logic top_0_0_clk;
logic top_0_0_reset;
logic [31:0] top_0_0_out;
logic top_0_0_done;
logic [31:0] left_0_0_in;
logic left_0_0_write_en;
logic left_0_0_clk;
logic left_0_0_reset;
logic [31:0] left_0_0_out;
logic left_0_0_done;
logic [1:0] t0_addr0;
logic [31:0] t0_write_data;
logic t0_write_en;
logic t0_clk;
logic t0_reset;
logic [31:0] t0_read_data;
logic t0_done;
logic [1:0] t0_idx_in;
logic t0_idx_write_en;
logic t0_idx_clk;
logic t0_idx_reset;
logic [1:0] t0_idx_out;
logic t0_idx_done;
logic [1:0] t0_add_left;
logic [1:0] t0_add_right;
logic [1:0] t0_add_out;
logic [1:0] l0_addr0;
logic [31:0] l0_write_data;
logic l0_write_en;
logic l0_clk;
logic l0_reset;
logic [31:0] l0_read_data;
logic l0_done;
logic [1:0] l0_idx_in;
logic l0_idx_write_en;
logic l0_idx_clk;
logic l0_idx_reset;
logic [1:0] l0_idx_out;
logic l0_idx_done;
logic [1:0] l0_add_left;
logic [1:0] l0_add_right;
logic [1:0] l0_add_out;
logic out_mem_addr0;
logic [31:0] out_mem_write_data;
logic out_mem_write_en;
logic out_mem_clk;
logic out_mem_reset;
logic [31:0] out_mem_read_data;
logic out_mem_done;
logic [4:0] fsm12_in;
logic fsm12_write_en;
logic fsm12_clk;
logic fsm12_reset;
logic [4:0] fsm12_out;
logic fsm12_done;
logic ud12_out;
logic [4:0] adder12_left;
logic [4:0] adder12_right;
logic [4:0] adder12_out;
logic signal_reg_in;
logic signal_reg_write_en;
logic signal_reg_clk;
logic signal_reg_reset;
logic signal_reg_out;
logic signal_reg_done;
logic [1:0] fsm20_in;
logic fsm20_write_en;
logic fsm20_clk;
logic fsm20_reset;
logic [1:0] fsm20_out;
logic fsm20_done;
logic pe_0_0_out_write_go_in;
logic pe_0_0_out_write_go_out;
logic pe_0_0_out_write_done_in;
logic pe_0_0_out_write_done_out;
logic early_reset_static_seq_go_in;
logic early_reset_static_seq_go_out;
logic early_reset_static_seq_done_in;
logic early_reset_static_seq_done_out;
logic wrapper_early_reset_static_seq_go_in;
logic wrapper_early_reset_static_seq_go_out;
logic wrapper_early_reset_static_seq_done_in;
logic wrapper_early_reset_static_seq_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
mac_pe pe_0_0 (
    .clk(pe_0_0_clk),
    .done(pe_0_0_done),
    .go(pe_0_0_go),
    .left(pe_0_0_left),
    .out(pe_0_0_out),
    .reset(pe_0_0_reset),
    .top(pe_0_0_top)
);
std_reg # (
    .WIDTH(32)
) top_0_0 (
    .clk(top_0_0_clk),
    .done(top_0_0_done),
    .in(top_0_0_in),
    .out(top_0_0_out),
    .reset(top_0_0_reset),
    .write_en(top_0_0_write_en)
);
std_reg # (
    .WIDTH(32)
) left_0_0 (
    .clk(left_0_0_clk),
    .done(left_0_0_done),
    .in(left_0_0_in),
    .out(left_0_0_out),
    .reset(left_0_0_reset),
    .write_en(left_0_0_write_en)
);
std_mem_d1 # (
    .IDX_SIZE(2),
    .SIZE(3),
    .WIDTH(32)
) t0 (
    .addr0(t0_addr0),
    .clk(t0_clk),
    .done(t0_done),
    .read_data(t0_read_data),
    .reset(t0_reset),
    .write_data(t0_write_data),
    .write_en(t0_write_en)
);
std_reg # (
    .WIDTH(2)
) t0_idx (
    .clk(t0_idx_clk),
    .done(t0_idx_done),
    .in(t0_idx_in),
    .out(t0_idx_out),
    .reset(t0_idx_reset),
    .write_en(t0_idx_write_en)
);
std_add # (
    .WIDTH(2)
) t0_add (
    .left(t0_add_left),
    .out(t0_add_out),
    .right(t0_add_right)
);
std_mem_d1 # (
    .IDX_SIZE(2),
    .SIZE(3),
    .WIDTH(32)
) l0 (
    .addr0(l0_addr0),
    .clk(l0_clk),
    .done(l0_done),
    .read_data(l0_read_data),
    .reset(l0_reset),
    .write_data(l0_write_data),
    .write_en(l0_write_en)
);
std_reg # (
    .WIDTH(2)
) l0_idx (
    .clk(l0_idx_clk),
    .done(l0_idx_done),
    .in(l0_idx_in),
    .out(l0_idx_out),
    .reset(l0_idx_reset),
    .write_en(l0_idx_write_en)
);
std_add # (
    .WIDTH(2)
) l0_add (
    .left(l0_add_left),
    .out(l0_add_out),
    .right(l0_add_right)
);
std_mem_d1 # (
    .IDX_SIZE(1),
    .SIZE(1),
    .WIDTH(32)
) out_mem (
    .addr0(out_mem_addr0),
    .clk(out_mem_clk),
    .done(out_mem_done),
    .read_data(out_mem_read_data),
    .reset(out_mem_reset),
    .write_data(out_mem_write_data),
    .write_en(out_mem_write_en)
);
std_reg # (
    .WIDTH(5)
) fsm12 (
    .clk(fsm12_clk),
    .done(fsm12_done),
    .in(fsm12_in),
    .out(fsm12_out),
    .reset(fsm12_reset),
    .write_en(fsm12_write_en)
);
undef # (
    .WIDTH(1)
) ud12 (
    .out(ud12_out)
);
std_add # (
    .WIDTH(5)
) adder12 (
    .left(adder12_left),
    .out(adder12_out),
    .right(adder12_right)
);
std_reg # (
    .WIDTH(1)
) signal_reg (
    .clk(signal_reg_clk),
    .done(signal_reg_done),
    .in(signal_reg_in),
    .out(signal_reg_out),
    .reset(signal_reg_reset),
    .write_en(signal_reg_write_en)
);
std_reg # (
    .WIDTH(2)
) fsm20 (
    .clk(fsm20_clk),
    .done(fsm20_done),
    .in(fsm20_in),
    .out(fsm20_out),
    .reset(fsm20_reset),
    .write_en(fsm20_write_en)
);
std_wire # (
    .WIDTH(1)
) pe_0_0_out_write_go (
    .in(pe_0_0_out_write_go_in),
    .out(pe_0_0_out_write_go_out)
);
std_wire # (
    .WIDTH(1)
) pe_0_0_out_write_done (
    .in(pe_0_0_out_write_done_in),
    .out(pe_0_0_out_write_done_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_static_seq_go (
    .in(early_reset_static_seq_go_in),
    .out(early_reset_static_seq_go_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_static_seq_done (
    .in(early_reset_static_seq_done_in),
    .out(early_reset_static_seq_done_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_static_seq_go (
    .in(wrapper_early_reset_static_seq_go_in),
    .out(wrapper_early_reset_static_seq_go_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_static_seq_done (
    .in(wrapper_early_reset_static_seq_done_in),
    .out(wrapper_early_reset_static_seq_done_out)
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
wire _guard0 = 1;
wire _guard1 = pe_0_0_out_write_go_out;
wire _guard2 = pe_0_0_out_write_go_out;
wire _guard3 = pe_0_0_out_write_go_out;
wire _guard4 = tdcc_done_out;
wire _guard5 = early_reset_static_seq_go_out;
wire _guard6 = early_reset_static_seq_go_out;
wire _guard7 = fsm20_out == 2'd2;
wire _guard8 = fsm20_out == 2'd0;
wire _guard9 = wrapper_early_reset_static_seq_done_out;
wire _guard10 = _guard8 & _guard9;
wire _guard11 = tdcc_go_out;
wire _guard12 = _guard10 & _guard11;
wire _guard13 = _guard7 | _guard12;
wire _guard14 = fsm20_out == 2'd1;
wire _guard15 = pe_0_0_out_write_done_out;
wire _guard16 = _guard14 & _guard15;
wire _guard17 = tdcc_go_out;
wire _guard18 = _guard16 & _guard17;
wire _guard19 = _guard13 | _guard18;
wire _guard20 = fsm20_out == 2'd0;
wire _guard21 = wrapper_early_reset_static_seq_done_out;
wire _guard22 = _guard20 & _guard21;
wire _guard23 = tdcc_go_out;
wire _guard24 = _guard22 & _guard23;
wire _guard25 = fsm20_out == 2'd2;
wire _guard26 = fsm20_out == 2'd1;
wire _guard27 = pe_0_0_out_write_done_out;
wire _guard28 = _guard26 & _guard27;
wire _guard29 = tdcc_go_out;
wire _guard30 = _guard28 & _guard29;
wire _guard31 = pe_0_0_out_write_done_out;
wire _guard32 = ~_guard31;
wire _guard33 = fsm20_out == 2'd1;
wire _guard34 = _guard32 & _guard33;
wire _guard35 = tdcc_go_out;
wire _guard36 = _guard34 & _guard35;
wire _guard37 = fsm12_out == 5'd0;
wire _guard38 = fsm12_out == 5'd1;
wire _guard39 = _guard37 | _guard38;
wire _guard40 = fsm12_out == 5'd3;
wire _guard41 = fsm12_out >= 5'd3;
wire _guard42 = fsm12_out < 5'd7;
wire _guard43 = _guard41 & _guard42;
wire _guard44 = _guard40 & _guard43;
wire _guard45 = _guard39 | _guard44;
wire _guard46 = fsm12_out == 5'd8;
wire _guard47 = fsm12_out >= 5'd8;
wire _guard48 = fsm12_out < 5'd12;
wire _guard49 = _guard47 & _guard48;
wire _guard50 = _guard46 & _guard49;
wire _guard51 = _guard45 | _guard50;
wire _guard52 = early_reset_static_seq_go_out;
wire _guard53 = _guard51 & _guard52;
wire _guard54 = fsm12_out == 5'd0;
wire _guard55 = early_reset_static_seq_go_out;
wire _guard56 = _guard54 & _guard55;
wire _guard57 = fsm12_out == 5'd1;
wire _guard58 = fsm12_out == 5'd3;
wire _guard59 = fsm12_out >= 5'd3;
wire _guard60 = fsm12_out < 5'd7;
wire _guard61 = _guard59 & _guard60;
wire _guard62 = _guard58 & _guard61;
wire _guard63 = _guard57 | _guard62;
wire _guard64 = fsm12_out == 5'd8;
wire _guard65 = fsm12_out >= 5'd8;
wire _guard66 = fsm12_out < 5'd12;
wire _guard67 = _guard65 & _guard66;
wire _guard68 = _guard64 & _guard67;
wire _guard69 = _guard63 | _guard68;
wire _guard70 = early_reset_static_seq_go_out;
wire _guard71 = _guard69 & _guard70;
wire _guard72 = fsm12_out == 5'd2;
wire _guard73 = fsm12_out == 5'd7;
wire _guard74 = _guard72 | _guard73;
wire _guard75 = fsm12_out == 5'd12;
wire _guard76 = _guard74 | _guard75;
wire _guard77 = early_reset_static_seq_go_out;
wire _guard78 = _guard76 & _guard77;
wire _guard79 = fsm12_out == 5'd2;
wire _guard80 = fsm12_out == 5'd7;
wire _guard81 = _guard79 | _guard80;
wire _guard82 = fsm12_out == 5'd12;
wire _guard83 = _guard81 | _guard82;
wire _guard84 = early_reset_static_seq_go_out;
wire _guard85 = _guard83 & _guard84;
wire _guard86 = fsm12_out == 5'd2;
wire _guard87 = fsm12_out == 5'd7;
wire _guard88 = _guard86 | _guard87;
wire _guard89 = fsm12_out == 5'd12;
wire _guard90 = _guard88 | _guard89;
wire _guard91 = early_reset_static_seq_go_out;
wire _guard92 = _guard90 & _guard91;
wire _guard93 = early_reset_static_seq_go_out;
wire _guard94 = fsm12_out != 5'd16;
wire _guard95 = early_reset_static_seq_go_out;
wire _guard96 = _guard94 & _guard95;
wire _guard97 = fsm12_out == 5'd16;
wire _guard98 = early_reset_static_seq_go_out;
wire _guard99 = _guard97 & _guard98;
wire _guard100 = fsm12_out == 5'd0;
wire _guard101 = fsm12_out == 5'd1;
wire _guard102 = _guard100 | _guard101;
wire _guard103 = fsm12_out == 5'd3;
wire _guard104 = fsm12_out >= 5'd3;
wire _guard105 = fsm12_out < 5'd7;
wire _guard106 = _guard104 & _guard105;
wire _guard107 = _guard103 & _guard106;
wire _guard108 = _guard102 | _guard107;
wire _guard109 = fsm12_out == 5'd8;
wire _guard110 = fsm12_out >= 5'd8;
wire _guard111 = fsm12_out < 5'd12;
wire _guard112 = _guard110 & _guard111;
wire _guard113 = _guard109 & _guard112;
wire _guard114 = _guard108 | _guard113;
wire _guard115 = early_reset_static_seq_go_out;
wire _guard116 = _guard114 & _guard115;
wire _guard117 = fsm12_out == 5'd0;
wire _guard118 = early_reset_static_seq_go_out;
wire _guard119 = _guard117 & _guard118;
wire _guard120 = fsm12_out == 5'd1;
wire _guard121 = fsm12_out == 5'd3;
wire _guard122 = fsm12_out >= 5'd3;
wire _guard123 = fsm12_out < 5'd7;
wire _guard124 = _guard122 & _guard123;
wire _guard125 = _guard121 & _guard124;
wire _guard126 = _guard120 | _guard125;
wire _guard127 = fsm12_out == 5'd8;
wire _guard128 = fsm12_out >= 5'd8;
wire _guard129 = fsm12_out < 5'd12;
wire _guard130 = _guard128 & _guard129;
wire _guard131 = _guard127 & _guard130;
wire _guard132 = _guard126 | _guard131;
wire _guard133 = early_reset_static_seq_go_out;
wire _guard134 = _guard132 & _guard133;
wire _guard135 = fsm12_out == 5'd0;
wire _guard136 = signal_reg_out;
wire _guard137 = _guard135 & _guard136;
wire _guard138 = wrapper_early_reset_static_seq_go_out;
wire _guard139 = fsm12_out == 5'd2;
wire _guard140 = fsm12_out == 5'd7;
wire _guard141 = _guard139 | _guard140;
wire _guard142 = fsm12_out == 5'd12;
wire _guard143 = _guard141 | _guard142;
wire _guard144 = early_reset_static_seq_go_out;
wire _guard145 = _guard143 & _guard144;
wire _guard146 = fsm12_out == 5'd1;
wire _guard147 = fsm12_out == 5'd3;
wire _guard148 = fsm12_out >= 5'd3;
wire _guard149 = fsm12_out < 5'd7;
wire _guard150 = _guard148 & _guard149;
wire _guard151 = _guard147 & _guard150;
wire _guard152 = _guard146 | _guard151;
wire _guard153 = fsm12_out == 5'd8;
wire _guard154 = fsm12_out >= 5'd8;
wire _guard155 = fsm12_out < 5'd12;
wire _guard156 = _guard154 & _guard155;
wire _guard157 = _guard153 & _guard156;
wire _guard158 = _guard152 | _guard157;
wire _guard159 = early_reset_static_seq_go_out;
wire _guard160 = _guard158 & _guard159;
wire _guard161 = fsm12_out == 5'd1;
wire _guard162 = fsm12_out == 5'd3;
wire _guard163 = fsm12_out >= 5'd3;
wire _guard164 = fsm12_out < 5'd7;
wire _guard165 = _guard163 & _guard164;
wire _guard166 = _guard162 & _guard165;
wire _guard167 = _guard161 | _guard166;
wire _guard168 = fsm12_out == 5'd8;
wire _guard169 = fsm12_out >= 5'd8;
wire _guard170 = fsm12_out < 5'd12;
wire _guard171 = _guard169 & _guard170;
wire _guard172 = _guard168 & _guard171;
wire _guard173 = _guard167 | _guard172;
wire _guard174 = early_reset_static_seq_go_out;
wire _guard175 = _guard173 & _guard174;
wire _guard176 = fsm12_out == 5'd1;
wire _guard177 = fsm12_out == 5'd3;
wire _guard178 = fsm12_out >= 5'd3;
wire _guard179 = fsm12_out < 5'd7;
wire _guard180 = _guard178 & _guard179;
wire _guard181 = _guard177 & _guard180;
wire _guard182 = _guard176 | _guard181;
wire _guard183 = fsm12_out == 5'd8;
wire _guard184 = fsm12_out >= 5'd8;
wire _guard185 = fsm12_out < 5'd12;
wire _guard186 = _guard184 & _guard185;
wire _guard187 = _guard183 & _guard186;
wire _guard188 = _guard182 | _guard187;
wire _guard189 = early_reset_static_seq_go_out;
wire _guard190 = _guard188 & _guard189;
wire _guard191 = fsm12_out == 5'd1;
wire _guard192 = fsm12_out == 5'd3;
wire _guard193 = fsm12_out >= 5'd3;
wire _guard194 = fsm12_out < 5'd7;
wire _guard195 = _guard193 & _guard194;
wire _guard196 = _guard192 & _guard195;
wire _guard197 = _guard191 | _guard196;
wire _guard198 = fsm12_out == 5'd8;
wire _guard199 = fsm12_out >= 5'd8;
wire _guard200 = fsm12_out < 5'd12;
wire _guard201 = _guard199 & _guard200;
wire _guard202 = _guard198 & _guard201;
wire _guard203 = _guard197 | _guard202;
wire _guard204 = early_reset_static_seq_go_out;
wire _guard205 = _guard203 & _guard204;
wire _guard206 = fsm12_out == 5'd0;
wire _guard207 = signal_reg_out;
wire _guard208 = _guard206 & _guard207;
wire _guard209 = fsm12_out == 5'd0;
wire _guard210 = signal_reg_out;
wire _guard211 = ~_guard210;
wire _guard212 = _guard209 & _guard211;
wire _guard213 = wrapper_early_reset_static_seq_go_out;
wire _guard214 = _guard212 & _guard213;
wire _guard215 = _guard208 | _guard214;
wire _guard216 = fsm12_out == 5'd0;
wire _guard217 = signal_reg_out;
wire _guard218 = ~_guard217;
wire _guard219 = _guard216 & _guard218;
wire _guard220 = wrapper_early_reset_static_seq_go_out;
wire _guard221 = _guard219 & _guard220;
wire _guard222 = fsm12_out == 5'd0;
wire _guard223 = signal_reg_out;
wire _guard224 = _guard222 & _guard223;
wire _guard225 = fsm12_out >= 5'd3;
wire _guard226 = fsm12_out < 5'd7;
wire _guard227 = _guard225 & _guard226;
wire _guard228 = fsm12_out >= 5'd8;
wire _guard229 = fsm12_out < 5'd12;
wire _guard230 = _guard228 & _guard229;
wire _guard231 = _guard227 | _guard230;
wire _guard232 = fsm12_out >= 5'd13;
wire _guard233 = fsm12_out < 5'd17;
wire _guard234 = _guard232 & _guard233;
wire _guard235 = _guard231 | _guard234;
wire _guard236 = early_reset_static_seq_go_out;
wire _guard237 = _guard235 & _guard236;
wire _guard238 = fsm12_out >= 5'd3;
wire _guard239 = fsm12_out < 5'd7;
wire _guard240 = _guard238 & _guard239;
wire _guard241 = fsm12_out >= 5'd8;
wire _guard242 = fsm12_out < 5'd12;
wire _guard243 = _guard241 & _guard242;
wire _guard244 = _guard240 | _guard243;
wire _guard245 = fsm12_out >= 5'd13;
wire _guard246 = fsm12_out < 5'd17;
wire _guard247 = _guard245 & _guard246;
wire _guard248 = _guard244 | _guard247;
wire _guard249 = early_reset_static_seq_go_out;
wire _guard250 = _guard248 & _guard249;
wire _guard251 = fsm12_out >= 5'd3;
wire _guard252 = fsm12_out < 5'd7;
wire _guard253 = _guard251 & _guard252;
wire _guard254 = fsm12_out >= 5'd8;
wire _guard255 = fsm12_out < 5'd12;
wire _guard256 = _guard254 & _guard255;
wire _guard257 = _guard253 | _guard256;
wire _guard258 = fsm12_out >= 5'd13;
wire _guard259 = fsm12_out < 5'd17;
wire _guard260 = _guard258 & _guard259;
wire _guard261 = _guard257 | _guard260;
wire _guard262 = early_reset_static_seq_go_out;
wire _guard263 = _guard261 & _guard262;
wire _guard264 = fsm20_out == 2'd2;
wire _guard265 = fsm12_out == 5'd2;
wire _guard266 = fsm12_out == 5'd7;
wire _guard267 = _guard265 | _guard266;
wire _guard268 = fsm12_out == 5'd12;
wire _guard269 = _guard267 | _guard268;
wire _guard270 = early_reset_static_seq_go_out;
wire _guard271 = _guard269 & _guard270;
wire _guard272 = fsm12_out == 5'd2;
wire _guard273 = fsm12_out == 5'd7;
wire _guard274 = _guard272 | _guard273;
wire _guard275 = fsm12_out == 5'd12;
wire _guard276 = _guard274 | _guard275;
wire _guard277 = early_reset_static_seq_go_out;
wire _guard278 = _guard276 & _guard277;
wire _guard279 = wrapper_early_reset_static_seq_done_out;
wire _guard280 = ~_guard279;
wire _guard281 = fsm20_out == 2'd0;
wire _guard282 = _guard280 & _guard281;
wire _guard283 = tdcc_go_out;
wire _guard284 = _guard282 & _guard283;
assign out_mem_write_en = _guard1;
assign out_mem_clk = clk;
assign out_mem_addr0 =
  _guard2 ? 1'd0 :
  1'd0;
assign out_mem_reset = reset;
assign out_mem_write_data = pe_0_0_out;
assign done = _guard4;
assign adder12_left =
  _guard5 ? fsm12_out :
  5'd0;
assign adder12_right =
  _guard6 ? 5'd1 :
  5'd0;
assign fsm20_write_en = _guard19;
assign fsm20_clk = clk;
assign fsm20_reset = reset;
assign fsm20_in =
  _guard24 ? 2'd1 :
  _guard25 ? 2'd0 :
  _guard30 ? 2'd2 :
  2'd0;
always_comb begin
  if(~$onehot0({_guard30, _guard25, _guard24})) begin
    $fatal(2, "Multiple assignment to port `fsm20.in'.");
end
end
assign pe_0_0_out_write_go_in = _guard36;
assign l0_idx_write_en = _guard53;
assign l0_idx_clk = clk;
assign l0_idx_reset = reset;
assign l0_idx_in =
  _guard56 ? 2'd3 :
  _guard71 ? l0_add_out :
  'x;
always_comb begin
  if(~$onehot0({_guard71, _guard56})) begin
    $fatal(2, "Multiple assignment to port `l0_idx.in'.");
end
end
assign left_0_0_write_en = _guard78;
assign left_0_0_clk = clk;
assign left_0_0_reset = reset;
assign left_0_0_in = l0_read_data;
assign l0_clk = clk;
assign l0_addr0 =
  _guard92 ? l0_idx_out :
  2'd0;
assign l0_reset = reset;
assign fsm12_write_en = _guard93;
assign fsm12_clk = clk;
assign fsm12_reset = reset;
assign fsm12_in =
  _guard96 ? adder12_out :
  _guard99 ? 5'd0 :
  5'd0;
always_comb begin
  if(~$onehot0({_guard99, _guard96})) begin
    $fatal(2, "Multiple assignment to port `fsm12.in'.");
end
end
assign t0_idx_write_en = _guard116;
assign t0_idx_clk = clk;
assign t0_idx_reset = reset;
assign t0_idx_in =
  _guard119 ? 2'd3 :
  _guard134 ? t0_add_out :
  'x;
always_comb begin
  if(~$onehot0({_guard134, _guard119})) begin
    $fatal(2, "Multiple assignment to port `t0_idx.in'.");
end
end
assign pe_0_0_out_write_done_in = out_mem_done;
assign tdcc_go_in = go;
assign wrapper_early_reset_static_seq_done_in = _guard137;
assign early_reset_static_seq_go_in = _guard138;
assign t0_clk = clk;
assign t0_addr0 =
  _guard145 ? t0_idx_out :
  2'd0;
assign t0_reset = reset;
assign t0_add_left = 2'd1;
assign t0_add_right = t0_idx_out;
assign l0_add_left = 2'd1;
assign l0_add_right = l0_idx_out;
assign signal_reg_write_en = _guard215;
assign signal_reg_clk = clk;
assign signal_reg_reset = reset;
assign signal_reg_in =
  _guard221 ? 1'd1 :
  _guard224 ? 1'd0 :
  1'd0;
always_comb begin
  if(~$onehot0({_guard224, _guard221})) begin
    $fatal(2, "Multiple assignment to port `signal_reg.in'.");
end
end
assign pe_0_0_clk = clk;
assign pe_0_0_top =
  _guard237 ? top_0_0_out :
  32'd0;
assign pe_0_0_left =
  _guard250 ? left_0_0_out :
  32'd0;
assign pe_0_0_reset = reset;
assign pe_0_0_go = _guard263;
assign early_reset_static_seq_done_in = ud12_out;
assign tdcc_done_in = _guard264;
assign top_0_0_write_en = _guard271;
assign top_0_0_clk = clk;
assign top_0_0_reset = reset;
assign top_0_0_in = t0_read_data;
assign wrapper_early_reset_static_seq_go_in = _guard284;
// COMPONENT END: main
endmodule
