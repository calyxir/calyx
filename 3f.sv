
/// This is mostly used for testing the static guarantees currently.
/// A realistic implementation would probably take four cycles.
module pipelined_mult (
    input wire clk,
    input wire reset,
    // inputs
    input wire [31:0] left,
    input wire [31:0] right,
    // The input has been committed
    output wire [31:0] out
);

logic [31:0] lt, rt, buff0, buff1, buff2, tmp_prod;

assign out = buff2;
assign tmp_prod = lt * rt;

always_ff @(posedge clk) begin
    if (reset) begin
        lt <= 0;
        rt <= 0;
        buff0 <= 0;
        buff1 <= 0;
        buff2 <= 0;
    end else begin
        lt <= left;
        rt <= right;
        buff0 <= tmp_prod;
        buff1 <= buff0;
        buff2 <= buff1;
    end
end

endmodule 

/// 
module pipelined_fp_smult #(
    parameter WIDTH = 32,
    parameter INT_WIDTH = 16,
    parameter FRAC_WIDTH = 16
)(
    input wire clk,
    input wire reset,
    // inputs
    input wire [WIDTH-1:0] left,
    input wire [WIDTH-1:0] right,
    // The input has been committed
    output wire [WIDTH-1:0] out
);

logic [WIDTH-1:0] lt, rt;
logic [(WIDTH << 1) - 1:0] tmp_prod, buff0, buff1, buff2;

assign out = buff2;
assign tmp_prod = $signed(
          { {WIDTH{lt[WIDTH-1]}}, lt} *
          { {WIDTH{rt[WIDTH-1]}}, rt}
        );

always_ff @(posedge clk) begin
    if (reset) begin
        lt <= 0;
        rt <= 0;
        buff0 <= 0;
        buff1 <= 0;
        buff2 <= 0;
    end else begin
        lt <= $signed(left);
        rt <= $signed(right);
        buff0 <= tmp_prod[(WIDTH << 1) - INT_WIDTH - 1 : WIDTH - INT_WIDTH];
        buff1 <= buff0;
        buff2 <= buff1;
    end
end

endmodule

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
  input logic mul_ready,
  input logic go,
  input logic clk,
  input logic reset,
  output logic [31:0] out,
  output logic done
);
// COMPONENT START: mac_pe
logic [31:0] acc_in;
logic acc_write_en;
logic acc_clk;
logic acc_reset;
logic [31:0] acc_out;
logic acc_done;
logic [31:0] adder_left;
logic [31:0] adder_right;
logic [31:0] adder_out;
logic mul_clk;
logic mul_reset;
logic [31:0] mul_left;
logic [31:0] mul_right;
logic [31:0] mul_out;
logic fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic fsm_out;
logic fsm_done;
logic ud_out;
logic adder0_left;
logic adder0_right;
logic adder0_out;
logic signal_reg_in;
logic signal_reg_write_en;
logic signal_reg_clk;
logic signal_reg_reset;
logic signal_reg_out;
logic signal_reg_done;
logic early_reset_static_par_go_in;
logic early_reset_static_par_go_out;
logic early_reset_static_par_done_in;
logic early_reset_static_par_done_out;
logic wrapper_early_reset_static_par_go_in;
logic wrapper_early_reset_static_par_go_out;
logic wrapper_early_reset_static_par_done_in;
logic wrapper_early_reset_static_par_done_out;
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
std_fp_sadd # (
    .FRAC_WIDTH(16),
    .INT_WIDTH(16),
    .WIDTH(32)
) adder (
    .left(adder_left),
    .out(adder_out),
    .right(adder_right)
);
pipelined_fp_smult # (
    .FRAC_WIDTH(16),
    .INT_WIDTH(16),
    .WIDTH(32)
) mul (
    .clk(mul_clk),
    .left(mul_left),
    .out(mul_out),
    .reset(mul_reset),
    .right(mul_right)
);
std_reg # (
    .WIDTH(1)
) fsm (
    .clk(fsm_clk),
    .done(fsm_done),
    .in(fsm_in),
    .out(fsm_out),
    .reset(fsm_reset),
    .write_en(fsm_write_en)
);
undef # (
    .WIDTH(1)
) ud (
    .out(ud_out)
);
std_add # (
    .WIDTH(1)
) adder0 (
    .left(adder0_left),
    .out(adder0_out),
    .right(adder0_right)
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
) early_reset_static_par_go (
    .in(early_reset_static_par_go_in),
    .out(early_reset_static_par_go_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_static_par_done (
    .in(early_reset_static_par_done_in),
    .out(early_reset_static_par_done_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_static_par_go (
    .in(wrapper_early_reset_static_par_go_in),
    .out(wrapper_early_reset_static_par_go_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_static_par_done (
    .in(wrapper_early_reset_static_par_done_in),
    .out(wrapper_early_reset_static_par_done_out)
);
wire _guard0 = 1;
wire _guard1 = early_reset_static_par_go_out;
wire _guard2 = early_reset_static_par_go_out;
wire _guard3 = early_reset_static_par_go_out;
wire _guard4 = early_reset_static_par_go_out;
wire _guard5 = early_reset_static_par_go_out;
wire _guard6 = fsm_out == 1'd0;
wire _guard7 = early_reset_static_par_go_out;
wire _guard8 = _guard6 & _guard7;
wire _guard9 = fsm_out != 1'd0;
wire _guard10 = early_reset_static_par_go_out;
wire _guard11 = _guard9 & _guard10;
wire _guard12 = wrapper_early_reset_static_par_done_out;
wire _guard13 = fsm_out == 1'd0;
wire _guard14 = signal_reg_out;
wire _guard15 = _guard13 & _guard14;
wire _guard16 = early_reset_static_par_go_out;
wire _guard17 = early_reset_static_par_go_out;
wire _guard18 = fsm_out == 1'd0;
wire _guard19 = signal_reg_out;
wire _guard20 = _guard18 & _guard19;
wire _guard21 = fsm_out == 1'd0;
wire _guard22 = signal_reg_out;
wire _guard23 = ~_guard22;
wire _guard24 = _guard21 & _guard23;
wire _guard25 = wrapper_early_reset_static_par_go_out;
wire _guard26 = _guard24 & _guard25;
wire _guard27 = _guard20 | _guard26;
wire _guard28 = fsm_out == 1'd0;
wire _guard29 = signal_reg_out;
wire _guard30 = ~_guard29;
wire _guard31 = _guard28 & _guard30;
wire _guard32 = wrapper_early_reset_static_par_go_out;
wire _guard33 = _guard31 & _guard32;
wire _guard34 = fsm_out == 1'd0;
wire _guard35 = signal_reg_out;
wire _guard36 = _guard34 & _guard35;
wire _guard37 = early_reset_static_par_go_out;
wire _guard38 = early_reset_static_par_go_out;
wire _guard39 = wrapper_early_reset_static_par_go_out;
assign acc_write_en =
  _guard1 ? mul_ready :
  1'd0;
assign acc_clk = clk;
assign acc_reset = reset;
assign acc_in = adder_out;
assign adder_left = acc_out;
assign adder_right = mul_out;
assign fsm_write_en = _guard5;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard8 ? 1'd0 :
  _guard11 ? adder0_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard11, _guard8})) begin
    $fatal(2, "Multiple assignment to port `fsm.in'.");
end
end
assign done = _guard12;
assign out = acc_out;
assign wrapper_early_reset_static_par_go_in = go;
assign wrapper_early_reset_static_par_done_in = _guard15;
assign adder0_left =
  _guard16 ? fsm_out :
  1'd0;
assign adder0_right = _guard17;
assign early_reset_static_par_done_in = ud_out;
assign signal_reg_write_en = _guard27;
assign signal_reg_clk = clk;
assign signal_reg_reset = reset;
assign signal_reg_in =
  _guard33 ? 1'd1 :
  _guard36 ? 1'd0 :
  1'd0;
always_comb begin
  if(~$onehot0({_guard36, _guard33})) begin
    $fatal(2, "Multiple assignment to port `signal_reg.in'.");
end
end
assign mul_clk = clk;
assign mul_left =
  _guard37 ? top :
  32'd0;
assign mul_reset = reset;
assign mul_right =
  _guard38 ? left :
  32'd0;
assign early_reset_static_par_go_in = _guard39;
// COMPONENT END: mac_pe
endmodule
module systolic_array_comp(
  input logic [31:0] depth,
  input logic [31:0] t0_read_data,
  input logic [31:0] t1_read_data,
  input logic [31:0] t2_read_data,
  input logic [31:0] l0_read_data,
  input logic [31:0] l1_read_data,
  input logic [31:0] l2_read_data,
  input logic go,
  input logic clk,
  input logic reset,
  output logic [1:0] t0_addr0,
  output logic [1:0] t1_addr0,
  output logic [1:0] t2_addr0,
  output logic [1:0] l0_addr0,
  output logic [1:0] l1_addr0,
  output logic [1:0] l2_addr0,
  output logic [31:0] out_mem_0_addr0,
  output logic [31:0] out_mem_0_write_data,
  output logic out_mem_0_write_en,
  output logic [31:0] out_mem_1_addr0,
  output logic [31:0] out_mem_1_write_data,
  output logic out_mem_1_write_en,
  output logic [31:0] out_mem_2_addr0,
  output logic [31:0] out_mem_2_write_data,
  output logic out_mem_2_write_en,
  output logic done
);
// COMPONENT START: systolic_array_comp
logic [31:0] min_depth_4_in;
logic min_depth_4_write_en;
logic min_depth_4_clk;
logic min_depth_4_reset;
logic [31:0] min_depth_4_out;
logic min_depth_4_done;
logic [31:0] iter_limit_in;
logic iter_limit_write_en;
logic iter_limit_clk;
logic iter_limit_reset;
logic [31:0] iter_limit_out;
logic iter_limit_done;
logic [31:0] depth_plus_4_left;
logic [31:0] depth_plus_4_right;
logic [31:0] depth_plus_4_out;
logic [31:0] min_depth_4_plus_4_left;
logic [31:0] min_depth_4_plus_4_right;
logic [31:0] min_depth_4_plus_4_out;
logic [31:0] depth_plus_8_left;
logic [31:0] depth_plus_8_right;
logic [31:0] depth_plus_8_out;
logic [31:0] min_depth_4_plus_5_left;
logic [31:0] min_depth_4_plus_5_right;
logic [31:0] min_depth_4_plus_5_out;
logic [31:0] min_depth_4_plus_2_left;
logic [31:0] min_depth_4_plus_2_right;
logic [31:0] min_depth_4_plus_2_out;
logic [31:0] depth_plus_5_left;
logic [31:0] depth_plus_5_right;
logic [31:0] depth_plus_5_out;
logic [31:0] depth_plus_7_left;
logic [31:0] depth_plus_7_right;
logic [31:0] depth_plus_7_out;
logic [31:0] depth_plus_0_left;
logic [31:0] depth_plus_0_right;
logic [31:0] depth_plus_0_out;
logic [31:0] depth_plus_9_left;
logic [31:0] depth_plus_9_right;
logic [31:0] depth_plus_9_out;
logic [31:0] depth_plus_6_left;
logic [31:0] depth_plus_6_right;
logic [31:0] depth_plus_6_out;
logic [31:0] depth_plus_1_left;
logic [31:0] depth_plus_1_right;
logic [31:0] depth_plus_1_out;
logic [31:0] min_depth_4_plus_1_left;
logic [31:0] min_depth_4_plus_1_right;
logic [31:0] min_depth_4_plus_1_out;
logic [31:0] depth_plus_3_left;
logic [31:0] depth_plus_3_right;
logic [31:0] depth_plus_3_out;
logic [31:0] min_depth_4_plus_3_left;
logic [31:0] min_depth_4_plus_3_right;
logic [31:0] min_depth_4_plus_3_out;
logic [31:0] depth_plus_2_left;
logic [31:0] depth_plus_2_right;
logic [31:0] depth_plus_2_out;
logic [31:0] pe_0_0_top;
logic [31:0] pe_0_0_left;
logic pe_0_0_mul_ready;
logic pe_0_0_go;
logic pe_0_0_clk;
logic pe_0_0_reset;
logic [31:0] pe_0_0_out;
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
logic [31:0] pe_0_1_top;
logic [31:0] pe_0_1_left;
logic pe_0_1_mul_ready;
logic pe_0_1_go;
logic pe_0_1_clk;
logic pe_0_1_reset;
logic [31:0] pe_0_1_out;
logic pe_0_1_done;
logic [31:0] top_0_1_in;
logic top_0_1_write_en;
logic top_0_1_clk;
logic top_0_1_reset;
logic [31:0] top_0_1_out;
logic top_0_1_done;
logic [31:0] left_0_1_in;
logic left_0_1_write_en;
logic left_0_1_clk;
logic left_0_1_reset;
logic [31:0] left_0_1_out;
logic left_0_1_done;
logic [31:0] pe_0_2_top;
logic [31:0] pe_0_2_left;
logic pe_0_2_mul_ready;
logic pe_0_2_go;
logic pe_0_2_clk;
logic pe_0_2_reset;
logic [31:0] pe_0_2_out;
logic pe_0_2_done;
logic [31:0] top_0_2_in;
logic top_0_2_write_en;
logic top_0_2_clk;
logic top_0_2_reset;
logic [31:0] top_0_2_out;
logic top_0_2_done;
logic [31:0] left_0_2_in;
logic left_0_2_write_en;
logic left_0_2_clk;
logic left_0_2_reset;
logic [31:0] left_0_2_out;
logic left_0_2_done;
logic [31:0] pe_1_0_top;
logic [31:0] pe_1_0_left;
logic pe_1_0_mul_ready;
logic pe_1_0_go;
logic pe_1_0_clk;
logic pe_1_0_reset;
logic [31:0] pe_1_0_out;
logic pe_1_0_done;
logic [31:0] top_1_0_in;
logic top_1_0_write_en;
logic top_1_0_clk;
logic top_1_0_reset;
logic [31:0] top_1_0_out;
logic top_1_0_done;
logic [31:0] left_1_0_in;
logic left_1_0_write_en;
logic left_1_0_clk;
logic left_1_0_reset;
logic [31:0] left_1_0_out;
logic left_1_0_done;
logic [31:0] pe_1_1_top;
logic [31:0] pe_1_1_left;
logic pe_1_1_mul_ready;
logic pe_1_1_go;
logic pe_1_1_clk;
logic pe_1_1_reset;
logic [31:0] pe_1_1_out;
logic pe_1_1_done;
logic [31:0] top_1_1_in;
logic top_1_1_write_en;
logic top_1_1_clk;
logic top_1_1_reset;
logic [31:0] top_1_1_out;
logic top_1_1_done;
logic [31:0] left_1_1_in;
logic left_1_1_write_en;
logic left_1_1_clk;
logic left_1_1_reset;
logic [31:0] left_1_1_out;
logic left_1_1_done;
logic [31:0] pe_1_2_top;
logic [31:0] pe_1_2_left;
logic pe_1_2_mul_ready;
logic pe_1_2_go;
logic pe_1_2_clk;
logic pe_1_2_reset;
logic [31:0] pe_1_2_out;
logic pe_1_2_done;
logic [31:0] top_1_2_in;
logic top_1_2_write_en;
logic top_1_2_clk;
logic top_1_2_reset;
logic [31:0] top_1_2_out;
logic top_1_2_done;
logic [31:0] left_1_2_in;
logic left_1_2_write_en;
logic left_1_2_clk;
logic left_1_2_reset;
logic [31:0] left_1_2_out;
logic left_1_2_done;
logic [31:0] pe_2_0_top;
logic [31:0] pe_2_0_left;
logic pe_2_0_mul_ready;
logic pe_2_0_go;
logic pe_2_0_clk;
logic pe_2_0_reset;
logic [31:0] pe_2_0_out;
logic pe_2_0_done;
logic [31:0] top_2_0_in;
logic top_2_0_write_en;
logic top_2_0_clk;
logic top_2_0_reset;
logic [31:0] top_2_0_out;
logic top_2_0_done;
logic [31:0] left_2_0_in;
logic left_2_0_write_en;
logic left_2_0_clk;
logic left_2_0_reset;
logic [31:0] left_2_0_out;
logic left_2_0_done;
logic [31:0] pe_2_1_top;
logic [31:0] pe_2_1_left;
logic pe_2_1_mul_ready;
logic pe_2_1_go;
logic pe_2_1_clk;
logic pe_2_1_reset;
logic [31:0] pe_2_1_out;
logic pe_2_1_done;
logic [31:0] top_2_1_in;
logic top_2_1_write_en;
logic top_2_1_clk;
logic top_2_1_reset;
logic [31:0] top_2_1_out;
logic top_2_1_done;
logic [31:0] left_2_1_in;
logic left_2_1_write_en;
logic left_2_1_clk;
logic left_2_1_reset;
logic [31:0] left_2_1_out;
logic left_2_1_done;
logic [31:0] pe_2_2_top;
logic [31:0] pe_2_2_left;
logic pe_2_2_mul_ready;
logic pe_2_2_go;
logic pe_2_2_clk;
logic pe_2_2_reset;
logic [31:0] pe_2_2_out;
logic pe_2_2_done;
logic [31:0] top_2_2_in;
logic top_2_2_write_en;
logic top_2_2_clk;
logic top_2_2_reset;
logic [31:0] top_2_2_out;
logic top_2_2_done;
logic [31:0] left_2_2_in;
logic left_2_2_write_en;
logic left_2_2_clk;
logic left_2_2_reset;
logic [31:0] left_2_2_out;
logic left_2_2_done;
logic [1:0] t0_idx_in;
logic t0_idx_write_en;
logic t0_idx_clk;
logic t0_idx_reset;
logic [1:0] t0_idx_out;
logic t0_idx_done;
logic [1:0] t0_add_left;
logic [1:0] t0_add_right;
logic [1:0] t0_add_out;
logic [1:0] t1_idx_in;
logic t1_idx_write_en;
logic t1_idx_clk;
logic t1_idx_reset;
logic [1:0] t1_idx_out;
logic t1_idx_done;
logic [1:0] t1_add_left;
logic [1:0] t1_add_right;
logic [1:0] t1_add_out;
logic [1:0] t2_idx_in;
logic t2_idx_write_en;
logic t2_idx_clk;
logic t2_idx_reset;
logic [1:0] t2_idx_out;
logic t2_idx_done;
logic [1:0] t2_add_left;
logic [1:0] t2_add_right;
logic [1:0] t2_add_out;
logic [1:0] l0_idx_in;
logic l0_idx_write_en;
logic l0_idx_clk;
logic l0_idx_reset;
logic [1:0] l0_idx_out;
logic l0_idx_done;
logic [1:0] l0_add_left;
logic [1:0] l0_add_right;
logic [1:0] l0_add_out;
logic [1:0] l1_idx_in;
logic l1_idx_write_en;
logic l1_idx_clk;
logic l1_idx_reset;
logic [1:0] l1_idx_out;
logic l1_idx_done;
logic [1:0] l1_add_left;
logic [1:0] l1_add_right;
logic [1:0] l1_add_out;
logic [1:0] l2_idx_in;
logic l2_idx_write_en;
logic l2_idx_clk;
logic l2_idx_reset;
logic [1:0] l2_idx_out;
logic l2_idx_done;
logic [1:0] l2_add_left;
logic [1:0] l2_add_right;
logic [1:0] l2_add_out;
logic [31:0] idx_in;
logic idx_write_en;
logic idx_clk;
logic idx_reset;
logic [31:0] idx_out;
logic idx_done;
logic [31:0] idx_add_left;
logic [31:0] idx_add_right;
logic [31:0] idx_add_out;
logic [31:0] lt_iter_limit_left;
logic [31:0] lt_iter_limit_right;
logic lt_iter_limit_out;
logic cond_reg_in;
logic cond_reg_write_en;
logic cond_reg_clk;
logic cond_reg_reset;
logic cond_reg_out;
logic cond_reg_done;
logic idx_between_4_depth_plus_4_reg_in;
logic idx_between_4_depth_plus_4_reg_write_en;
logic idx_between_4_depth_plus_4_reg_clk;
logic idx_between_4_depth_plus_4_reg_reset;
logic idx_between_4_depth_plus_4_reg_out;
logic idx_between_4_depth_plus_4_reg_done;
logic [31:0] index_lt_depth_plus_4_left;
logic [31:0] index_lt_depth_plus_4_right;
logic index_lt_depth_plus_4_out;
logic [31:0] index_ge_4_left;
logic [31:0] index_ge_4_right;
logic index_ge_4_out;
logic idx_between_4_depth_plus_4_comb_left;
logic idx_between_4_depth_plus_4_comb_right;
logic idx_between_4_depth_plus_4_comb_out;
logic idx_between_4_min_depth_4_plus_4_reg_in;
logic idx_between_4_min_depth_4_plus_4_reg_write_en;
logic idx_between_4_min_depth_4_plus_4_reg_clk;
logic idx_between_4_min_depth_4_plus_4_reg_reset;
logic idx_between_4_min_depth_4_plus_4_reg_out;
logic idx_between_4_min_depth_4_plus_4_reg_done;
logic [31:0] index_lt_min_depth_4_plus_4_left;
logic [31:0] index_lt_min_depth_4_plus_4_right;
logic index_lt_min_depth_4_plus_4_out;
logic idx_between_4_min_depth_4_plus_4_comb_left;
logic idx_between_4_min_depth_4_plus_4_comb_right;
logic idx_between_4_min_depth_4_plus_4_comb_out;
logic idx_between_8_depth_plus_8_reg_in;
logic idx_between_8_depth_plus_8_reg_write_en;
logic idx_between_8_depth_plus_8_reg_clk;
logic idx_between_8_depth_plus_8_reg_reset;
logic idx_between_8_depth_plus_8_reg_out;
logic idx_between_8_depth_plus_8_reg_done;
logic [31:0] index_lt_depth_plus_8_left;
logic [31:0] index_lt_depth_plus_8_right;
logic index_lt_depth_plus_8_out;
logic [31:0] index_ge_8_left;
logic [31:0] index_ge_8_right;
logic index_ge_8_out;
logic idx_between_8_depth_plus_8_comb_left;
logic idx_between_8_depth_plus_8_comb_right;
logic idx_between_8_depth_plus_8_comb_out;
logic idx_between_5_min_depth_4_plus_5_reg_in;
logic idx_between_5_min_depth_4_plus_5_reg_write_en;
logic idx_between_5_min_depth_4_plus_5_reg_clk;
logic idx_between_5_min_depth_4_plus_5_reg_reset;
logic idx_between_5_min_depth_4_plus_5_reg_out;
logic idx_between_5_min_depth_4_plus_5_reg_done;
logic [31:0] index_lt_min_depth_4_plus_5_left;
logic [31:0] index_lt_min_depth_4_plus_5_right;
logic index_lt_min_depth_4_plus_5_out;
logic [31:0] index_ge_5_left;
logic [31:0] index_ge_5_right;
logic index_ge_5_out;
logic idx_between_5_min_depth_4_plus_5_comb_left;
logic idx_between_5_min_depth_4_plus_5_comb_right;
logic idx_between_5_min_depth_4_plus_5_comb_out;
logic idx_between_2_min_depth_4_plus_2_reg_in;
logic idx_between_2_min_depth_4_plus_2_reg_write_en;
logic idx_between_2_min_depth_4_plus_2_reg_clk;
logic idx_between_2_min_depth_4_plus_2_reg_reset;
logic idx_between_2_min_depth_4_plus_2_reg_out;
logic idx_between_2_min_depth_4_plus_2_reg_done;
logic [31:0] index_lt_min_depth_4_plus_2_left;
logic [31:0] index_lt_min_depth_4_plus_2_right;
logic index_lt_min_depth_4_plus_2_out;
logic [31:0] index_ge_2_left;
logic [31:0] index_ge_2_right;
logic index_ge_2_out;
logic idx_between_2_min_depth_4_plus_2_comb_left;
logic idx_between_2_min_depth_4_plus_2_comb_right;
logic idx_between_2_min_depth_4_plus_2_comb_out;
logic idx_between_5_depth_plus_5_reg_in;
logic idx_between_5_depth_plus_5_reg_write_en;
logic idx_between_5_depth_plus_5_reg_clk;
logic idx_between_5_depth_plus_5_reg_reset;
logic idx_between_5_depth_plus_5_reg_out;
logic idx_between_5_depth_plus_5_reg_done;
logic [31:0] index_lt_depth_plus_5_left;
logic [31:0] index_lt_depth_plus_5_right;
logic index_lt_depth_plus_5_out;
logic idx_between_5_depth_plus_5_comb_left;
logic idx_between_5_depth_plus_5_comb_right;
logic idx_between_5_depth_plus_5_comb_out;
logic idx_between_7_depth_plus_7_reg_in;
logic idx_between_7_depth_plus_7_reg_write_en;
logic idx_between_7_depth_plus_7_reg_clk;
logic idx_between_7_depth_plus_7_reg_reset;
logic idx_between_7_depth_plus_7_reg_out;
logic idx_between_7_depth_plus_7_reg_done;
logic [31:0] index_lt_depth_plus_7_left;
logic [31:0] index_lt_depth_plus_7_right;
logic index_lt_depth_plus_7_out;
logic [31:0] index_ge_7_left;
logic [31:0] index_ge_7_right;
logic index_ge_7_out;
logic idx_between_7_depth_plus_7_comb_left;
logic idx_between_7_depth_plus_7_comb_right;
logic idx_between_7_depth_plus_7_comb_out;
logic idx_between_0_depth_plus_0_reg_in;
logic idx_between_0_depth_plus_0_reg_write_en;
logic idx_between_0_depth_plus_0_reg_clk;
logic idx_between_0_depth_plus_0_reg_reset;
logic idx_between_0_depth_plus_0_reg_out;
logic idx_between_0_depth_plus_0_reg_done;
logic [31:0] index_lt_depth_plus_0_left;
logic [31:0] index_lt_depth_plus_0_right;
logic index_lt_depth_plus_0_out;
logic idx_between_9_depth_plus_9_reg_in;
logic idx_between_9_depth_plus_9_reg_write_en;
logic idx_between_9_depth_plus_9_reg_clk;
logic idx_between_9_depth_plus_9_reg_reset;
logic idx_between_9_depth_plus_9_reg_out;
logic idx_between_9_depth_plus_9_reg_done;
logic [31:0] index_lt_depth_plus_9_left;
logic [31:0] index_lt_depth_plus_9_right;
logic index_lt_depth_plus_9_out;
logic [31:0] index_ge_9_left;
logic [31:0] index_ge_9_right;
logic index_ge_9_out;
logic idx_between_9_depth_plus_9_comb_left;
logic idx_between_9_depth_plus_9_comb_right;
logic idx_between_9_depth_plus_9_comb_out;
logic idx_between_depth_plus_6_None_reg_in;
logic idx_between_depth_plus_6_None_reg_write_en;
logic idx_between_depth_plus_6_None_reg_clk;
logic idx_between_depth_plus_6_None_reg_reset;
logic idx_between_depth_plus_6_None_reg_out;
logic idx_between_depth_plus_6_None_reg_done;
logic idx_between_1_depth_plus_1_reg_in;
logic idx_between_1_depth_plus_1_reg_write_en;
logic idx_between_1_depth_plus_1_reg_clk;
logic idx_between_1_depth_plus_1_reg_reset;
logic idx_between_1_depth_plus_1_reg_out;
logic idx_between_1_depth_plus_1_reg_done;
logic [31:0] index_lt_depth_plus_1_left;
logic [31:0] index_lt_depth_plus_1_right;
logic index_lt_depth_plus_1_out;
logic [31:0] index_ge_1_left;
logic [31:0] index_ge_1_right;
logic index_ge_1_out;
logic idx_between_1_depth_plus_1_comb_left;
logic idx_between_1_depth_plus_1_comb_right;
logic idx_between_1_depth_plus_1_comb_out;
logic idx_between_1_min_depth_4_plus_1_reg_in;
logic idx_between_1_min_depth_4_plus_1_reg_write_en;
logic idx_between_1_min_depth_4_plus_1_reg_clk;
logic idx_between_1_min_depth_4_plus_1_reg_reset;
logic idx_between_1_min_depth_4_plus_1_reg_out;
logic idx_between_1_min_depth_4_plus_1_reg_done;
logic [31:0] index_lt_min_depth_4_plus_1_left;
logic [31:0] index_lt_min_depth_4_plus_1_right;
logic index_lt_min_depth_4_plus_1_out;
logic idx_between_1_min_depth_4_plus_1_comb_left;
logic idx_between_1_min_depth_4_plus_1_comb_right;
logic idx_between_1_min_depth_4_plus_1_comb_out;
logic idx_between_3_depth_plus_3_reg_in;
logic idx_between_3_depth_plus_3_reg_write_en;
logic idx_between_3_depth_plus_3_reg_clk;
logic idx_between_3_depth_plus_3_reg_reset;
logic idx_between_3_depth_plus_3_reg_out;
logic idx_between_3_depth_plus_3_reg_done;
logic [31:0] index_lt_depth_plus_3_left;
logic [31:0] index_lt_depth_plus_3_right;
logic index_lt_depth_plus_3_out;
logic [31:0] index_ge_3_left;
logic [31:0] index_ge_3_right;
logic index_ge_3_out;
logic idx_between_3_depth_plus_3_comb_left;
logic idx_between_3_depth_plus_3_comb_right;
logic idx_between_3_depth_plus_3_comb_out;
logic idx_between_3_min_depth_4_plus_3_reg_in;
logic idx_between_3_min_depth_4_plus_3_reg_write_en;
logic idx_between_3_min_depth_4_plus_3_reg_clk;
logic idx_between_3_min_depth_4_plus_3_reg_reset;
logic idx_between_3_min_depth_4_plus_3_reg_out;
logic idx_between_3_min_depth_4_plus_3_reg_done;
logic [31:0] index_lt_min_depth_4_plus_3_left;
logic [31:0] index_lt_min_depth_4_plus_3_right;
logic index_lt_min_depth_4_plus_3_out;
logic idx_between_3_min_depth_4_plus_3_comb_left;
logic idx_between_3_min_depth_4_plus_3_comb_right;
logic idx_between_3_min_depth_4_plus_3_comb_out;
logic idx_between_depth_plus_7_None_reg_in;
logic idx_between_depth_plus_7_None_reg_write_en;
logic idx_between_depth_plus_7_None_reg_clk;
logic idx_between_depth_plus_7_None_reg_reset;
logic idx_between_depth_plus_7_None_reg_out;
logic idx_between_depth_plus_7_None_reg_done;
logic idx_between_2_depth_plus_2_reg_in;
logic idx_between_2_depth_plus_2_reg_write_en;
logic idx_between_2_depth_plus_2_reg_clk;
logic idx_between_2_depth_plus_2_reg_reset;
logic idx_between_2_depth_plus_2_reg_out;
logic idx_between_2_depth_plus_2_reg_done;
logic [31:0] index_lt_depth_plus_2_left;
logic [31:0] index_lt_depth_plus_2_right;
logic index_lt_depth_plus_2_out;
logic idx_between_2_depth_plus_2_comb_left;
logic idx_between_2_depth_plus_2_comb_right;
logic idx_between_2_depth_plus_2_comb_out;
logic idx_between_6_depth_plus_6_reg_in;
logic idx_between_6_depth_plus_6_reg_write_en;
logic idx_between_6_depth_plus_6_reg_clk;
logic idx_between_6_depth_plus_6_reg_reset;
logic idx_between_6_depth_plus_6_reg_out;
logic idx_between_6_depth_plus_6_reg_done;
logic [31:0] index_lt_depth_plus_6_left;
logic [31:0] index_lt_depth_plus_6_right;
logic index_lt_depth_plus_6_out;
logic [31:0] index_ge_6_left;
logic [31:0] index_ge_6_right;
logic index_ge_6_out;
logic idx_between_6_depth_plus_6_comb_left;
logic idx_between_6_depth_plus_6_comb_right;
logic idx_between_6_depth_plus_6_comb_out;
logic idx_between_depth_plus_5_None_reg_in;
logic idx_between_depth_plus_5_None_reg_write_en;
logic idx_between_depth_plus_5_None_reg_clk;
logic idx_between_depth_plus_5_None_reg_reset;
logic idx_between_depth_plus_5_None_reg_out;
logic idx_between_depth_plus_5_None_reg_done;
logic [31:0] relu_r0_cur_val_in;
logic [31:0] relu_r0_cur_val_out;
logic [31:0] relu_r0_cur_idx_in;
logic relu_r0_cur_idx_write_en;
logic relu_r0_cur_idx_clk;
logic relu_r0_cur_idx_reset;
logic [31:0] relu_r0_cur_idx_out;
logic relu_r0_cur_idx_done;
logic [31:0] relu_r0_val_gt_left;
logic [31:0] relu_r0_val_gt_right;
logic relu_r0_val_gt_out;
logic [31:0] relu_r0_go_next_in;
logic [31:0] relu_r0_go_next_out;
logic [31:0] relu_r0_incr_left;
logic [31:0] relu_r0_incr_right;
logic [31:0] relu_r0_incr_out;
logic relu_r0_val_mult_clk;
logic relu_r0_val_mult_reset;
logic relu_r0_val_mult_go;
logic [31:0] relu_r0_val_mult_left;
logic [31:0] relu_r0_val_mult_right;
logic [31:0] relu_r0_val_mult_out;
logic relu_r0_val_mult_done;
logic [31:0] relu_r1_cur_val_in;
logic [31:0] relu_r1_cur_val_out;
logic [31:0] relu_r1_cur_idx_in;
logic relu_r1_cur_idx_write_en;
logic relu_r1_cur_idx_clk;
logic relu_r1_cur_idx_reset;
logic [31:0] relu_r1_cur_idx_out;
logic relu_r1_cur_idx_done;
logic [31:0] relu_r1_val_gt_left;
logic [31:0] relu_r1_val_gt_right;
logic relu_r1_val_gt_out;
logic [31:0] relu_r1_go_next_in;
logic [31:0] relu_r1_go_next_out;
logic [31:0] relu_r1_incr_left;
logic [31:0] relu_r1_incr_right;
logic [31:0] relu_r1_incr_out;
logic relu_r1_val_mult_clk;
logic relu_r1_val_mult_reset;
logic relu_r1_val_mult_go;
logic [31:0] relu_r1_val_mult_left;
logic [31:0] relu_r1_val_mult_right;
logic [31:0] relu_r1_val_mult_out;
logic relu_r1_val_mult_done;
logic [31:0] relu_r2_cur_val_in;
logic [31:0] relu_r2_cur_val_out;
logic [31:0] relu_r2_cur_idx_in;
logic relu_r2_cur_idx_write_en;
logic relu_r2_cur_idx_clk;
logic relu_r2_cur_idx_reset;
logic [31:0] relu_r2_cur_idx_out;
logic relu_r2_cur_idx_done;
logic [31:0] relu_r2_val_gt_left;
logic [31:0] relu_r2_val_gt_right;
logic relu_r2_val_gt_out;
logic [31:0] relu_r2_go_next_in;
logic [31:0] relu_r2_go_next_out;
logic [31:0] relu_r2_incr_left;
logic [31:0] relu_r2_incr_right;
logic [31:0] relu_r2_incr_out;
logic relu_r2_val_mult_clk;
logic relu_r2_val_mult_reset;
logic relu_r2_val_mult_go;
logic [31:0] relu_r2_val_mult_left;
logic [31:0] relu_r2_val_mult_right;
logic [31:0] relu_r2_val_mult_out;
logic relu_r2_val_mult_done;
logic cond_in;
logic cond_write_en;
logic cond_clk;
logic cond_reset;
logic cond_out;
logic cond_done;
logic cond_wire_in;
logic cond_wire_out;
logic cond0_in;
logic cond0_write_en;
logic cond0_clk;
logic cond0_reset;
logic cond0_out;
logic cond0_done;
logic cond_wire0_in;
logic cond_wire0_out;
logic cond1_in;
logic cond1_write_en;
logic cond1_clk;
logic cond1_reset;
logic cond1_out;
logic cond1_done;
logic cond_wire1_in;
logic cond_wire1_out;
logic cond2_in;
logic cond2_write_en;
logic cond2_clk;
logic cond2_reset;
logic cond2_out;
logic cond2_done;
logic cond_wire2_in;
logic cond_wire2_out;
logic cond3_in;
logic cond3_write_en;
logic cond3_clk;
logic cond3_reset;
logic cond3_out;
logic cond3_done;
logic cond_wire3_in;
logic cond_wire3_out;
logic cond4_in;
logic cond4_write_en;
logic cond4_clk;
logic cond4_reset;
logic cond4_out;
logic cond4_done;
logic cond_wire4_in;
logic cond_wire4_out;
logic cond5_in;
logic cond5_write_en;
logic cond5_clk;
logic cond5_reset;
logic cond5_out;
logic cond5_done;
logic cond_wire5_in;
logic cond_wire5_out;
logic cond6_in;
logic cond6_write_en;
logic cond6_clk;
logic cond6_reset;
logic cond6_out;
logic cond6_done;
logic cond_wire6_in;
logic cond_wire6_out;
logic cond7_in;
logic cond7_write_en;
logic cond7_clk;
logic cond7_reset;
logic cond7_out;
logic cond7_done;
logic cond_wire7_in;
logic cond_wire7_out;
logic cond8_in;
logic cond8_write_en;
logic cond8_clk;
logic cond8_reset;
logic cond8_out;
logic cond8_done;
logic cond_wire8_in;
logic cond_wire8_out;
logic cond9_in;
logic cond9_write_en;
logic cond9_clk;
logic cond9_reset;
logic cond9_out;
logic cond9_done;
logic cond_wire9_in;
logic cond_wire9_out;
logic cond10_in;
logic cond10_write_en;
logic cond10_clk;
logic cond10_reset;
logic cond10_out;
logic cond10_done;
logic cond_wire10_in;
logic cond_wire10_out;
logic cond11_in;
logic cond11_write_en;
logic cond11_clk;
logic cond11_reset;
logic cond11_out;
logic cond11_done;
logic cond_wire11_in;
logic cond_wire11_out;
logic cond12_in;
logic cond12_write_en;
logic cond12_clk;
logic cond12_reset;
logic cond12_out;
logic cond12_done;
logic cond_wire12_in;
logic cond_wire12_out;
logic cond13_in;
logic cond13_write_en;
logic cond13_clk;
logic cond13_reset;
logic cond13_out;
logic cond13_done;
logic cond_wire13_in;
logic cond_wire13_out;
logic cond14_in;
logic cond14_write_en;
logic cond14_clk;
logic cond14_reset;
logic cond14_out;
logic cond14_done;
logic cond_wire14_in;
logic cond_wire14_out;
logic cond15_in;
logic cond15_write_en;
logic cond15_clk;
logic cond15_reset;
logic cond15_out;
logic cond15_done;
logic cond_wire15_in;
logic cond_wire15_out;
logic cond16_in;
logic cond16_write_en;
logic cond16_clk;
logic cond16_reset;
logic cond16_out;
logic cond16_done;
logic cond_wire16_in;
logic cond_wire16_out;
logic cond17_in;
logic cond17_write_en;
logic cond17_clk;
logic cond17_reset;
logic cond17_out;
logic cond17_done;
logic cond_wire17_in;
logic cond_wire17_out;
logic cond18_in;
logic cond18_write_en;
logic cond18_clk;
logic cond18_reset;
logic cond18_out;
logic cond18_done;
logic cond_wire18_in;
logic cond_wire18_out;
logic cond19_in;
logic cond19_write_en;
logic cond19_clk;
logic cond19_reset;
logic cond19_out;
logic cond19_done;
logic cond_wire19_in;
logic cond_wire19_out;
logic cond20_in;
logic cond20_write_en;
logic cond20_clk;
logic cond20_reset;
logic cond20_out;
logic cond20_done;
logic cond_wire20_in;
logic cond_wire20_out;
logic cond21_in;
logic cond21_write_en;
logic cond21_clk;
logic cond21_reset;
logic cond21_out;
logic cond21_done;
logic cond_wire21_in;
logic cond_wire21_out;
logic cond22_in;
logic cond22_write_en;
logic cond22_clk;
logic cond22_reset;
logic cond22_out;
logic cond22_done;
logic cond_wire22_in;
logic cond_wire22_out;
logic cond23_in;
logic cond23_write_en;
logic cond23_clk;
logic cond23_reset;
logic cond23_out;
logic cond23_done;
logic cond_wire23_in;
logic cond_wire23_out;
logic cond24_in;
logic cond24_write_en;
logic cond24_clk;
logic cond24_reset;
logic cond24_out;
logic cond24_done;
logic cond_wire24_in;
logic cond_wire24_out;
logic cond25_in;
logic cond25_write_en;
logic cond25_clk;
logic cond25_reset;
logic cond25_out;
logic cond25_done;
logic cond_wire25_in;
logic cond_wire25_out;
logic cond26_in;
logic cond26_write_en;
logic cond26_clk;
logic cond26_reset;
logic cond26_out;
logic cond26_done;
logic cond_wire26_in;
logic cond_wire26_out;
logic cond27_in;
logic cond27_write_en;
logic cond27_clk;
logic cond27_reset;
logic cond27_out;
logic cond27_done;
logic cond_wire27_in;
logic cond_wire27_out;
logic cond28_in;
logic cond28_write_en;
logic cond28_clk;
logic cond28_reset;
logic cond28_out;
logic cond28_done;
logic cond_wire28_in;
logic cond_wire28_out;
logic cond29_in;
logic cond29_write_en;
logic cond29_clk;
logic cond29_reset;
logic cond29_out;
logic cond29_done;
logic cond_wire29_in;
logic cond_wire29_out;
logic cond30_in;
logic cond30_write_en;
logic cond30_clk;
logic cond30_reset;
logic cond30_out;
logic cond30_done;
logic cond_wire30_in;
logic cond_wire30_out;
logic cond31_in;
logic cond31_write_en;
logic cond31_clk;
logic cond31_reset;
logic cond31_out;
logic cond31_done;
logic cond_wire31_in;
logic cond_wire31_out;
logic cond32_in;
logic cond32_write_en;
logic cond32_clk;
logic cond32_reset;
logic cond32_out;
logic cond32_done;
logic cond_wire32_in;
logic cond_wire32_out;
logic fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic fsm_out;
logic fsm_done;
logic ud_out;
logic adder_left;
logic adder_right;
logic adder_out;
logic ud0_out;
logic adder0_left;
logic adder0_right;
logic adder0_out;
logic signal_reg_in;
logic signal_reg_write_en;
logic signal_reg_clk;
logic signal_reg_reset;
logic signal_reg_out;
logic signal_reg_done;
logic [1:0] fsm0_in;
logic fsm0_write_en;
logic fsm0_clk;
logic fsm0_reset;
logic [1:0] fsm0_out;
logic fsm0_done;
logic early_reset_static_par_go_in;
logic early_reset_static_par_go_out;
logic early_reset_static_par_done_in;
logic early_reset_static_par_done_out;
logic early_reset_static_par0_go_in;
logic early_reset_static_par0_go_out;
logic early_reset_static_par0_done_in;
logic early_reset_static_par0_done_out;
logic wrapper_early_reset_static_par_go_in;
logic wrapper_early_reset_static_par_go_out;
logic wrapper_early_reset_static_par_done_in;
logic wrapper_early_reset_static_par_done_out;
logic while_wrapper_early_reset_static_par0_go_in;
logic while_wrapper_early_reset_static_par0_go_out;
logic while_wrapper_early_reset_static_par0_done_in;
logic while_wrapper_early_reset_static_par0_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
std_reg # (
    .WIDTH(32)
) min_depth_4 (
    .clk(min_depth_4_clk),
    .done(min_depth_4_done),
    .in(min_depth_4_in),
    .out(min_depth_4_out),
    .reset(min_depth_4_reset),
    .write_en(min_depth_4_write_en)
);
std_reg # (
    .WIDTH(32)
) iter_limit (
    .clk(iter_limit_clk),
    .done(iter_limit_done),
    .in(iter_limit_in),
    .out(iter_limit_out),
    .reset(iter_limit_reset),
    .write_en(iter_limit_write_en)
);
std_add # (
    .WIDTH(32)
) depth_plus_4 (
    .left(depth_plus_4_left),
    .out(depth_plus_4_out),
    .right(depth_plus_4_right)
);
std_add # (
    .WIDTH(32)
) min_depth_4_plus_4 (
    .left(min_depth_4_plus_4_left),
    .out(min_depth_4_plus_4_out),
    .right(min_depth_4_plus_4_right)
);
std_add # (
    .WIDTH(32)
) depth_plus_8 (
    .left(depth_plus_8_left),
    .out(depth_plus_8_out),
    .right(depth_plus_8_right)
);
std_add # (
    .WIDTH(32)
) min_depth_4_plus_5 (
    .left(min_depth_4_plus_5_left),
    .out(min_depth_4_plus_5_out),
    .right(min_depth_4_plus_5_right)
);
std_add # (
    .WIDTH(32)
) min_depth_4_plus_2 (
    .left(min_depth_4_plus_2_left),
    .out(min_depth_4_plus_2_out),
    .right(min_depth_4_plus_2_right)
);
std_add # (
    .WIDTH(32)
) depth_plus_5 (
    .left(depth_plus_5_left),
    .out(depth_plus_5_out),
    .right(depth_plus_5_right)
);
std_add # (
    .WIDTH(32)
) depth_plus_7 (
    .left(depth_plus_7_left),
    .out(depth_plus_7_out),
    .right(depth_plus_7_right)
);
std_add # (
    .WIDTH(32)
) depth_plus_0 (
    .left(depth_plus_0_left),
    .out(depth_plus_0_out),
    .right(depth_plus_0_right)
);
std_add # (
    .WIDTH(32)
) depth_plus_9 (
    .left(depth_plus_9_left),
    .out(depth_plus_9_out),
    .right(depth_plus_9_right)
);
std_add # (
    .WIDTH(32)
) depth_plus_6 (
    .left(depth_plus_6_left),
    .out(depth_plus_6_out),
    .right(depth_plus_6_right)
);
std_add # (
    .WIDTH(32)
) depth_plus_1 (
    .left(depth_plus_1_left),
    .out(depth_plus_1_out),
    .right(depth_plus_1_right)
);
std_add # (
    .WIDTH(32)
) min_depth_4_plus_1 (
    .left(min_depth_4_plus_1_left),
    .out(min_depth_4_plus_1_out),
    .right(min_depth_4_plus_1_right)
);
std_add # (
    .WIDTH(32)
) depth_plus_3 (
    .left(depth_plus_3_left),
    .out(depth_plus_3_out),
    .right(depth_plus_3_right)
);
std_add # (
    .WIDTH(32)
) min_depth_4_plus_3 (
    .left(min_depth_4_plus_3_left),
    .out(min_depth_4_plus_3_out),
    .right(min_depth_4_plus_3_right)
);
std_add # (
    .WIDTH(32)
) depth_plus_2 (
    .left(depth_plus_2_left),
    .out(depth_plus_2_out),
    .right(depth_plus_2_right)
);
mac_pe pe_0_0 (
    .clk(pe_0_0_clk),
    .done(pe_0_0_done),
    .go(pe_0_0_go),
    .left(pe_0_0_left),
    .mul_ready(pe_0_0_mul_ready),
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
mac_pe pe_0_1 (
    .clk(pe_0_1_clk),
    .done(pe_0_1_done),
    .go(pe_0_1_go),
    .left(pe_0_1_left),
    .mul_ready(pe_0_1_mul_ready),
    .out(pe_0_1_out),
    .reset(pe_0_1_reset),
    .top(pe_0_1_top)
);
std_reg # (
    .WIDTH(32)
) top_0_1 (
    .clk(top_0_1_clk),
    .done(top_0_1_done),
    .in(top_0_1_in),
    .out(top_0_1_out),
    .reset(top_0_1_reset),
    .write_en(top_0_1_write_en)
);
std_reg # (
    .WIDTH(32)
) left_0_1 (
    .clk(left_0_1_clk),
    .done(left_0_1_done),
    .in(left_0_1_in),
    .out(left_0_1_out),
    .reset(left_0_1_reset),
    .write_en(left_0_1_write_en)
);
mac_pe pe_0_2 (
    .clk(pe_0_2_clk),
    .done(pe_0_2_done),
    .go(pe_0_2_go),
    .left(pe_0_2_left),
    .mul_ready(pe_0_2_mul_ready),
    .out(pe_0_2_out),
    .reset(pe_0_2_reset),
    .top(pe_0_2_top)
);
std_reg # (
    .WIDTH(32)
) top_0_2 (
    .clk(top_0_2_clk),
    .done(top_0_2_done),
    .in(top_0_2_in),
    .out(top_0_2_out),
    .reset(top_0_2_reset),
    .write_en(top_0_2_write_en)
);
std_reg # (
    .WIDTH(32)
) left_0_2 (
    .clk(left_0_2_clk),
    .done(left_0_2_done),
    .in(left_0_2_in),
    .out(left_0_2_out),
    .reset(left_0_2_reset),
    .write_en(left_0_2_write_en)
);
mac_pe pe_1_0 (
    .clk(pe_1_0_clk),
    .done(pe_1_0_done),
    .go(pe_1_0_go),
    .left(pe_1_0_left),
    .mul_ready(pe_1_0_mul_ready),
    .out(pe_1_0_out),
    .reset(pe_1_0_reset),
    .top(pe_1_0_top)
);
std_reg # (
    .WIDTH(32)
) top_1_0 (
    .clk(top_1_0_clk),
    .done(top_1_0_done),
    .in(top_1_0_in),
    .out(top_1_0_out),
    .reset(top_1_0_reset),
    .write_en(top_1_0_write_en)
);
std_reg # (
    .WIDTH(32)
) left_1_0 (
    .clk(left_1_0_clk),
    .done(left_1_0_done),
    .in(left_1_0_in),
    .out(left_1_0_out),
    .reset(left_1_0_reset),
    .write_en(left_1_0_write_en)
);
mac_pe pe_1_1 (
    .clk(pe_1_1_clk),
    .done(pe_1_1_done),
    .go(pe_1_1_go),
    .left(pe_1_1_left),
    .mul_ready(pe_1_1_mul_ready),
    .out(pe_1_1_out),
    .reset(pe_1_1_reset),
    .top(pe_1_1_top)
);
std_reg # (
    .WIDTH(32)
) top_1_1 (
    .clk(top_1_1_clk),
    .done(top_1_1_done),
    .in(top_1_1_in),
    .out(top_1_1_out),
    .reset(top_1_1_reset),
    .write_en(top_1_1_write_en)
);
std_reg # (
    .WIDTH(32)
) left_1_1 (
    .clk(left_1_1_clk),
    .done(left_1_1_done),
    .in(left_1_1_in),
    .out(left_1_1_out),
    .reset(left_1_1_reset),
    .write_en(left_1_1_write_en)
);
mac_pe pe_1_2 (
    .clk(pe_1_2_clk),
    .done(pe_1_2_done),
    .go(pe_1_2_go),
    .left(pe_1_2_left),
    .mul_ready(pe_1_2_mul_ready),
    .out(pe_1_2_out),
    .reset(pe_1_2_reset),
    .top(pe_1_2_top)
);
std_reg # (
    .WIDTH(32)
) top_1_2 (
    .clk(top_1_2_clk),
    .done(top_1_2_done),
    .in(top_1_2_in),
    .out(top_1_2_out),
    .reset(top_1_2_reset),
    .write_en(top_1_2_write_en)
);
std_reg # (
    .WIDTH(32)
) left_1_2 (
    .clk(left_1_2_clk),
    .done(left_1_2_done),
    .in(left_1_2_in),
    .out(left_1_2_out),
    .reset(left_1_2_reset),
    .write_en(left_1_2_write_en)
);
mac_pe pe_2_0 (
    .clk(pe_2_0_clk),
    .done(pe_2_0_done),
    .go(pe_2_0_go),
    .left(pe_2_0_left),
    .mul_ready(pe_2_0_mul_ready),
    .out(pe_2_0_out),
    .reset(pe_2_0_reset),
    .top(pe_2_0_top)
);
std_reg # (
    .WIDTH(32)
) top_2_0 (
    .clk(top_2_0_clk),
    .done(top_2_0_done),
    .in(top_2_0_in),
    .out(top_2_0_out),
    .reset(top_2_0_reset),
    .write_en(top_2_0_write_en)
);
std_reg # (
    .WIDTH(32)
) left_2_0 (
    .clk(left_2_0_clk),
    .done(left_2_0_done),
    .in(left_2_0_in),
    .out(left_2_0_out),
    .reset(left_2_0_reset),
    .write_en(left_2_0_write_en)
);
mac_pe pe_2_1 (
    .clk(pe_2_1_clk),
    .done(pe_2_1_done),
    .go(pe_2_1_go),
    .left(pe_2_1_left),
    .mul_ready(pe_2_1_mul_ready),
    .out(pe_2_1_out),
    .reset(pe_2_1_reset),
    .top(pe_2_1_top)
);
std_reg # (
    .WIDTH(32)
) top_2_1 (
    .clk(top_2_1_clk),
    .done(top_2_1_done),
    .in(top_2_1_in),
    .out(top_2_1_out),
    .reset(top_2_1_reset),
    .write_en(top_2_1_write_en)
);
std_reg # (
    .WIDTH(32)
) left_2_1 (
    .clk(left_2_1_clk),
    .done(left_2_1_done),
    .in(left_2_1_in),
    .out(left_2_1_out),
    .reset(left_2_1_reset),
    .write_en(left_2_1_write_en)
);
mac_pe pe_2_2 (
    .clk(pe_2_2_clk),
    .done(pe_2_2_done),
    .go(pe_2_2_go),
    .left(pe_2_2_left),
    .mul_ready(pe_2_2_mul_ready),
    .out(pe_2_2_out),
    .reset(pe_2_2_reset),
    .top(pe_2_2_top)
);
std_reg # (
    .WIDTH(32)
) top_2_2 (
    .clk(top_2_2_clk),
    .done(top_2_2_done),
    .in(top_2_2_in),
    .out(top_2_2_out),
    .reset(top_2_2_reset),
    .write_en(top_2_2_write_en)
);
std_reg # (
    .WIDTH(32)
) left_2_2 (
    .clk(left_2_2_clk),
    .done(left_2_2_done),
    .in(left_2_2_in),
    .out(left_2_2_out),
    .reset(left_2_2_reset),
    .write_en(left_2_2_write_en)
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
std_reg # (
    .WIDTH(2)
) t1_idx (
    .clk(t1_idx_clk),
    .done(t1_idx_done),
    .in(t1_idx_in),
    .out(t1_idx_out),
    .reset(t1_idx_reset),
    .write_en(t1_idx_write_en)
);
std_add # (
    .WIDTH(2)
) t1_add (
    .left(t1_add_left),
    .out(t1_add_out),
    .right(t1_add_right)
);
std_reg # (
    .WIDTH(2)
) t2_idx (
    .clk(t2_idx_clk),
    .done(t2_idx_done),
    .in(t2_idx_in),
    .out(t2_idx_out),
    .reset(t2_idx_reset),
    .write_en(t2_idx_write_en)
);
std_add # (
    .WIDTH(2)
) t2_add (
    .left(t2_add_left),
    .out(t2_add_out),
    .right(t2_add_right)
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
std_reg # (
    .WIDTH(2)
) l1_idx (
    .clk(l1_idx_clk),
    .done(l1_idx_done),
    .in(l1_idx_in),
    .out(l1_idx_out),
    .reset(l1_idx_reset),
    .write_en(l1_idx_write_en)
);
std_add # (
    .WIDTH(2)
) l1_add (
    .left(l1_add_left),
    .out(l1_add_out),
    .right(l1_add_right)
);
std_reg # (
    .WIDTH(2)
) l2_idx (
    .clk(l2_idx_clk),
    .done(l2_idx_done),
    .in(l2_idx_in),
    .out(l2_idx_out),
    .reset(l2_idx_reset),
    .write_en(l2_idx_write_en)
);
std_add # (
    .WIDTH(2)
) l2_add (
    .left(l2_add_left),
    .out(l2_add_out),
    .right(l2_add_right)
);
std_reg # (
    .WIDTH(32)
) idx (
    .clk(idx_clk),
    .done(idx_done),
    .in(idx_in),
    .out(idx_out),
    .reset(idx_reset),
    .write_en(idx_write_en)
);
std_add # (
    .WIDTH(32)
) idx_add (
    .left(idx_add_left),
    .out(idx_add_out),
    .right(idx_add_right)
);
std_lt # (
    .WIDTH(32)
) lt_iter_limit (
    .left(lt_iter_limit_left),
    .out(lt_iter_limit_out),
    .right(lt_iter_limit_right)
);
std_reg # (
    .WIDTH(1)
) cond_reg (
    .clk(cond_reg_clk),
    .done(cond_reg_done),
    .in(cond_reg_in),
    .out(cond_reg_out),
    .reset(cond_reg_reset),
    .write_en(cond_reg_write_en)
);
std_reg # (
    .WIDTH(1)
) idx_between_4_depth_plus_4_reg (
    .clk(idx_between_4_depth_plus_4_reg_clk),
    .done(idx_between_4_depth_plus_4_reg_done),
    .in(idx_between_4_depth_plus_4_reg_in),
    .out(idx_between_4_depth_plus_4_reg_out),
    .reset(idx_between_4_depth_plus_4_reg_reset),
    .write_en(idx_between_4_depth_plus_4_reg_write_en)
);
std_lt # (
    .WIDTH(32)
) index_lt_depth_plus_4 (
    .left(index_lt_depth_plus_4_left),
    .out(index_lt_depth_plus_4_out),
    .right(index_lt_depth_plus_4_right)
);
std_ge # (
    .WIDTH(32)
) index_ge_4 (
    .left(index_ge_4_left),
    .out(index_ge_4_out),
    .right(index_ge_4_right)
);
std_and # (
    .WIDTH(1)
) idx_between_4_depth_plus_4_comb (
    .left(idx_between_4_depth_plus_4_comb_left),
    .out(idx_between_4_depth_plus_4_comb_out),
    .right(idx_between_4_depth_plus_4_comb_right)
);
std_reg # (
    .WIDTH(1)
) idx_between_4_min_depth_4_plus_4_reg (
    .clk(idx_between_4_min_depth_4_plus_4_reg_clk),
    .done(idx_between_4_min_depth_4_plus_4_reg_done),
    .in(idx_between_4_min_depth_4_plus_4_reg_in),
    .out(idx_between_4_min_depth_4_plus_4_reg_out),
    .reset(idx_between_4_min_depth_4_plus_4_reg_reset),
    .write_en(idx_between_4_min_depth_4_plus_4_reg_write_en)
);
std_lt # (
    .WIDTH(32)
) index_lt_min_depth_4_plus_4 (
    .left(index_lt_min_depth_4_plus_4_left),
    .out(index_lt_min_depth_4_plus_4_out),
    .right(index_lt_min_depth_4_plus_4_right)
);
std_and # (
    .WIDTH(1)
) idx_between_4_min_depth_4_plus_4_comb (
    .left(idx_between_4_min_depth_4_plus_4_comb_left),
    .out(idx_between_4_min_depth_4_plus_4_comb_out),
    .right(idx_between_4_min_depth_4_plus_4_comb_right)
);
std_reg # (
    .WIDTH(1)
) idx_between_8_depth_plus_8_reg (
    .clk(idx_between_8_depth_plus_8_reg_clk),
    .done(idx_between_8_depth_plus_8_reg_done),
    .in(idx_between_8_depth_plus_8_reg_in),
    .out(idx_between_8_depth_plus_8_reg_out),
    .reset(idx_between_8_depth_plus_8_reg_reset),
    .write_en(idx_between_8_depth_plus_8_reg_write_en)
);
std_lt # (
    .WIDTH(32)
) index_lt_depth_plus_8 (
    .left(index_lt_depth_plus_8_left),
    .out(index_lt_depth_plus_8_out),
    .right(index_lt_depth_plus_8_right)
);
std_ge # (
    .WIDTH(32)
) index_ge_8 (
    .left(index_ge_8_left),
    .out(index_ge_8_out),
    .right(index_ge_8_right)
);
std_and # (
    .WIDTH(1)
) idx_between_8_depth_plus_8_comb (
    .left(idx_between_8_depth_plus_8_comb_left),
    .out(idx_between_8_depth_plus_8_comb_out),
    .right(idx_between_8_depth_plus_8_comb_right)
);
std_reg # (
    .WIDTH(1)
) idx_between_5_min_depth_4_plus_5_reg (
    .clk(idx_between_5_min_depth_4_plus_5_reg_clk),
    .done(idx_between_5_min_depth_4_plus_5_reg_done),
    .in(idx_between_5_min_depth_4_plus_5_reg_in),
    .out(idx_between_5_min_depth_4_plus_5_reg_out),
    .reset(idx_between_5_min_depth_4_plus_5_reg_reset),
    .write_en(idx_between_5_min_depth_4_plus_5_reg_write_en)
);
std_lt # (
    .WIDTH(32)
) index_lt_min_depth_4_plus_5 (
    .left(index_lt_min_depth_4_plus_5_left),
    .out(index_lt_min_depth_4_plus_5_out),
    .right(index_lt_min_depth_4_plus_5_right)
);
std_ge # (
    .WIDTH(32)
) index_ge_5 (
    .left(index_ge_5_left),
    .out(index_ge_5_out),
    .right(index_ge_5_right)
);
std_and # (
    .WIDTH(1)
) idx_between_5_min_depth_4_plus_5_comb (
    .left(idx_between_5_min_depth_4_plus_5_comb_left),
    .out(idx_between_5_min_depth_4_plus_5_comb_out),
    .right(idx_between_5_min_depth_4_plus_5_comb_right)
);
std_reg # (
    .WIDTH(1)
) idx_between_2_min_depth_4_plus_2_reg (
    .clk(idx_between_2_min_depth_4_plus_2_reg_clk),
    .done(idx_between_2_min_depth_4_plus_2_reg_done),
    .in(idx_between_2_min_depth_4_plus_2_reg_in),
    .out(idx_between_2_min_depth_4_plus_2_reg_out),
    .reset(idx_between_2_min_depth_4_plus_2_reg_reset),
    .write_en(idx_between_2_min_depth_4_plus_2_reg_write_en)
);
std_lt # (
    .WIDTH(32)
) index_lt_min_depth_4_plus_2 (
    .left(index_lt_min_depth_4_plus_2_left),
    .out(index_lt_min_depth_4_plus_2_out),
    .right(index_lt_min_depth_4_plus_2_right)
);
std_ge # (
    .WIDTH(32)
) index_ge_2 (
    .left(index_ge_2_left),
    .out(index_ge_2_out),
    .right(index_ge_2_right)
);
std_and # (
    .WIDTH(1)
) idx_between_2_min_depth_4_plus_2_comb (
    .left(idx_between_2_min_depth_4_plus_2_comb_left),
    .out(idx_between_2_min_depth_4_plus_2_comb_out),
    .right(idx_between_2_min_depth_4_plus_2_comb_right)
);
std_reg # (
    .WIDTH(1)
) idx_between_5_depth_plus_5_reg (
    .clk(idx_between_5_depth_plus_5_reg_clk),
    .done(idx_between_5_depth_plus_5_reg_done),
    .in(idx_between_5_depth_plus_5_reg_in),
    .out(idx_between_5_depth_plus_5_reg_out),
    .reset(idx_between_5_depth_plus_5_reg_reset),
    .write_en(idx_between_5_depth_plus_5_reg_write_en)
);
std_lt # (
    .WIDTH(32)
) index_lt_depth_plus_5 (
    .left(index_lt_depth_plus_5_left),
    .out(index_lt_depth_plus_5_out),
    .right(index_lt_depth_plus_5_right)
);
std_and # (
    .WIDTH(1)
) idx_between_5_depth_plus_5_comb (
    .left(idx_between_5_depth_plus_5_comb_left),
    .out(idx_between_5_depth_plus_5_comb_out),
    .right(idx_between_5_depth_plus_5_comb_right)
);
std_reg # (
    .WIDTH(1)
) idx_between_7_depth_plus_7_reg (
    .clk(idx_between_7_depth_plus_7_reg_clk),
    .done(idx_between_7_depth_plus_7_reg_done),
    .in(idx_between_7_depth_plus_7_reg_in),
    .out(idx_between_7_depth_plus_7_reg_out),
    .reset(idx_between_7_depth_plus_7_reg_reset),
    .write_en(idx_between_7_depth_plus_7_reg_write_en)
);
std_lt # (
    .WIDTH(32)
) index_lt_depth_plus_7 (
    .left(index_lt_depth_plus_7_left),
    .out(index_lt_depth_plus_7_out),
    .right(index_lt_depth_plus_7_right)
);
std_ge # (
    .WIDTH(32)
) index_ge_7 (
    .left(index_ge_7_left),
    .out(index_ge_7_out),
    .right(index_ge_7_right)
);
std_and # (
    .WIDTH(1)
) idx_between_7_depth_plus_7_comb (
    .left(idx_between_7_depth_plus_7_comb_left),
    .out(idx_between_7_depth_plus_7_comb_out),
    .right(idx_between_7_depth_plus_7_comb_right)
);
std_reg # (
    .WIDTH(1)
) idx_between_0_depth_plus_0_reg (
    .clk(idx_between_0_depth_plus_0_reg_clk),
    .done(idx_between_0_depth_plus_0_reg_done),
    .in(idx_between_0_depth_plus_0_reg_in),
    .out(idx_between_0_depth_plus_0_reg_out),
    .reset(idx_between_0_depth_plus_0_reg_reset),
    .write_en(idx_between_0_depth_plus_0_reg_write_en)
);
std_lt # (
    .WIDTH(32)
) index_lt_depth_plus_0 (
    .left(index_lt_depth_plus_0_left),
    .out(index_lt_depth_plus_0_out),
    .right(index_lt_depth_plus_0_right)
);
std_reg # (
    .WIDTH(1)
) idx_between_9_depth_plus_9_reg (
    .clk(idx_between_9_depth_plus_9_reg_clk),
    .done(idx_between_9_depth_plus_9_reg_done),
    .in(idx_between_9_depth_plus_9_reg_in),
    .out(idx_between_9_depth_plus_9_reg_out),
    .reset(idx_between_9_depth_plus_9_reg_reset),
    .write_en(idx_between_9_depth_plus_9_reg_write_en)
);
std_lt # (
    .WIDTH(32)
) index_lt_depth_plus_9 (
    .left(index_lt_depth_plus_9_left),
    .out(index_lt_depth_plus_9_out),
    .right(index_lt_depth_plus_9_right)
);
std_ge # (
    .WIDTH(32)
) index_ge_9 (
    .left(index_ge_9_left),
    .out(index_ge_9_out),
    .right(index_ge_9_right)
);
std_and # (
    .WIDTH(1)
) idx_between_9_depth_plus_9_comb (
    .left(idx_between_9_depth_plus_9_comb_left),
    .out(idx_between_9_depth_plus_9_comb_out),
    .right(idx_between_9_depth_plus_9_comb_right)
);
std_reg # (
    .WIDTH(1)
) idx_between_depth_plus_6_None_reg (
    .clk(idx_between_depth_plus_6_None_reg_clk),
    .done(idx_between_depth_plus_6_None_reg_done),
    .in(idx_between_depth_plus_6_None_reg_in),
    .out(idx_between_depth_plus_6_None_reg_out),
    .reset(idx_between_depth_plus_6_None_reg_reset),
    .write_en(idx_between_depth_plus_6_None_reg_write_en)
);
std_reg # (
    .WIDTH(1)
) idx_between_1_depth_plus_1_reg (
    .clk(idx_between_1_depth_plus_1_reg_clk),
    .done(idx_between_1_depth_plus_1_reg_done),
    .in(idx_between_1_depth_plus_1_reg_in),
    .out(idx_between_1_depth_plus_1_reg_out),
    .reset(idx_between_1_depth_plus_1_reg_reset),
    .write_en(idx_between_1_depth_plus_1_reg_write_en)
);
std_lt # (
    .WIDTH(32)
) index_lt_depth_plus_1 (
    .left(index_lt_depth_plus_1_left),
    .out(index_lt_depth_plus_1_out),
    .right(index_lt_depth_plus_1_right)
);
std_ge # (
    .WIDTH(32)
) index_ge_1 (
    .left(index_ge_1_left),
    .out(index_ge_1_out),
    .right(index_ge_1_right)
);
std_and # (
    .WIDTH(1)
) idx_between_1_depth_plus_1_comb (
    .left(idx_between_1_depth_plus_1_comb_left),
    .out(idx_between_1_depth_plus_1_comb_out),
    .right(idx_between_1_depth_plus_1_comb_right)
);
std_reg # (
    .WIDTH(1)
) idx_between_1_min_depth_4_plus_1_reg (
    .clk(idx_between_1_min_depth_4_plus_1_reg_clk),
    .done(idx_between_1_min_depth_4_plus_1_reg_done),
    .in(idx_between_1_min_depth_4_plus_1_reg_in),
    .out(idx_between_1_min_depth_4_plus_1_reg_out),
    .reset(idx_between_1_min_depth_4_plus_1_reg_reset),
    .write_en(idx_between_1_min_depth_4_plus_1_reg_write_en)
);
std_lt # (
    .WIDTH(32)
) index_lt_min_depth_4_plus_1 (
    .left(index_lt_min_depth_4_plus_1_left),
    .out(index_lt_min_depth_4_plus_1_out),
    .right(index_lt_min_depth_4_plus_1_right)
);
std_and # (
    .WIDTH(1)
) idx_between_1_min_depth_4_plus_1_comb (
    .left(idx_between_1_min_depth_4_plus_1_comb_left),
    .out(idx_between_1_min_depth_4_plus_1_comb_out),
    .right(idx_between_1_min_depth_4_plus_1_comb_right)
);
std_reg # (
    .WIDTH(1)
) idx_between_3_depth_plus_3_reg (
    .clk(idx_between_3_depth_plus_3_reg_clk),
    .done(idx_between_3_depth_plus_3_reg_done),
    .in(idx_between_3_depth_plus_3_reg_in),
    .out(idx_between_3_depth_plus_3_reg_out),
    .reset(idx_between_3_depth_plus_3_reg_reset),
    .write_en(idx_between_3_depth_plus_3_reg_write_en)
);
std_lt # (
    .WIDTH(32)
) index_lt_depth_plus_3 (
    .left(index_lt_depth_plus_3_left),
    .out(index_lt_depth_plus_3_out),
    .right(index_lt_depth_plus_3_right)
);
std_ge # (
    .WIDTH(32)
) index_ge_3 (
    .left(index_ge_3_left),
    .out(index_ge_3_out),
    .right(index_ge_3_right)
);
std_and # (
    .WIDTH(1)
) idx_between_3_depth_plus_3_comb (
    .left(idx_between_3_depth_plus_3_comb_left),
    .out(idx_between_3_depth_plus_3_comb_out),
    .right(idx_between_3_depth_plus_3_comb_right)
);
std_reg # (
    .WIDTH(1)
) idx_between_3_min_depth_4_plus_3_reg (
    .clk(idx_between_3_min_depth_4_plus_3_reg_clk),
    .done(idx_between_3_min_depth_4_plus_3_reg_done),
    .in(idx_between_3_min_depth_4_plus_3_reg_in),
    .out(idx_between_3_min_depth_4_plus_3_reg_out),
    .reset(idx_between_3_min_depth_4_plus_3_reg_reset),
    .write_en(idx_between_3_min_depth_4_plus_3_reg_write_en)
);
std_lt # (
    .WIDTH(32)
) index_lt_min_depth_4_plus_3 (
    .left(index_lt_min_depth_4_plus_3_left),
    .out(index_lt_min_depth_4_plus_3_out),
    .right(index_lt_min_depth_4_plus_3_right)
);
std_and # (
    .WIDTH(1)
) idx_between_3_min_depth_4_plus_3_comb (
    .left(idx_between_3_min_depth_4_plus_3_comb_left),
    .out(idx_between_3_min_depth_4_plus_3_comb_out),
    .right(idx_between_3_min_depth_4_plus_3_comb_right)
);
std_reg # (
    .WIDTH(1)
) idx_between_depth_plus_7_None_reg (
    .clk(idx_between_depth_plus_7_None_reg_clk),
    .done(idx_between_depth_plus_7_None_reg_done),
    .in(idx_between_depth_plus_7_None_reg_in),
    .out(idx_between_depth_plus_7_None_reg_out),
    .reset(idx_between_depth_plus_7_None_reg_reset),
    .write_en(idx_between_depth_plus_7_None_reg_write_en)
);
std_reg # (
    .WIDTH(1)
) idx_between_2_depth_plus_2_reg (
    .clk(idx_between_2_depth_plus_2_reg_clk),
    .done(idx_between_2_depth_plus_2_reg_done),
    .in(idx_between_2_depth_plus_2_reg_in),
    .out(idx_between_2_depth_plus_2_reg_out),
    .reset(idx_between_2_depth_plus_2_reg_reset),
    .write_en(idx_between_2_depth_plus_2_reg_write_en)
);
std_lt # (
    .WIDTH(32)
) index_lt_depth_plus_2 (
    .left(index_lt_depth_plus_2_left),
    .out(index_lt_depth_plus_2_out),
    .right(index_lt_depth_plus_2_right)
);
std_and # (
    .WIDTH(1)
) idx_between_2_depth_plus_2_comb (
    .left(idx_between_2_depth_plus_2_comb_left),
    .out(idx_between_2_depth_plus_2_comb_out),
    .right(idx_between_2_depth_plus_2_comb_right)
);
std_reg # (
    .WIDTH(1)
) idx_between_6_depth_plus_6_reg (
    .clk(idx_between_6_depth_plus_6_reg_clk),
    .done(idx_between_6_depth_plus_6_reg_done),
    .in(idx_between_6_depth_plus_6_reg_in),
    .out(idx_between_6_depth_plus_6_reg_out),
    .reset(idx_between_6_depth_plus_6_reg_reset),
    .write_en(idx_between_6_depth_plus_6_reg_write_en)
);
std_lt # (
    .WIDTH(32)
) index_lt_depth_plus_6 (
    .left(index_lt_depth_plus_6_left),
    .out(index_lt_depth_plus_6_out),
    .right(index_lt_depth_plus_6_right)
);
std_ge # (
    .WIDTH(32)
) index_ge_6 (
    .left(index_ge_6_left),
    .out(index_ge_6_out),
    .right(index_ge_6_right)
);
std_and # (
    .WIDTH(1)
) idx_between_6_depth_plus_6_comb (
    .left(idx_between_6_depth_plus_6_comb_left),
    .out(idx_between_6_depth_plus_6_comb_out),
    .right(idx_between_6_depth_plus_6_comb_right)
);
std_reg # (
    .WIDTH(1)
) idx_between_depth_plus_5_None_reg (
    .clk(idx_between_depth_plus_5_None_reg_clk),
    .done(idx_between_depth_plus_5_None_reg_done),
    .in(idx_between_depth_plus_5_None_reg_in),
    .out(idx_between_depth_plus_5_None_reg_out),
    .reset(idx_between_depth_plus_5_None_reg_reset),
    .write_en(idx_between_depth_plus_5_None_reg_write_en)
);
std_wire # (
    .WIDTH(32)
) relu_r0_cur_val (
    .in(relu_r0_cur_val_in),
    .out(relu_r0_cur_val_out)
);
std_reg # (
    .WIDTH(32)
) relu_r0_cur_idx (
    .clk(relu_r0_cur_idx_clk),
    .done(relu_r0_cur_idx_done),
    .in(relu_r0_cur_idx_in),
    .out(relu_r0_cur_idx_out),
    .reset(relu_r0_cur_idx_reset),
    .write_en(relu_r0_cur_idx_write_en)
);
std_fp_sgt # (
    .FRAC_WIDTH(16),
    .INT_WIDTH(16),
    .WIDTH(32)
) relu_r0_val_gt (
    .left(relu_r0_val_gt_left),
    .out(relu_r0_val_gt_out),
    .right(relu_r0_val_gt_right)
);
std_wire # (
    .WIDTH(32)
) relu_r0_go_next (
    .in(relu_r0_go_next_in),
    .out(relu_r0_go_next_out)
);
std_add # (
    .WIDTH(32)
) relu_r0_incr (
    .left(relu_r0_incr_left),
    .out(relu_r0_incr_out),
    .right(relu_r0_incr_right)
);
std_fp_smult_pipe # (
    .FRAC_WIDTH(16),
    .INT_WIDTH(16),
    .WIDTH(32)
) relu_r0_val_mult (
    .clk(relu_r0_val_mult_clk),
    .done(relu_r0_val_mult_done),
    .go(relu_r0_val_mult_go),
    .left(relu_r0_val_mult_left),
    .out(relu_r0_val_mult_out),
    .reset(relu_r0_val_mult_reset),
    .right(relu_r0_val_mult_right)
);
std_wire # (
    .WIDTH(32)
) relu_r1_cur_val (
    .in(relu_r1_cur_val_in),
    .out(relu_r1_cur_val_out)
);
std_reg # (
    .WIDTH(32)
) relu_r1_cur_idx (
    .clk(relu_r1_cur_idx_clk),
    .done(relu_r1_cur_idx_done),
    .in(relu_r1_cur_idx_in),
    .out(relu_r1_cur_idx_out),
    .reset(relu_r1_cur_idx_reset),
    .write_en(relu_r1_cur_idx_write_en)
);
std_fp_sgt # (
    .FRAC_WIDTH(16),
    .INT_WIDTH(16),
    .WIDTH(32)
) relu_r1_val_gt (
    .left(relu_r1_val_gt_left),
    .out(relu_r1_val_gt_out),
    .right(relu_r1_val_gt_right)
);
std_wire # (
    .WIDTH(32)
) relu_r1_go_next (
    .in(relu_r1_go_next_in),
    .out(relu_r1_go_next_out)
);
std_add # (
    .WIDTH(32)
) relu_r1_incr (
    .left(relu_r1_incr_left),
    .out(relu_r1_incr_out),
    .right(relu_r1_incr_right)
);
std_fp_smult_pipe # (
    .FRAC_WIDTH(16),
    .INT_WIDTH(16),
    .WIDTH(32)
) relu_r1_val_mult (
    .clk(relu_r1_val_mult_clk),
    .done(relu_r1_val_mult_done),
    .go(relu_r1_val_mult_go),
    .left(relu_r1_val_mult_left),
    .out(relu_r1_val_mult_out),
    .reset(relu_r1_val_mult_reset),
    .right(relu_r1_val_mult_right)
);
std_wire # (
    .WIDTH(32)
) relu_r2_cur_val (
    .in(relu_r2_cur_val_in),
    .out(relu_r2_cur_val_out)
);
std_reg # (
    .WIDTH(32)
) relu_r2_cur_idx (
    .clk(relu_r2_cur_idx_clk),
    .done(relu_r2_cur_idx_done),
    .in(relu_r2_cur_idx_in),
    .out(relu_r2_cur_idx_out),
    .reset(relu_r2_cur_idx_reset),
    .write_en(relu_r2_cur_idx_write_en)
);
std_fp_sgt # (
    .FRAC_WIDTH(16),
    .INT_WIDTH(16),
    .WIDTH(32)
) relu_r2_val_gt (
    .left(relu_r2_val_gt_left),
    .out(relu_r2_val_gt_out),
    .right(relu_r2_val_gt_right)
);
std_wire # (
    .WIDTH(32)
) relu_r2_go_next (
    .in(relu_r2_go_next_in),
    .out(relu_r2_go_next_out)
);
std_add # (
    .WIDTH(32)
) relu_r2_incr (
    .left(relu_r2_incr_left),
    .out(relu_r2_incr_out),
    .right(relu_r2_incr_right)
);
std_fp_smult_pipe # (
    .FRAC_WIDTH(16),
    .INT_WIDTH(16),
    .WIDTH(32)
) relu_r2_val_mult (
    .clk(relu_r2_val_mult_clk),
    .done(relu_r2_val_mult_done),
    .go(relu_r2_val_mult_go),
    .left(relu_r2_val_mult_left),
    .out(relu_r2_val_mult_out),
    .reset(relu_r2_val_mult_reset),
    .right(relu_r2_val_mult_right)
);
std_reg # (
    .WIDTH(1)
) cond (
    .clk(cond_clk),
    .done(cond_done),
    .in(cond_in),
    .out(cond_out),
    .reset(cond_reset),
    .write_en(cond_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire (
    .in(cond_wire_in),
    .out(cond_wire_out)
);
std_reg # (
    .WIDTH(1)
) cond0 (
    .clk(cond0_clk),
    .done(cond0_done),
    .in(cond0_in),
    .out(cond0_out),
    .reset(cond0_reset),
    .write_en(cond0_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire0 (
    .in(cond_wire0_in),
    .out(cond_wire0_out)
);
std_reg # (
    .WIDTH(1)
) cond1 (
    .clk(cond1_clk),
    .done(cond1_done),
    .in(cond1_in),
    .out(cond1_out),
    .reset(cond1_reset),
    .write_en(cond1_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire1 (
    .in(cond_wire1_in),
    .out(cond_wire1_out)
);
std_reg # (
    .WIDTH(1)
) cond2 (
    .clk(cond2_clk),
    .done(cond2_done),
    .in(cond2_in),
    .out(cond2_out),
    .reset(cond2_reset),
    .write_en(cond2_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire2 (
    .in(cond_wire2_in),
    .out(cond_wire2_out)
);
std_reg # (
    .WIDTH(1)
) cond3 (
    .clk(cond3_clk),
    .done(cond3_done),
    .in(cond3_in),
    .out(cond3_out),
    .reset(cond3_reset),
    .write_en(cond3_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire3 (
    .in(cond_wire3_in),
    .out(cond_wire3_out)
);
std_reg # (
    .WIDTH(1)
) cond4 (
    .clk(cond4_clk),
    .done(cond4_done),
    .in(cond4_in),
    .out(cond4_out),
    .reset(cond4_reset),
    .write_en(cond4_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire4 (
    .in(cond_wire4_in),
    .out(cond_wire4_out)
);
std_reg # (
    .WIDTH(1)
) cond5 (
    .clk(cond5_clk),
    .done(cond5_done),
    .in(cond5_in),
    .out(cond5_out),
    .reset(cond5_reset),
    .write_en(cond5_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire5 (
    .in(cond_wire5_in),
    .out(cond_wire5_out)
);
std_reg # (
    .WIDTH(1)
) cond6 (
    .clk(cond6_clk),
    .done(cond6_done),
    .in(cond6_in),
    .out(cond6_out),
    .reset(cond6_reset),
    .write_en(cond6_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire6 (
    .in(cond_wire6_in),
    .out(cond_wire6_out)
);
std_reg # (
    .WIDTH(1)
) cond7 (
    .clk(cond7_clk),
    .done(cond7_done),
    .in(cond7_in),
    .out(cond7_out),
    .reset(cond7_reset),
    .write_en(cond7_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire7 (
    .in(cond_wire7_in),
    .out(cond_wire7_out)
);
std_reg # (
    .WIDTH(1)
) cond8 (
    .clk(cond8_clk),
    .done(cond8_done),
    .in(cond8_in),
    .out(cond8_out),
    .reset(cond8_reset),
    .write_en(cond8_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire8 (
    .in(cond_wire8_in),
    .out(cond_wire8_out)
);
std_reg # (
    .WIDTH(1)
) cond9 (
    .clk(cond9_clk),
    .done(cond9_done),
    .in(cond9_in),
    .out(cond9_out),
    .reset(cond9_reset),
    .write_en(cond9_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire9 (
    .in(cond_wire9_in),
    .out(cond_wire9_out)
);
std_reg # (
    .WIDTH(1)
) cond10 (
    .clk(cond10_clk),
    .done(cond10_done),
    .in(cond10_in),
    .out(cond10_out),
    .reset(cond10_reset),
    .write_en(cond10_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire10 (
    .in(cond_wire10_in),
    .out(cond_wire10_out)
);
std_reg # (
    .WIDTH(1)
) cond11 (
    .clk(cond11_clk),
    .done(cond11_done),
    .in(cond11_in),
    .out(cond11_out),
    .reset(cond11_reset),
    .write_en(cond11_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire11 (
    .in(cond_wire11_in),
    .out(cond_wire11_out)
);
std_reg # (
    .WIDTH(1)
) cond12 (
    .clk(cond12_clk),
    .done(cond12_done),
    .in(cond12_in),
    .out(cond12_out),
    .reset(cond12_reset),
    .write_en(cond12_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire12 (
    .in(cond_wire12_in),
    .out(cond_wire12_out)
);
std_reg # (
    .WIDTH(1)
) cond13 (
    .clk(cond13_clk),
    .done(cond13_done),
    .in(cond13_in),
    .out(cond13_out),
    .reset(cond13_reset),
    .write_en(cond13_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire13 (
    .in(cond_wire13_in),
    .out(cond_wire13_out)
);
std_reg # (
    .WIDTH(1)
) cond14 (
    .clk(cond14_clk),
    .done(cond14_done),
    .in(cond14_in),
    .out(cond14_out),
    .reset(cond14_reset),
    .write_en(cond14_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire14 (
    .in(cond_wire14_in),
    .out(cond_wire14_out)
);
std_reg # (
    .WIDTH(1)
) cond15 (
    .clk(cond15_clk),
    .done(cond15_done),
    .in(cond15_in),
    .out(cond15_out),
    .reset(cond15_reset),
    .write_en(cond15_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire15 (
    .in(cond_wire15_in),
    .out(cond_wire15_out)
);
std_reg # (
    .WIDTH(1)
) cond16 (
    .clk(cond16_clk),
    .done(cond16_done),
    .in(cond16_in),
    .out(cond16_out),
    .reset(cond16_reset),
    .write_en(cond16_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire16 (
    .in(cond_wire16_in),
    .out(cond_wire16_out)
);
std_reg # (
    .WIDTH(1)
) cond17 (
    .clk(cond17_clk),
    .done(cond17_done),
    .in(cond17_in),
    .out(cond17_out),
    .reset(cond17_reset),
    .write_en(cond17_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire17 (
    .in(cond_wire17_in),
    .out(cond_wire17_out)
);
std_reg # (
    .WIDTH(1)
) cond18 (
    .clk(cond18_clk),
    .done(cond18_done),
    .in(cond18_in),
    .out(cond18_out),
    .reset(cond18_reset),
    .write_en(cond18_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire18 (
    .in(cond_wire18_in),
    .out(cond_wire18_out)
);
std_reg # (
    .WIDTH(1)
) cond19 (
    .clk(cond19_clk),
    .done(cond19_done),
    .in(cond19_in),
    .out(cond19_out),
    .reset(cond19_reset),
    .write_en(cond19_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire19 (
    .in(cond_wire19_in),
    .out(cond_wire19_out)
);
std_reg # (
    .WIDTH(1)
) cond20 (
    .clk(cond20_clk),
    .done(cond20_done),
    .in(cond20_in),
    .out(cond20_out),
    .reset(cond20_reset),
    .write_en(cond20_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire20 (
    .in(cond_wire20_in),
    .out(cond_wire20_out)
);
std_reg # (
    .WIDTH(1)
) cond21 (
    .clk(cond21_clk),
    .done(cond21_done),
    .in(cond21_in),
    .out(cond21_out),
    .reset(cond21_reset),
    .write_en(cond21_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire21 (
    .in(cond_wire21_in),
    .out(cond_wire21_out)
);
std_reg # (
    .WIDTH(1)
) cond22 (
    .clk(cond22_clk),
    .done(cond22_done),
    .in(cond22_in),
    .out(cond22_out),
    .reset(cond22_reset),
    .write_en(cond22_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire22 (
    .in(cond_wire22_in),
    .out(cond_wire22_out)
);
std_reg # (
    .WIDTH(1)
) cond23 (
    .clk(cond23_clk),
    .done(cond23_done),
    .in(cond23_in),
    .out(cond23_out),
    .reset(cond23_reset),
    .write_en(cond23_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire23 (
    .in(cond_wire23_in),
    .out(cond_wire23_out)
);
std_reg # (
    .WIDTH(1)
) cond24 (
    .clk(cond24_clk),
    .done(cond24_done),
    .in(cond24_in),
    .out(cond24_out),
    .reset(cond24_reset),
    .write_en(cond24_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire24 (
    .in(cond_wire24_in),
    .out(cond_wire24_out)
);
std_reg # (
    .WIDTH(1)
) cond25 (
    .clk(cond25_clk),
    .done(cond25_done),
    .in(cond25_in),
    .out(cond25_out),
    .reset(cond25_reset),
    .write_en(cond25_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire25 (
    .in(cond_wire25_in),
    .out(cond_wire25_out)
);
std_reg # (
    .WIDTH(1)
) cond26 (
    .clk(cond26_clk),
    .done(cond26_done),
    .in(cond26_in),
    .out(cond26_out),
    .reset(cond26_reset),
    .write_en(cond26_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire26 (
    .in(cond_wire26_in),
    .out(cond_wire26_out)
);
std_reg # (
    .WIDTH(1)
) cond27 (
    .clk(cond27_clk),
    .done(cond27_done),
    .in(cond27_in),
    .out(cond27_out),
    .reset(cond27_reset),
    .write_en(cond27_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire27 (
    .in(cond_wire27_in),
    .out(cond_wire27_out)
);
std_reg # (
    .WIDTH(1)
) cond28 (
    .clk(cond28_clk),
    .done(cond28_done),
    .in(cond28_in),
    .out(cond28_out),
    .reset(cond28_reset),
    .write_en(cond28_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire28 (
    .in(cond_wire28_in),
    .out(cond_wire28_out)
);
std_reg # (
    .WIDTH(1)
) cond29 (
    .clk(cond29_clk),
    .done(cond29_done),
    .in(cond29_in),
    .out(cond29_out),
    .reset(cond29_reset),
    .write_en(cond29_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire29 (
    .in(cond_wire29_in),
    .out(cond_wire29_out)
);
std_reg # (
    .WIDTH(1)
) cond30 (
    .clk(cond30_clk),
    .done(cond30_done),
    .in(cond30_in),
    .out(cond30_out),
    .reset(cond30_reset),
    .write_en(cond30_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire30 (
    .in(cond_wire30_in),
    .out(cond_wire30_out)
);
std_reg # (
    .WIDTH(1)
) cond31 (
    .clk(cond31_clk),
    .done(cond31_done),
    .in(cond31_in),
    .out(cond31_out),
    .reset(cond31_reset),
    .write_en(cond31_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire31 (
    .in(cond_wire31_in),
    .out(cond_wire31_out)
);
std_reg # (
    .WIDTH(1)
) cond32 (
    .clk(cond32_clk),
    .done(cond32_done),
    .in(cond32_in),
    .out(cond32_out),
    .reset(cond32_reset),
    .write_en(cond32_write_en)
);
std_wire # (
    .WIDTH(1)
) cond_wire32 (
    .in(cond_wire32_in),
    .out(cond_wire32_out)
);
std_reg # (
    .WIDTH(1)
) fsm (
    .clk(fsm_clk),
    .done(fsm_done),
    .in(fsm_in),
    .out(fsm_out),
    .reset(fsm_reset),
    .write_en(fsm_write_en)
);
undef # (
    .WIDTH(1)
) ud (
    .out(ud_out)
);
std_add # (
    .WIDTH(1)
) adder (
    .left(adder_left),
    .out(adder_out),
    .right(adder_right)
);
undef # (
    .WIDTH(1)
) ud0 (
    .out(ud0_out)
);
std_add # (
    .WIDTH(1)
) adder0 (
    .left(adder0_left),
    .out(adder0_out),
    .right(adder0_right)
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
) fsm0 (
    .clk(fsm0_clk),
    .done(fsm0_done),
    .in(fsm0_in),
    .out(fsm0_out),
    .reset(fsm0_reset),
    .write_en(fsm0_write_en)
);
std_wire # (
    .WIDTH(1)
) early_reset_static_par_go (
    .in(early_reset_static_par_go_in),
    .out(early_reset_static_par_go_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_static_par_done (
    .in(early_reset_static_par_done_in),
    .out(early_reset_static_par_done_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_static_par0_go (
    .in(early_reset_static_par0_go_in),
    .out(early_reset_static_par0_go_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_static_par0_done (
    .in(early_reset_static_par0_done_in),
    .out(early_reset_static_par0_done_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_static_par_go (
    .in(wrapper_early_reset_static_par_go_in),
    .out(wrapper_early_reset_static_par_go_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_static_par_done (
    .in(wrapper_early_reset_static_par_done_in),
    .out(wrapper_early_reset_static_par_done_out)
);
std_wire # (
    .WIDTH(1)
) while_wrapper_early_reset_static_par0_go (
    .in(while_wrapper_early_reset_static_par0_go_in),
    .out(while_wrapper_early_reset_static_par0_go_out)
);
std_wire # (
    .WIDTH(1)
) while_wrapper_early_reset_static_par0_done (
    .in(while_wrapper_early_reset_static_par0_done_in),
    .out(while_wrapper_early_reset_static_par0_done_out)
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
wire _guard1 = early_reset_static_par0_go_out;
wire _guard2 = early_reset_static_par0_go_out;
wire _guard3 = cond_wire11_out;
wire _guard4 = early_reset_static_par0_go_out;
wire _guard5 = _guard3 & _guard4;
wire _guard6 = cond_wire11_out;
wire _guard7 = early_reset_static_par0_go_out;
wire _guard8 = _guard6 & _guard7;
wire _guard9 = ~_guard0;
wire _guard10 = early_reset_static_par0_go_out;
wire _guard11 = _guard9 & _guard10;
wire _guard12 = early_reset_static_par0_go_out;
wire _guard13 = early_reset_static_par0_go_out;
wire _guard14 = ~_guard0;
wire _guard15 = early_reset_static_par0_go_out;
wire _guard16 = _guard14 & _guard15;
wire _guard17 = early_reset_static_par_go_out;
wire _guard18 = early_reset_static_par_go_out;
wire _guard19 = early_reset_static_par_go_out;
wire _guard20 = early_reset_static_par0_go_out;
wire _guard21 = _guard19 | _guard20;
wire _guard22 = fsm_out != 1'd0;
wire _guard23 = early_reset_static_par_go_out;
wire _guard24 = _guard22 & _guard23;
wire _guard25 = fsm_out == 1'd0;
wire _guard26 = early_reset_static_par_go_out;
wire _guard27 = _guard25 & _guard26;
wire _guard28 = fsm_out == 1'd0;
wire _guard29 = early_reset_static_par0_go_out;
wire _guard30 = _guard28 & _guard29;
wire _guard31 = _guard27 | _guard30;
wire _guard32 = fsm_out != 1'd0;
wire _guard33 = early_reset_static_par0_go_out;
wire _guard34 = _guard32 & _guard33;
wire _guard35 = cond_wire1_out;
wire _guard36 = early_reset_static_par0_go_out;
wire _guard37 = _guard35 & _guard36;
wire _guard38 = cond_wire1_out;
wire _guard39 = early_reset_static_par0_go_out;
wire _guard40 = _guard38 & _guard39;
wire _guard41 = cond_wire20_out;
wire _guard42 = early_reset_static_par0_go_out;
wire _guard43 = _guard41 & _guard42;
wire _guard44 = cond_wire18_out;
wire _guard45 = early_reset_static_par0_go_out;
wire _guard46 = _guard44 & _guard45;
wire _guard47 = fsm_out == 1'd0;
wire _guard48 = cond_wire18_out;
wire _guard49 = _guard47 & _guard48;
wire _guard50 = fsm_out == 1'd0;
wire _guard51 = _guard49 & _guard50;
wire _guard52 = fsm_out == 1'd0;
wire _guard53 = cond_wire20_out;
wire _guard54 = _guard52 & _guard53;
wire _guard55 = fsm_out == 1'd0;
wire _guard56 = _guard54 & _guard55;
wire _guard57 = _guard51 | _guard56;
wire _guard58 = early_reset_static_par0_go_out;
wire _guard59 = _guard57 & _guard58;
wire _guard60 = fsm_out == 1'd0;
wire _guard61 = cond_wire18_out;
wire _guard62 = _guard60 & _guard61;
wire _guard63 = fsm_out == 1'd0;
wire _guard64 = _guard62 & _guard63;
wire _guard65 = fsm_out == 1'd0;
wire _guard66 = cond_wire20_out;
wire _guard67 = _guard65 & _guard66;
wire _guard68 = fsm_out == 1'd0;
wire _guard69 = _guard67 & _guard68;
wire _guard70 = _guard64 | _guard69;
wire _guard71 = early_reset_static_par0_go_out;
wire _guard72 = _guard70 & _guard71;
wire _guard73 = fsm_out == 1'd0;
wire _guard74 = cond_wire18_out;
wire _guard75 = _guard73 & _guard74;
wire _guard76 = fsm_out == 1'd0;
wire _guard77 = _guard75 & _guard76;
wire _guard78 = fsm_out == 1'd0;
wire _guard79 = cond_wire20_out;
wire _guard80 = _guard78 & _guard79;
wire _guard81 = fsm_out == 1'd0;
wire _guard82 = _guard80 & _guard81;
wire _guard83 = _guard77 | _guard82;
wire _guard84 = early_reset_static_par0_go_out;
wire _guard85 = _guard83 & _guard84;
wire _guard86 = early_reset_static_par0_go_out;
wire _guard87 = early_reset_static_par0_go_out;
wire _guard88 = early_reset_static_par0_go_out;
wire _guard89 = early_reset_static_par0_go_out;
wire _guard90 = early_reset_static_par0_go_out;
wire _guard91 = early_reset_static_par0_go_out;
wire _guard92 = early_reset_static_par0_go_out;
wire _guard93 = early_reset_static_par0_go_out;
wire _guard94 = early_reset_static_par0_go_out;
wire _guard95 = early_reset_static_par0_go_out;
wire _guard96 = early_reset_static_par_go_out;
wire _guard97 = early_reset_static_par0_go_out;
wire _guard98 = _guard96 | _guard97;
wire _guard99 = early_reset_static_par0_go_out;
wire _guard100 = early_reset_static_par_go_out;
wire _guard101 = relu_r1_go_next_out;
wire _guard102 = cond_wire31_out;
wire _guard103 = _guard101 & _guard102;
wire _guard104 = early_reset_static_par0_go_out;
wire _guard105 = _guard103 & _guard104;
wire _guard106 = relu_r1_go_next_out;
wire _guard107 = cond_wire31_out;
wire _guard108 = _guard106 & _guard107;
wire _guard109 = early_reset_static_par0_go_out;
wire _guard110 = _guard108 & _guard109;
wire _guard111 = relu_r2_val_mult_done;
wire _guard112 = relu_r2_val_gt_out;
wire _guard113 = _guard111 | _guard112;
wire _guard114 = cond_wire32_out;
wire _guard115 = _guard113 & _guard114;
wire _guard116 = early_reset_static_par0_go_out;
wire _guard117 = _guard115 & _guard116;
wire _guard118 = early_reset_static_par0_go_out;
wire _guard119 = ~_guard0;
wire _guard120 = early_reset_static_par0_go_out;
wire _guard121 = _guard119 & _guard120;
wire _guard122 = ~_guard0;
wire _guard123 = early_reset_static_par0_go_out;
wire _guard124 = _guard122 & _guard123;
wire _guard125 = early_reset_static_par0_go_out;
wire _guard126 = early_reset_static_par0_go_out;
wire _guard127 = early_reset_static_par0_go_out;
wire _guard128 = ~_guard0;
wire _guard129 = early_reset_static_par0_go_out;
wire _guard130 = _guard128 & _guard129;
wire _guard131 = early_reset_static_par0_go_out;
wire _guard132 = ~_guard0;
wire _guard133 = early_reset_static_par0_go_out;
wire _guard134 = _guard132 & _guard133;
wire _guard135 = early_reset_static_par0_go_out;
wire _guard136 = early_reset_static_par0_go_out;
wire _guard137 = early_reset_static_par0_go_out;
wire _guard138 = early_reset_static_par0_go_out;
wire _guard139 = ~_guard0;
wire _guard140 = early_reset_static_par0_go_out;
wire _guard141 = _guard139 & _guard140;
wire _guard142 = while_wrapper_early_reset_static_par0_go_out;
wire _guard143 = tdcc_done_out;
wire _guard144 = cond_wire7_out;
wire _guard145 = early_reset_static_par0_go_out;
wire _guard146 = _guard144 & _guard145;
wire _guard147 = cond_wire_out;
wire _guard148 = early_reset_static_par0_go_out;
wire _guard149 = _guard147 & _guard148;
wire _guard150 = cond_wire31_out;
wire _guard151 = early_reset_static_par0_go_out;
wire _guard152 = _guard150 & _guard151;
wire _guard153 = relu_r0_val_gt_out;
wire _guard154 = ~_guard153;
wire _guard155 = cond_wire30_out;
wire _guard156 = _guard154 & _guard155;
wire _guard157 = early_reset_static_par0_go_out;
wire _guard158 = _guard156 & _guard157;
wire _guard159 = relu_r0_val_gt_out;
wire _guard160 = cond_wire30_out;
wire _guard161 = _guard159 & _guard160;
wire _guard162 = early_reset_static_par0_go_out;
wire _guard163 = _guard161 & _guard162;
wire _guard164 = cond_wire11_out;
wire _guard165 = early_reset_static_par0_go_out;
wire _guard166 = _guard164 & _guard165;
wire _guard167 = relu_r2_val_gt_out;
wire _guard168 = ~_guard167;
wire _guard169 = cond_wire32_out;
wire _guard170 = _guard168 & _guard169;
wire _guard171 = early_reset_static_par0_go_out;
wire _guard172 = _guard170 & _guard171;
wire _guard173 = relu_r2_val_gt_out;
wire _guard174 = cond_wire32_out;
wire _guard175 = _guard173 & _guard174;
wire _guard176 = early_reset_static_par0_go_out;
wire _guard177 = _guard175 & _guard176;
wire _guard178 = relu_r1_val_gt_out;
wire _guard179 = cond_wire31_out;
wire _guard180 = _guard178 & _guard179;
wire _guard181 = early_reset_static_par0_go_out;
wire _guard182 = _guard180 & _guard181;
wire _guard183 = relu_r1_val_gt_out;
wire _guard184 = ~_guard183;
wire _guard185 = cond_wire31_out;
wire _guard186 = _guard184 & _guard185;
wire _guard187 = early_reset_static_par0_go_out;
wire _guard188 = _guard186 & _guard187;
wire _guard189 = relu_r1_go_next_out;
wire _guard190 = cond_wire31_out;
wire _guard191 = _guard189 & _guard190;
wire _guard192 = early_reset_static_par0_go_out;
wire _guard193 = _guard191 & _guard192;
wire _guard194 = cond_wire_out;
wire _guard195 = early_reset_static_par0_go_out;
wire _guard196 = _guard194 & _guard195;
wire _guard197 = cond_wire3_out;
wire _guard198 = early_reset_static_par0_go_out;
wire _guard199 = _guard197 & _guard198;
wire _guard200 = relu_r0_go_next_out;
wire _guard201 = cond_wire30_out;
wire _guard202 = _guard200 & _guard201;
wire _guard203 = early_reset_static_par0_go_out;
wire _guard204 = _guard202 & _guard203;
wire _guard205 = relu_r2_go_next_out;
wire _guard206 = cond_wire32_out;
wire _guard207 = _guard205 & _guard206;
wire _guard208 = early_reset_static_par0_go_out;
wire _guard209 = _guard207 & _guard208;
wire _guard210 = cond_wire21_out;
wire _guard211 = early_reset_static_par0_go_out;
wire _guard212 = _guard210 & _guard211;
wire _guard213 = cond_wire30_out;
wire _guard214 = early_reset_static_par0_go_out;
wire _guard215 = _guard213 & _guard214;
wire _guard216 = cond_wire32_out;
wire _guard217 = early_reset_static_par0_go_out;
wire _guard218 = _guard216 & _guard217;
wire _guard219 = early_reset_static_par0_go_out;
wire _guard220 = early_reset_static_par_go_out;
wire _guard221 = early_reset_static_par_go_out;
wire _guard222 = early_reset_static_par0_go_out;
wire _guard223 = early_reset_static_par_go_out;
wire _guard224 = cond_wire_out;
wire _guard225 = early_reset_static_par0_go_out;
wire _guard226 = _guard224 & _guard225;
wire _guard227 = _guard223 | _guard226;
wire _guard228 = early_reset_static_par_go_out;
wire _guard229 = cond_wire_out;
wire _guard230 = early_reset_static_par0_go_out;
wire _guard231 = _guard229 & _guard230;
wire _guard232 = early_reset_static_par_go_out;
wire _guard233 = early_reset_static_par0_go_out;
wire _guard234 = _guard232 | _guard233;
wire _guard235 = early_reset_static_par_go_out;
wire _guard236 = early_reset_static_par0_go_out;
wire _guard237 = early_reset_static_par0_go_out;
wire _guard238 = early_reset_static_par0_go_out;
wire _guard239 = cond_wire32_out;
wire _guard240 = early_reset_static_par0_go_out;
wire _guard241 = _guard239 & _guard240;
wire _guard242 = cond_wire32_out;
wire _guard243 = early_reset_static_par0_go_out;
wire _guard244 = _guard242 & _guard243;
wire _guard245 = early_reset_static_par0_go_out;
wire _guard246 = early_reset_static_par0_go_out;
wire _guard247 = early_reset_static_par0_go_out;
wire _guard248 = early_reset_static_par0_go_out;
wire _guard249 = ~_guard0;
wire _guard250 = early_reset_static_par0_go_out;
wire _guard251 = _guard249 & _guard250;
wire _guard252 = early_reset_static_par0_go_out;
wire _guard253 = early_reset_static_par0_go_out;
wire _guard254 = early_reset_static_par0_go_out;
wire _guard255 = cond_wire6_out;
wire _guard256 = early_reset_static_par0_go_out;
wire _guard257 = _guard255 & _guard256;
wire _guard258 = cond_wire4_out;
wire _guard259 = early_reset_static_par0_go_out;
wire _guard260 = _guard258 & _guard259;
wire _guard261 = fsm_out == 1'd0;
wire _guard262 = cond_wire4_out;
wire _guard263 = _guard261 & _guard262;
wire _guard264 = fsm_out == 1'd0;
wire _guard265 = _guard263 & _guard264;
wire _guard266 = fsm_out == 1'd0;
wire _guard267 = cond_wire6_out;
wire _guard268 = _guard266 & _guard267;
wire _guard269 = fsm_out == 1'd0;
wire _guard270 = _guard268 & _guard269;
wire _guard271 = _guard265 | _guard270;
wire _guard272 = early_reset_static_par0_go_out;
wire _guard273 = _guard271 & _guard272;
wire _guard274 = fsm_out == 1'd0;
wire _guard275 = cond_wire4_out;
wire _guard276 = _guard274 & _guard275;
wire _guard277 = fsm_out == 1'd0;
wire _guard278 = _guard276 & _guard277;
wire _guard279 = fsm_out == 1'd0;
wire _guard280 = cond_wire6_out;
wire _guard281 = _guard279 & _guard280;
wire _guard282 = fsm_out == 1'd0;
wire _guard283 = _guard281 & _guard282;
wire _guard284 = _guard278 | _guard283;
wire _guard285 = early_reset_static_par0_go_out;
wire _guard286 = _guard284 & _guard285;
wire _guard287 = fsm_out == 1'd0;
wire _guard288 = cond_wire4_out;
wire _guard289 = _guard287 & _guard288;
wire _guard290 = fsm_out == 1'd0;
wire _guard291 = _guard289 & _guard290;
wire _guard292 = fsm_out == 1'd0;
wire _guard293 = cond_wire6_out;
wire _guard294 = _guard292 & _guard293;
wire _guard295 = fsm_out == 1'd0;
wire _guard296 = _guard294 & _guard295;
wire _guard297 = _guard291 | _guard296;
wire _guard298 = early_reset_static_par0_go_out;
wire _guard299 = _guard297 & _guard298;
wire _guard300 = cond_wire16_out;
wire _guard301 = early_reset_static_par0_go_out;
wire _guard302 = _guard300 & _guard301;
wire _guard303 = cond_wire16_out;
wire _guard304 = early_reset_static_par0_go_out;
wire _guard305 = _guard303 & _guard304;
wire _guard306 = cond_wire26_out;
wire _guard307 = early_reset_static_par0_go_out;
wire _guard308 = _guard306 & _guard307;
wire _guard309 = cond_wire26_out;
wire _guard310 = early_reset_static_par0_go_out;
wire _guard311 = _guard309 & _guard310;
wire _guard312 = early_reset_static_par0_go_out;
wire _guard313 = early_reset_static_par0_go_out;
wire _guard314 = early_reset_static_par0_go_out;
wire _guard315 = early_reset_static_par0_go_out;
wire _guard316 = early_reset_static_par0_go_out;
wire _guard317 = early_reset_static_par0_go_out;
wire _guard318 = cond_wire32_out;
wire _guard319 = early_reset_static_par0_go_out;
wire _guard320 = _guard318 & _guard319;
wire _guard321 = relu_r2_go_next_out;
wire _guard322 = ~_guard321;
wire _guard323 = cond_wire32_out;
wire _guard324 = _guard322 & _guard323;
wire _guard325 = early_reset_static_par0_go_out;
wire _guard326 = _guard324 & _guard325;
wire _guard327 = cond_wire32_out;
wire _guard328 = early_reset_static_par0_go_out;
wire _guard329 = _guard327 & _guard328;
wire _guard330 = early_reset_static_par0_go_out;
wire _guard331 = ~_guard0;
wire _guard332 = early_reset_static_par0_go_out;
wire _guard333 = _guard331 & _guard332;
wire _guard334 = early_reset_static_par0_go_out;
wire _guard335 = early_reset_static_par0_go_out;
wire _guard336 = cond_wire14_out;
wire _guard337 = early_reset_static_par0_go_out;
wire _guard338 = _guard336 & _guard337;
wire _guard339 = cond_wire12_out;
wire _guard340 = early_reset_static_par0_go_out;
wire _guard341 = _guard339 & _guard340;
wire _guard342 = fsm_out == 1'd0;
wire _guard343 = cond_wire12_out;
wire _guard344 = _guard342 & _guard343;
wire _guard345 = fsm_out == 1'd0;
wire _guard346 = _guard344 & _guard345;
wire _guard347 = fsm_out == 1'd0;
wire _guard348 = cond_wire14_out;
wire _guard349 = _guard347 & _guard348;
wire _guard350 = fsm_out == 1'd0;
wire _guard351 = _guard349 & _guard350;
wire _guard352 = _guard346 | _guard351;
wire _guard353 = early_reset_static_par0_go_out;
wire _guard354 = _guard352 & _guard353;
wire _guard355 = fsm_out == 1'd0;
wire _guard356 = cond_wire12_out;
wire _guard357 = _guard355 & _guard356;
wire _guard358 = fsm_out == 1'd0;
wire _guard359 = _guard357 & _guard358;
wire _guard360 = fsm_out == 1'd0;
wire _guard361 = cond_wire14_out;
wire _guard362 = _guard360 & _guard361;
wire _guard363 = fsm_out == 1'd0;
wire _guard364 = _guard362 & _guard363;
wire _guard365 = _guard359 | _guard364;
wire _guard366 = early_reset_static_par0_go_out;
wire _guard367 = _guard365 & _guard366;
wire _guard368 = fsm_out == 1'd0;
wire _guard369 = cond_wire12_out;
wire _guard370 = _guard368 & _guard369;
wire _guard371 = fsm_out == 1'd0;
wire _guard372 = _guard370 & _guard371;
wire _guard373 = fsm_out == 1'd0;
wire _guard374 = cond_wire14_out;
wire _guard375 = _guard373 & _guard374;
wire _guard376 = fsm_out == 1'd0;
wire _guard377 = _guard375 & _guard376;
wire _guard378 = _guard372 | _guard377;
wire _guard379 = early_reset_static_par0_go_out;
wire _guard380 = _guard378 & _guard379;
wire _guard381 = cond_wire11_out;
wire _guard382 = early_reset_static_par0_go_out;
wire _guard383 = _guard381 & _guard382;
wire _guard384 = cond_wire11_out;
wire _guard385 = early_reset_static_par0_go_out;
wire _guard386 = _guard384 & _guard385;
wire _guard387 = cond_wire13_out;
wire _guard388 = early_reset_static_par0_go_out;
wire _guard389 = _guard387 & _guard388;
wire _guard390 = cond_wire13_out;
wire _guard391 = early_reset_static_par0_go_out;
wire _guard392 = _guard390 & _guard391;
wire _guard393 = cond_wire9_out;
wire _guard394 = early_reset_static_par0_go_out;
wire _guard395 = _guard393 & _guard394;
wire _guard396 = cond_wire9_out;
wire _guard397 = early_reset_static_par0_go_out;
wire _guard398 = _guard396 & _guard397;
wire _guard399 = cond_wire21_out;
wire _guard400 = early_reset_static_par0_go_out;
wire _guard401 = _guard399 & _guard400;
wire _guard402 = cond_wire21_out;
wire _guard403 = early_reset_static_par0_go_out;
wire _guard404 = _guard402 & _guard403;
wire _guard405 = early_reset_static_par_go_out;
wire _guard406 = cond_wire3_out;
wire _guard407 = early_reset_static_par0_go_out;
wire _guard408 = _guard406 & _guard407;
wire _guard409 = _guard405 | _guard408;
wire _guard410 = early_reset_static_par_go_out;
wire _guard411 = cond_wire3_out;
wire _guard412 = early_reset_static_par0_go_out;
wire _guard413 = _guard411 & _guard412;
wire _guard414 = relu_r0_go_next_out;
wire _guard415 = cond_wire30_out;
wire _guard416 = _guard414 & _guard415;
wire _guard417 = early_reset_static_par0_go_out;
wire _guard418 = _guard416 & _guard417;
wire _guard419 = relu_r0_go_next_out;
wire _guard420 = cond_wire30_out;
wire _guard421 = _guard419 & _guard420;
wire _guard422 = early_reset_static_par0_go_out;
wire _guard423 = _guard421 & _guard422;
wire _guard424 = early_reset_static_par0_go_out;
wire _guard425 = early_reset_static_par0_go_out;
wire _guard426 = early_reset_static_par0_go_out;
wire _guard427 = ~_guard0;
wire _guard428 = early_reset_static_par0_go_out;
wire _guard429 = _guard427 & _guard428;
wire _guard430 = early_reset_static_par0_go_out;
wire _guard431 = ~_guard0;
wire _guard432 = early_reset_static_par0_go_out;
wire _guard433 = _guard431 & _guard432;
wire _guard434 = wrapper_early_reset_static_par_done_out;
wire _guard435 = ~_guard434;
wire _guard436 = fsm0_out == 2'd0;
wire _guard437 = _guard435 & _guard436;
wire _guard438 = tdcc_go_out;
wire _guard439 = _guard437 & _guard438;
wire _guard440 = early_reset_static_par0_go_out;
wire _guard441 = early_reset_static_par0_go_out;
wire _guard442 = early_reset_static_par0_go_out;
wire _guard443 = early_reset_static_par0_go_out;
wire _guard444 = cond_wire_out;
wire _guard445 = early_reset_static_par0_go_out;
wire _guard446 = _guard444 & _guard445;
wire _guard447 = cond_wire_out;
wire _guard448 = early_reset_static_par0_go_out;
wire _guard449 = _guard447 & _guard448;
wire _guard450 = early_reset_static_par0_go_out;
wire _guard451 = early_reset_static_par0_go_out;
wire _guard452 = early_reset_static_par_go_out;
wire _guard453 = early_reset_static_par0_go_out;
wire _guard454 = _guard452 | _guard453;
wire _guard455 = early_reset_static_par0_go_out;
wire _guard456 = early_reset_static_par_go_out;
wire _guard457 = ~_guard0;
wire _guard458 = early_reset_static_par0_go_out;
wire _guard459 = _guard457 & _guard458;
wire _guard460 = early_reset_static_par0_go_out;
wire _guard461 = early_reset_static_par0_go_out;
wire _guard462 = early_reset_static_par0_go_out;
wire _guard463 = ~_guard0;
wire _guard464 = early_reset_static_par0_go_out;
wire _guard465 = _guard463 & _guard464;
wire _guard466 = early_reset_static_par0_go_out;
wire _guard467 = fsm_out == 1'd0;
wire _guard468 = signal_reg_out;
wire _guard469 = _guard467 & _guard468;
wire _guard470 = early_reset_static_par0_go_out;
wire _guard471 = early_reset_static_par0_go_out;
wire _guard472 = cond_wire10_out;
wire _guard473 = early_reset_static_par0_go_out;
wire _guard474 = _guard472 & _guard473;
wire _guard475 = cond_wire8_out;
wire _guard476 = early_reset_static_par0_go_out;
wire _guard477 = _guard475 & _guard476;
wire _guard478 = fsm_out == 1'd0;
wire _guard479 = cond_wire8_out;
wire _guard480 = _guard478 & _guard479;
wire _guard481 = fsm_out == 1'd0;
wire _guard482 = _guard480 & _guard481;
wire _guard483 = fsm_out == 1'd0;
wire _guard484 = cond_wire10_out;
wire _guard485 = _guard483 & _guard484;
wire _guard486 = fsm_out == 1'd0;
wire _guard487 = _guard485 & _guard486;
wire _guard488 = _guard482 | _guard487;
wire _guard489 = early_reset_static_par0_go_out;
wire _guard490 = _guard488 & _guard489;
wire _guard491 = fsm_out == 1'd0;
wire _guard492 = cond_wire8_out;
wire _guard493 = _guard491 & _guard492;
wire _guard494 = fsm_out == 1'd0;
wire _guard495 = _guard493 & _guard494;
wire _guard496 = fsm_out == 1'd0;
wire _guard497 = cond_wire10_out;
wire _guard498 = _guard496 & _guard497;
wire _guard499 = fsm_out == 1'd0;
wire _guard500 = _guard498 & _guard499;
wire _guard501 = _guard495 | _guard500;
wire _guard502 = early_reset_static_par0_go_out;
wire _guard503 = _guard501 & _guard502;
wire _guard504 = fsm_out == 1'd0;
wire _guard505 = cond_wire8_out;
wire _guard506 = _guard504 & _guard505;
wire _guard507 = fsm_out == 1'd0;
wire _guard508 = _guard506 & _guard507;
wire _guard509 = fsm_out == 1'd0;
wire _guard510 = cond_wire10_out;
wire _guard511 = _guard509 & _guard510;
wire _guard512 = fsm_out == 1'd0;
wire _guard513 = _guard511 & _guard512;
wire _guard514 = _guard508 | _guard513;
wire _guard515 = early_reset_static_par0_go_out;
wire _guard516 = _guard514 & _guard515;
wire _guard517 = early_reset_static_par_go_out;
wire _guard518 = cond_wire_out;
wire _guard519 = early_reset_static_par0_go_out;
wire _guard520 = _guard518 & _guard519;
wire _guard521 = _guard517 | _guard520;
wire _guard522 = early_reset_static_par_go_out;
wire _guard523 = cond_wire_out;
wire _guard524 = early_reset_static_par0_go_out;
wire _guard525 = _guard523 & _guard524;
wire _guard526 = early_reset_static_par_go_out;
wire _guard527 = cond_wire11_out;
wire _guard528 = early_reset_static_par0_go_out;
wire _guard529 = _guard527 & _guard528;
wire _guard530 = _guard526 | _guard529;
wire _guard531 = cond_wire11_out;
wire _guard532 = early_reset_static_par0_go_out;
wire _guard533 = _guard531 & _guard532;
wire _guard534 = early_reset_static_par_go_out;
wire _guard535 = early_reset_static_par0_go_out;
wire _guard536 = early_reset_static_par0_go_out;
wire _guard537 = early_reset_static_par0_go_out;
wire _guard538 = early_reset_static_par0_go_out;
wire _guard539 = early_reset_static_par0_go_out;
wire _guard540 = early_reset_static_par0_go_out;
wire _guard541 = early_reset_static_par0_go_out;
wire _guard542 = early_reset_static_par0_go_out;
wire _guard543 = early_reset_static_par0_go_out;
wire _guard544 = early_reset_static_par0_go_out;
wire _guard545 = early_reset_static_par0_go_out;
wire _guard546 = early_reset_static_par0_go_out;
wire _guard547 = cond_wire27_out;
wire _guard548 = early_reset_static_par0_go_out;
wire _guard549 = _guard547 & _guard548;
wire _guard550 = cond_wire25_out;
wire _guard551 = early_reset_static_par0_go_out;
wire _guard552 = _guard550 & _guard551;
wire _guard553 = fsm_out == 1'd0;
wire _guard554 = cond_wire25_out;
wire _guard555 = _guard553 & _guard554;
wire _guard556 = fsm_out == 1'd0;
wire _guard557 = _guard555 & _guard556;
wire _guard558 = fsm_out == 1'd0;
wire _guard559 = cond_wire27_out;
wire _guard560 = _guard558 & _guard559;
wire _guard561 = fsm_out == 1'd0;
wire _guard562 = _guard560 & _guard561;
wire _guard563 = _guard557 | _guard562;
wire _guard564 = early_reset_static_par0_go_out;
wire _guard565 = _guard563 & _guard564;
wire _guard566 = fsm_out == 1'd0;
wire _guard567 = cond_wire25_out;
wire _guard568 = _guard566 & _guard567;
wire _guard569 = fsm_out == 1'd0;
wire _guard570 = _guard568 & _guard569;
wire _guard571 = fsm_out == 1'd0;
wire _guard572 = cond_wire27_out;
wire _guard573 = _guard571 & _guard572;
wire _guard574 = fsm_out == 1'd0;
wire _guard575 = _guard573 & _guard574;
wire _guard576 = _guard570 | _guard575;
wire _guard577 = early_reset_static_par0_go_out;
wire _guard578 = _guard576 & _guard577;
wire _guard579 = fsm_out == 1'd0;
wire _guard580 = cond_wire25_out;
wire _guard581 = _guard579 & _guard580;
wire _guard582 = fsm_out == 1'd0;
wire _guard583 = _guard581 & _guard582;
wire _guard584 = fsm_out == 1'd0;
wire _guard585 = cond_wire27_out;
wire _guard586 = _guard584 & _guard585;
wire _guard587 = fsm_out == 1'd0;
wire _guard588 = _guard586 & _guard587;
wire _guard589 = _guard583 | _guard588;
wire _guard590 = early_reset_static_par0_go_out;
wire _guard591 = _guard589 & _guard590;
wire _guard592 = early_reset_static_par_go_out;
wire _guard593 = early_reset_static_par0_go_out;
wire _guard594 = early_reset_static_par_go_out;
wire _guard595 = early_reset_static_par0_go_out;
wire _guard596 = early_reset_static_par0_go_out;
wire _guard597 = early_reset_static_par0_go_out;
wire _guard598 = early_reset_static_par_go_out;
wire _guard599 = early_reset_static_par0_go_out;
wire _guard600 = _guard598 | _guard599;
wire _guard601 = early_reset_static_par0_go_out;
wire _guard602 = early_reset_static_par_go_out;
wire _guard603 = early_reset_static_par0_go_out;
wire _guard604 = early_reset_static_par0_go_out;
wire _guard605 = relu_r1_cur_idx_out == 32'd2;
wire _guard606 = cond_wire31_out;
wire _guard607 = _guard605 & _guard606;
wire _guard608 = early_reset_static_par0_go_out;
wire _guard609 = _guard607 & _guard608;
wire _guard610 = relu_r1_cur_idx_out == 32'd0;
wire _guard611 = cond_wire31_out;
wire _guard612 = _guard610 & _guard611;
wire _guard613 = early_reset_static_par0_go_out;
wire _guard614 = _guard612 & _guard613;
wire _guard615 = relu_r1_cur_idx_out == 32'd1;
wire _guard616 = cond_wire31_out;
wire _guard617 = _guard615 & _guard616;
wire _guard618 = early_reset_static_par0_go_out;
wire _guard619 = _guard617 & _guard618;
wire _guard620 = relu_r1_val_mult_done;
wire _guard621 = relu_r1_val_gt_out;
wire _guard622 = _guard620 | _guard621;
wire _guard623 = cond_wire31_out;
wire _guard624 = _guard622 & _guard623;
wire _guard625 = early_reset_static_par0_go_out;
wire _guard626 = _guard624 & _guard625;
wire _guard627 = cond_wire31_out;
wire _guard628 = early_reset_static_par0_go_out;
wire _guard629 = _guard627 & _guard628;
wire _guard630 = cond_wire31_out;
wire _guard631 = early_reset_static_par0_go_out;
wire _guard632 = _guard630 & _guard631;
wire _guard633 = early_reset_static_par0_go_out;
wire _guard634 = early_reset_static_par0_go_out;
wire _guard635 = early_reset_static_par0_go_out;
wire _guard636 = ~_guard0;
wire _guard637 = early_reset_static_par0_go_out;
wire _guard638 = _guard636 & _guard637;
wire _guard639 = early_reset_static_par0_go_out;
wire _guard640 = early_reset_static_par0_go_out;
wire _guard641 = early_reset_static_par0_go_out;
wire _guard642 = early_reset_static_par0_go_out;
wire _guard643 = fsm0_out == 2'd2;
wire _guard644 = fsm0_out == 2'd0;
wire _guard645 = wrapper_early_reset_static_par_done_out;
wire _guard646 = _guard644 & _guard645;
wire _guard647 = tdcc_go_out;
wire _guard648 = _guard646 & _guard647;
wire _guard649 = _guard643 | _guard648;
wire _guard650 = fsm0_out == 2'd1;
wire _guard651 = while_wrapper_early_reset_static_par0_done_out;
wire _guard652 = _guard650 & _guard651;
wire _guard653 = tdcc_go_out;
wire _guard654 = _guard652 & _guard653;
wire _guard655 = _guard649 | _guard654;
wire _guard656 = fsm0_out == 2'd0;
wire _guard657 = wrapper_early_reset_static_par_done_out;
wire _guard658 = _guard656 & _guard657;
wire _guard659 = tdcc_go_out;
wire _guard660 = _guard658 & _guard659;
wire _guard661 = fsm0_out == 2'd2;
wire _guard662 = fsm0_out == 2'd1;
wire _guard663 = while_wrapper_early_reset_static_par0_done_out;
wire _guard664 = _guard662 & _guard663;
wire _guard665 = tdcc_go_out;
wire _guard666 = _guard664 & _guard665;
wire _guard667 = early_reset_static_par0_go_out;
wire _guard668 = early_reset_static_par0_go_out;
wire _guard669 = early_reset_static_par0_go_out;
wire _guard670 = early_reset_static_par0_go_out;
wire _guard671 = early_reset_static_par0_go_out;
wire _guard672 = early_reset_static_par0_go_out;
wire _guard673 = cond_wire13_out;
wire _guard674 = early_reset_static_par0_go_out;
wire _guard675 = _guard673 & _guard674;
wire _guard676 = cond_wire13_out;
wire _guard677 = early_reset_static_par0_go_out;
wire _guard678 = _guard676 & _guard677;
wire _guard679 = early_reset_static_par_go_out;
wire _guard680 = early_reset_static_par0_go_out;
wire _guard681 = _guard679 | _guard680;
wire _guard682 = early_reset_static_par_go_out;
wire _guard683 = early_reset_static_par0_go_out;
wire _guard684 = early_reset_static_par0_go_out;
wire _guard685 = early_reset_static_par0_go_out;
wire _guard686 = early_reset_static_par0_go_out;
wire _guard687 = early_reset_static_par0_go_out;
wire _guard688 = early_reset_static_par_go_out;
wire _guard689 = early_reset_static_par0_go_out;
wire _guard690 = _guard688 | _guard689;
wire _guard691 = early_reset_static_par0_go_out;
wire _guard692 = early_reset_static_par_go_out;
wire _guard693 = early_reset_static_par0_go_out;
wire _guard694 = early_reset_static_par0_go_out;
wire _guard695 = early_reset_static_par_go_out;
wire _guard696 = early_reset_static_par0_go_out;
wire _guard697 = _guard695 | _guard696;
wire _guard698 = early_reset_static_par_go_out;
wire _guard699 = early_reset_static_par0_go_out;
wire _guard700 = early_reset_static_par0_go_out;
wire _guard701 = early_reset_static_par0_go_out;
wire _guard702 = early_reset_static_par0_go_out;
wire _guard703 = early_reset_static_par0_go_out;
wire _guard704 = cond_wire30_out;
wire _guard705 = early_reset_static_par0_go_out;
wire _guard706 = _guard704 & _guard705;
wire _guard707 = cond_wire30_out;
wire _guard708 = early_reset_static_par0_go_out;
wire _guard709 = _guard707 & _guard708;
wire _guard710 = cond_wire30_out;
wire _guard711 = early_reset_static_par0_go_out;
wire _guard712 = _guard710 & _guard711;
wire _guard713 = relu_r0_go_next_out;
wire _guard714 = ~_guard713;
wire _guard715 = cond_wire30_out;
wire _guard716 = _guard714 & _guard715;
wire _guard717 = early_reset_static_par0_go_out;
wire _guard718 = _guard716 & _guard717;
wire _guard719 = cond_wire30_out;
wire _guard720 = early_reset_static_par0_go_out;
wire _guard721 = _guard719 & _guard720;
wire _guard722 = early_reset_static_par0_go_out;
wire _guard723 = early_reset_static_par0_go_out;
wire _guard724 = early_reset_static_par0_go_out;
wire _guard725 = ~_guard0;
wire _guard726 = early_reset_static_par0_go_out;
wire _guard727 = _guard725 & _guard726;
wire _guard728 = early_reset_static_par0_go_out;
wire _guard729 = early_reset_static_par0_go_out;
wire _guard730 = early_reset_static_par0_go_out;
wire _guard731 = early_reset_static_par0_go_out;
wire _guard732 = ~_guard0;
wire _guard733 = early_reset_static_par0_go_out;
wire _guard734 = _guard732 & _guard733;
wire _guard735 = early_reset_static_par0_go_out;
wire _guard736 = early_reset_static_par0_go_out;
wire _guard737 = early_reset_static_par0_go_out;
wire _guard738 = early_reset_static_par0_go_out;
wire _guard739 = early_reset_static_par0_go_out;
wire _guard740 = early_reset_static_par0_go_out;
wire _guard741 = early_reset_static_par0_go_out;
wire _guard742 = early_reset_static_par_go_out;
wire _guard743 = lt_iter_limit_out;
wire _guard744 = early_reset_static_par_go_out;
wire _guard745 = _guard743 & _guard744;
wire _guard746 = lt_iter_limit_out;
wire _guard747 = ~_guard746;
wire _guard748 = early_reset_static_par_go_out;
wire _guard749 = _guard747 & _guard748;
wire _guard750 = cond_wire3_out;
wire _guard751 = early_reset_static_par0_go_out;
wire _guard752 = _guard750 & _guard751;
wire _guard753 = cond_wire3_out;
wire _guard754 = early_reset_static_par0_go_out;
wire _guard755 = _guard753 & _guard754;
wire _guard756 = cond_wire5_out;
wire _guard757 = early_reset_static_par0_go_out;
wire _guard758 = _guard756 & _guard757;
wire _guard759 = cond_wire5_out;
wire _guard760 = early_reset_static_par0_go_out;
wire _guard761 = _guard759 & _guard760;
wire _guard762 = early_reset_static_par0_go_out;
wire _guard763 = early_reset_static_par0_go_out;
wire _guard764 = early_reset_static_par_go_out;
wire _guard765 = early_reset_static_par0_go_out;
wire _guard766 = _guard764 | _guard765;
wire _guard767 = early_reset_static_par_go_out;
wire _guard768 = early_reset_static_par0_go_out;
wire _guard769 = early_reset_static_par0_go_out;
wire _guard770 = early_reset_static_par0_go_out;
wire _guard771 = early_reset_static_par_go_out;
wire _guard772 = early_reset_static_par0_go_out;
wire _guard773 = _guard771 | _guard772;
wire _guard774 = early_reset_static_par0_go_out;
wire _guard775 = early_reset_static_par_go_out;
wire _guard776 = early_reset_static_par0_go_out;
wire _guard777 = early_reset_static_par0_go_out;
wire _guard778 = early_reset_static_par0_go_out;
wire _guard779 = early_reset_static_par0_go_out;
wire _guard780 = ~_guard0;
wire _guard781 = early_reset_static_par0_go_out;
wire _guard782 = _guard780 & _guard781;
wire _guard783 = early_reset_static_par0_go_out;
wire _guard784 = early_reset_static_par0_go_out;
wire _guard785 = early_reset_static_par0_go_out;
wire _guard786 = fsm_out == 1'd0;
wire _guard787 = signal_reg_out;
wire _guard788 = _guard786 & _guard787;
wire _guard789 = fsm_out == 1'd0;
wire _guard790 = signal_reg_out;
wire _guard791 = ~_guard790;
wire _guard792 = _guard789 & _guard791;
wire _guard793 = wrapper_early_reset_static_par_go_out;
wire _guard794 = _guard792 & _guard793;
wire _guard795 = _guard788 | _guard794;
wire _guard796 = fsm_out == 1'd0;
wire _guard797 = signal_reg_out;
wire _guard798 = ~_guard797;
wire _guard799 = _guard796 & _guard798;
wire _guard800 = wrapper_early_reset_static_par_go_out;
wire _guard801 = _guard799 & _guard800;
wire _guard802 = fsm_out == 1'd0;
wire _guard803 = signal_reg_out;
wire _guard804 = _guard802 & _guard803;
wire _guard805 = early_reset_static_par0_go_out;
wire _guard806 = early_reset_static_par0_go_out;
wire _guard807 = cond_wire7_out;
wire _guard808 = early_reset_static_par0_go_out;
wire _guard809 = _guard807 & _guard808;
wire _guard810 = cond_wire7_out;
wire _guard811 = early_reset_static_par0_go_out;
wire _guard812 = _guard810 & _guard811;
wire _guard813 = cond_wire5_out;
wire _guard814 = early_reset_static_par0_go_out;
wire _guard815 = _guard813 & _guard814;
wire _guard816 = cond_wire5_out;
wire _guard817 = early_reset_static_par0_go_out;
wire _guard818 = _guard816 & _guard817;
wire _guard819 = cond_wire_out;
wire _guard820 = early_reset_static_par0_go_out;
wire _guard821 = _guard819 & _guard820;
wire _guard822 = cond_wire_out;
wire _guard823 = early_reset_static_par0_go_out;
wire _guard824 = _guard822 & _guard823;
wire _guard825 = cond_wire_out;
wire _guard826 = early_reset_static_par0_go_out;
wire _guard827 = _guard825 & _guard826;
wire _guard828 = cond_wire_out;
wire _guard829 = early_reset_static_par0_go_out;
wire _guard830 = _guard828 & _guard829;
wire _guard831 = early_reset_static_par0_go_out;
wire _guard832 = early_reset_static_par0_go_out;
wire _guard833 = early_reset_static_par0_go_out;
wire _guard834 = early_reset_static_par0_go_out;
wire _guard835 = relu_r0_val_mult_done;
wire _guard836 = relu_r0_val_gt_out;
wire _guard837 = _guard835 | _guard836;
wire _guard838 = cond_wire30_out;
wire _guard839 = _guard837 & _guard838;
wire _guard840 = early_reset_static_par0_go_out;
wire _guard841 = _guard839 & _guard840;
wire _guard842 = ~_guard0;
wire _guard843 = early_reset_static_par0_go_out;
wire _guard844 = _guard842 & _guard843;
wire _guard845 = early_reset_static_par0_go_out;
wire _guard846 = ~_guard0;
wire _guard847 = early_reset_static_par0_go_out;
wire _guard848 = _guard846 & _guard847;
wire _guard849 = early_reset_static_par0_go_out;
wire _guard850 = early_reset_static_par0_go_out;
wire _guard851 = early_reset_static_par0_go_out;
wire _guard852 = ~_guard0;
wire _guard853 = early_reset_static_par0_go_out;
wire _guard854 = _guard852 & _guard853;
wire _guard855 = early_reset_static_par0_go_out;
wire _guard856 = early_reset_static_par0_go_out;
wire _guard857 = early_reset_static_par0_go_out;
wire _guard858 = early_reset_static_par0_go_out;
wire _guard859 = early_reset_static_par0_go_out;
wire _guard860 = early_reset_static_par0_go_out;
wire _guard861 = early_reset_static_par0_go_out;
wire _guard862 = early_reset_static_par0_go_out;
wire _guard863 = early_reset_static_par0_go_out;
wire _guard864 = early_reset_static_par0_go_out;
wire _guard865 = early_reset_static_par0_go_out;
wire _guard866 = cond_wire24_out;
wire _guard867 = early_reset_static_par0_go_out;
wire _guard868 = _guard866 & _guard867;
wire _guard869 = cond_wire22_out;
wire _guard870 = early_reset_static_par0_go_out;
wire _guard871 = _guard869 & _guard870;
wire _guard872 = fsm_out == 1'd0;
wire _guard873 = cond_wire22_out;
wire _guard874 = _guard872 & _guard873;
wire _guard875 = fsm_out == 1'd0;
wire _guard876 = _guard874 & _guard875;
wire _guard877 = fsm_out == 1'd0;
wire _guard878 = cond_wire24_out;
wire _guard879 = _guard877 & _guard878;
wire _guard880 = fsm_out == 1'd0;
wire _guard881 = _guard879 & _guard880;
wire _guard882 = _guard876 | _guard881;
wire _guard883 = early_reset_static_par0_go_out;
wire _guard884 = _guard882 & _guard883;
wire _guard885 = fsm_out == 1'd0;
wire _guard886 = cond_wire22_out;
wire _guard887 = _guard885 & _guard886;
wire _guard888 = fsm_out == 1'd0;
wire _guard889 = _guard887 & _guard888;
wire _guard890 = fsm_out == 1'd0;
wire _guard891 = cond_wire24_out;
wire _guard892 = _guard890 & _guard891;
wire _guard893 = fsm_out == 1'd0;
wire _guard894 = _guard892 & _guard893;
wire _guard895 = _guard889 | _guard894;
wire _guard896 = early_reset_static_par0_go_out;
wire _guard897 = _guard895 & _guard896;
wire _guard898 = fsm_out == 1'd0;
wire _guard899 = cond_wire22_out;
wire _guard900 = _guard898 & _guard899;
wire _guard901 = fsm_out == 1'd0;
wire _guard902 = _guard900 & _guard901;
wire _guard903 = fsm_out == 1'd0;
wire _guard904 = cond_wire24_out;
wire _guard905 = _guard903 & _guard904;
wire _guard906 = fsm_out == 1'd0;
wire _guard907 = _guard905 & _guard906;
wire _guard908 = _guard902 | _guard907;
wire _guard909 = early_reset_static_par0_go_out;
wire _guard910 = _guard908 & _guard909;
wire _guard911 = cond_wire23_out;
wire _guard912 = early_reset_static_par0_go_out;
wire _guard913 = _guard911 & _guard912;
wire _guard914 = cond_wire23_out;
wire _guard915 = early_reset_static_par0_go_out;
wire _guard916 = _guard914 & _guard915;
wire _guard917 = cond_wire19_out;
wire _guard918 = early_reset_static_par0_go_out;
wire _guard919 = _guard917 & _guard918;
wire _guard920 = cond_wire19_out;
wire _guard921 = early_reset_static_par0_go_out;
wire _guard922 = _guard920 & _guard921;
wire _guard923 = cond_wire3_out;
wire _guard924 = early_reset_static_par0_go_out;
wire _guard925 = _guard923 & _guard924;
wire _guard926 = cond_wire3_out;
wire _guard927 = early_reset_static_par0_go_out;
wire _guard928 = _guard926 & _guard927;
wire _guard929 = early_reset_static_par0_go_out;
wire _guard930 = early_reset_static_par0_go_out;
wire _guard931 = early_reset_static_par0_go_out;
wire _guard932 = early_reset_static_par0_go_out;
wire _guard933 = early_reset_static_par0_go_out;
wire _guard934 = early_reset_static_par0_go_out;
wire _guard935 = early_reset_static_par0_go_out;
wire _guard936 = early_reset_static_par0_go_out;
wire _guard937 = early_reset_static_par_go_out;
wire _guard938 = early_reset_static_par0_go_out;
wire _guard939 = _guard937 | _guard938;
wire _guard940 = early_reset_static_par_go_out;
wire _guard941 = early_reset_static_par0_go_out;
wire _guard942 = early_reset_static_par_go_out;
wire _guard943 = early_reset_static_par0_go_out;
wire _guard944 = _guard942 | _guard943;
wire _guard945 = early_reset_static_par0_go_out;
wire _guard946 = early_reset_static_par_go_out;
wire _guard947 = ~_guard0;
wire _guard948 = early_reset_static_par0_go_out;
wire _guard949 = _guard947 & _guard948;
wire _guard950 = early_reset_static_par0_go_out;
wire _guard951 = ~_guard0;
wire _guard952 = early_reset_static_par0_go_out;
wire _guard953 = _guard951 & _guard952;
wire _guard954 = early_reset_static_par0_go_out;
wire _guard955 = early_reset_static_par0_go_out;
wire _guard956 = early_reset_static_par0_go_out;
wire _guard957 = early_reset_static_par0_go_out;
wire _guard958 = early_reset_static_par0_go_out;
wire _guard959 = cond_wire2_out;
wire _guard960 = early_reset_static_par0_go_out;
wire _guard961 = _guard959 & _guard960;
wire _guard962 = cond_wire0_out;
wire _guard963 = early_reset_static_par0_go_out;
wire _guard964 = _guard962 & _guard963;
wire _guard965 = fsm_out == 1'd0;
wire _guard966 = cond_wire0_out;
wire _guard967 = _guard965 & _guard966;
wire _guard968 = fsm_out == 1'd0;
wire _guard969 = _guard967 & _guard968;
wire _guard970 = fsm_out == 1'd0;
wire _guard971 = cond_wire2_out;
wire _guard972 = _guard970 & _guard971;
wire _guard973 = fsm_out == 1'd0;
wire _guard974 = _guard972 & _guard973;
wire _guard975 = _guard969 | _guard974;
wire _guard976 = early_reset_static_par0_go_out;
wire _guard977 = _guard975 & _guard976;
wire _guard978 = fsm_out == 1'd0;
wire _guard979 = cond_wire0_out;
wire _guard980 = _guard978 & _guard979;
wire _guard981 = fsm_out == 1'd0;
wire _guard982 = _guard980 & _guard981;
wire _guard983 = fsm_out == 1'd0;
wire _guard984 = cond_wire2_out;
wire _guard985 = _guard983 & _guard984;
wire _guard986 = fsm_out == 1'd0;
wire _guard987 = _guard985 & _guard986;
wire _guard988 = _guard982 | _guard987;
wire _guard989 = early_reset_static_par0_go_out;
wire _guard990 = _guard988 & _guard989;
wire _guard991 = fsm_out == 1'd0;
wire _guard992 = cond_wire0_out;
wire _guard993 = _guard991 & _guard992;
wire _guard994 = fsm_out == 1'd0;
wire _guard995 = _guard993 & _guard994;
wire _guard996 = fsm_out == 1'd0;
wire _guard997 = cond_wire2_out;
wire _guard998 = _guard996 & _guard997;
wire _guard999 = fsm_out == 1'd0;
wire _guard1000 = _guard998 & _guard999;
wire _guard1001 = _guard995 | _guard1000;
wire _guard1002 = early_reset_static_par0_go_out;
wire _guard1003 = _guard1001 & _guard1002;
wire _guard1004 = cond_wire7_out;
wire _guard1005 = early_reset_static_par0_go_out;
wire _guard1006 = _guard1004 & _guard1005;
wire _guard1007 = cond_wire7_out;
wire _guard1008 = early_reset_static_par0_go_out;
wire _guard1009 = _guard1007 & _guard1008;
wire _guard1010 = early_reset_static_par_go_out;
wire _guard1011 = early_reset_static_par0_go_out;
wire _guard1012 = _guard1010 | _guard1011;
wire _guard1013 = early_reset_static_par0_go_out;
wire _guard1014 = early_reset_static_par_go_out;
wire _guard1015 = early_reset_static_par0_go_out;
wire _guard1016 = early_reset_static_par0_go_out;
wire _guard1017 = early_reset_static_par_go_out;
wire _guard1018 = early_reset_static_par0_go_out;
wire _guard1019 = _guard1017 | _guard1018;
wire _guard1020 = early_reset_static_par_go_out;
wire _guard1021 = early_reset_static_par0_go_out;
wire _guard1022 = early_reset_static_par0_go_out;
wire _guard1023 = early_reset_static_par0_go_out;
wire _guard1024 = early_reset_static_par_go_out;
wire _guard1025 = early_reset_static_par0_go_out;
wire _guard1026 = _guard1024 | _guard1025;
wire _guard1027 = early_reset_static_par0_go_out;
wire _guard1028 = early_reset_static_par_go_out;
wire _guard1029 = early_reset_static_par_go_out;
wire _guard1030 = early_reset_static_par0_go_out;
wire _guard1031 = _guard1029 | _guard1030;
wire _guard1032 = early_reset_static_par_go_out;
wire _guard1033 = early_reset_static_par0_go_out;
wire _guard1034 = early_reset_static_par_go_out;
wire _guard1035 = early_reset_static_par0_go_out;
wire _guard1036 = _guard1034 | _guard1035;
wire _guard1037 = early_reset_static_par_go_out;
wire _guard1038 = early_reset_static_par0_go_out;
wire _guard1039 = early_reset_static_par0_go_out;
wire _guard1040 = early_reset_static_par0_go_out;
wire _guard1041 = cond_wire31_out;
wire _guard1042 = early_reset_static_par0_go_out;
wire _guard1043 = _guard1041 & _guard1042;
wire _guard1044 = relu_r1_go_next_out;
wire _guard1045 = ~_guard1044;
wire _guard1046 = cond_wire31_out;
wire _guard1047 = _guard1045 & _guard1046;
wire _guard1048 = early_reset_static_par0_go_out;
wire _guard1049 = _guard1047 & _guard1048;
wire _guard1050 = cond_wire31_out;
wire _guard1051 = early_reset_static_par0_go_out;
wire _guard1052 = _guard1050 & _guard1051;
wire _guard1053 = early_reset_static_par0_go_out;
wire _guard1054 = early_reset_static_par0_go_out;
wire _guard1055 = ~_guard0;
wire _guard1056 = early_reset_static_par0_go_out;
wire _guard1057 = _guard1055 & _guard1056;
wire _guard1058 = early_reset_static_par0_go_out;
wire _guard1059 = early_reset_static_par0_go_out;
wire _guard1060 = ~_guard0;
wire _guard1061 = early_reset_static_par0_go_out;
wire _guard1062 = _guard1060 & _guard1061;
wire _guard1063 = early_reset_static_par0_go_out;
wire _guard1064 = ~_guard0;
wire _guard1065 = early_reset_static_par0_go_out;
wire _guard1066 = _guard1064 & _guard1065;
wire _guard1067 = ~_guard0;
wire _guard1068 = early_reset_static_par0_go_out;
wire _guard1069 = _guard1067 & _guard1068;
wire _guard1070 = early_reset_static_par0_go_out;
wire _guard1071 = ~_guard0;
wire _guard1072 = early_reset_static_par0_go_out;
wire _guard1073 = _guard1071 & _guard1072;
wire _guard1074 = early_reset_static_par0_go_out;
wire _guard1075 = ~_guard0;
wire _guard1076 = early_reset_static_par0_go_out;
wire _guard1077 = _guard1075 & _guard1076;
wire _guard1078 = early_reset_static_par0_go_out;
wire _guard1079 = ~_guard0;
wire _guard1080 = early_reset_static_par0_go_out;
wire _guard1081 = _guard1079 & _guard1080;
wire _guard1082 = early_reset_static_par0_go_out;
wire _guard1083 = fsm0_out == 2'd2;
wire _guard1084 = wrapper_early_reset_static_par_go_out;
wire _guard1085 = cond_wire16_out;
wire _guard1086 = early_reset_static_par0_go_out;
wire _guard1087 = _guard1085 & _guard1086;
wire _guard1088 = cond_wire16_out;
wire _guard1089 = early_reset_static_par0_go_out;
wire _guard1090 = _guard1088 & _guard1089;
wire _guard1091 = early_reset_static_par_go_out;
wire _guard1092 = cond_wire7_out;
wire _guard1093 = early_reset_static_par0_go_out;
wire _guard1094 = _guard1092 & _guard1093;
wire _guard1095 = _guard1091 | _guard1094;
wire _guard1096 = early_reset_static_par_go_out;
wire _guard1097 = cond_wire7_out;
wire _guard1098 = early_reset_static_par0_go_out;
wire _guard1099 = _guard1097 & _guard1098;
wire _guard1100 = early_reset_static_par_go_out;
wire _guard1101 = cond_wire21_out;
wire _guard1102 = early_reset_static_par0_go_out;
wire _guard1103 = _guard1101 & _guard1102;
wire _guard1104 = _guard1100 | _guard1103;
wire _guard1105 = early_reset_static_par_go_out;
wire _guard1106 = cond_wire21_out;
wire _guard1107 = early_reset_static_par0_go_out;
wire _guard1108 = _guard1106 & _guard1107;
wire _guard1109 = early_reset_static_par0_go_out;
wire _guard1110 = early_reset_static_par0_go_out;
wire _guard1111 = early_reset_static_par_go_out;
wire _guard1112 = early_reset_static_par0_go_out;
wire _guard1113 = _guard1111 | _guard1112;
wire _guard1114 = early_reset_static_par0_go_out;
wire _guard1115 = early_reset_static_par_go_out;
wire _guard1116 = early_reset_static_par0_go_out;
wire _guard1117 = early_reset_static_par0_go_out;
wire _guard1118 = relu_r0_cur_idx_out == 32'd1;
wire _guard1119 = cond_wire30_out;
wire _guard1120 = _guard1118 & _guard1119;
wire _guard1121 = early_reset_static_par0_go_out;
wire _guard1122 = _guard1120 & _guard1121;
wire _guard1123 = relu_r0_cur_idx_out == 32'd2;
wire _guard1124 = cond_wire30_out;
wire _guard1125 = _guard1123 & _guard1124;
wire _guard1126 = early_reset_static_par0_go_out;
wire _guard1127 = _guard1125 & _guard1126;
wire _guard1128 = relu_r0_cur_idx_out == 32'd0;
wire _guard1129 = cond_wire30_out;
wire _guard1130 = _guard1128 & _guard1129;
wire _guard1131 = early_reset_static_par0_go_out;
wire _guard1132 = _guard1130 & _guard1131;
wire _guard1133 = relu_r2_cur_idx_out == 32'd1;
wire _guard1134 = cond_wire32_out;
wire _guard1135 = _guard1133 & _guard1134;
wire _guard1136 = early_reset_static_par0_go_out;
wire _guard1137 = _guard1135 & _guard1136;
wire _guard1138 = relu_r2_cur_idx_out == 32'd0;
wire _guard1139 = cond_wire32_out;
wire _guard1140 = _guard1138 & _guard1139;
wire _guard1141 = early_reset_static_par0_go_out;
wire _guard1142 = _guard1140 & _guard1141;
wire _guard1143 = relu_r2_cur_idx_out == 32'd2;
wire _guard1144 = cond_wire32_out;
wire _guard1145 = _guard1143 & _guard1144;
wire _guard1146 = early_reset_static_par0_go_out;
wire _guard1147 = _guard1145 & _guard1146;
wire _guard1148 = early_reset_static_par0_go_out;
wire _guard1149 = early_reset_static_par0_go_out;
wire _guard1150 = early_reset_static_par0_go_out;
wire _guard1151 = early_reset_static_par0_go_out;
wire _guard1152 = ~_guard0;
wire _guard1153 = early_reset_static_par0_go_out;
wire _guard1154 = _guard1152 & _guard1153;
wire _guard1155 = early_reset_static_par0_go_out;
wire _guard1156 = early_reset_static_par0_go_out;
wire _guard1157 = early_reset_static_par0_go_out;
wire _guard1158 = early_reset_static_par0_go_out;
wire _guard1159 = early_reset_static_par0_go_out;
wire _guard1160 = early_reset_static_par_go_out;
wire _guard1161 = early_reset_static_par_go_out;
wire _guard1162 = early_reset_static_par0_go_out;
wire _guard1163 = early_reset_static_par0_go_out;
wire _guard1164 = early_reset_static_par0_go_out;
wire _guard1165 = early_reset_static_par0_go_out;
wire _guard1166 = cond_wire_out;
wire _guard1167 = early_reset_static_par0_go_out;
wire _guard1168 = _guard1166 & _guard1167;
wire _guard1169 = cond_wire_out;
wire _guard1170 = early_reset_static_par0_go_out;
wire _guard1171 = _guard1169 & _guard1170;
wire _guard1172 = cond_wire17_out;
wire _guard1173 = early_reset_static_par0_go_out;
wire _guard1174 = _guard1172 & _guard1173;
wire _guard1175 = cond_wire15_out;
wire _guard1176 = early_reset_static_par0_go_out;
wire _guard1177 = _guard1175 & _guard1176;
wire _guard1178 = fsm_out == 1'd0;
wire _guard1179 = cond_wire15_out;
wire _guard1180 = _guard1178 & _guard1179;
wire _guard1181 = fsm_out == 1'd0;
wire _guard1182 = _guard1180 & _guard1181;
wire _guard1183 = fsm_out == 1'd0;
wire _guard1184 = cond_wire17_out;
wire _guard1185 = _guard1183 & _guard1184;
wire _guard1186 = fsm_out == 1'd0;
wire _guard1187 = _guard1185 & _guard1186;
wire _guard1188 = _guard1182 | _guard1187;
wire _guard1189 = early_reset_static_par0_go_out;
wire _guard1190 = _guard1188 & _guard1189;
wire _guard1191 = fsm_out == 1'd0;
wire _guard1192 = cond_wire15_out;
wire _guard1193 = _guard1191 & _guard1192;
wire _guard1194 = fsm_out == 1'd0;
wire _guard1195 = _guard1193 & _guard1194;
wire _guard1196 = fsm_out == 1'd0;
wire _guard1197 = cond_wire17_out;
wire _guard1198 = _guard1196 & _guard1197;
wire _guard1199 = fsm_out == 1'd0;
wire _guard1200 = _guard1198 & _guard1199;
wire _guard1201 = _guard1195 | _guard1200;
wire _guard1202 = early_reset_static_par0_go_out;
wire _guard1203 = _guard1201 & _guard1202;
wire _guard1204 = fsm_out == 1'd0;
wire _guard1205 = cond_wire15_out;
wire _guard1206 = _guard1204 & _guard1205;
wire _guard1207 = fsm_out == 1'd0;
wire _guard1208 = _guard1206 & _guard1207;
wire _guard1209 = fsm_out == 1'd0;
wire _guard1210 = cond_wire17_out;
wire _guard1211 = _guard1209 & _guard1210;
wire _guard1212 = fsm_out == 1'd0;
wire _guard1213 = _guard1211 & _guard1212;
wire _guard1214 = _guard1208 | _guard1213;
wire _guard1215 = early_reset_static_par0_go_out;
wire _guard1216 = _guard1214 & _guard1215;
wire _guard1217 = early_reset_static_par_go_out;
wire _guard1218 = early_reset_static_par0_go_out;
wire _guard1219 = _guard1217 | _guard1218;
wire _guard1220 = early_reset_static_par_go_out;
wire _guard1221 = early_reset_static_par0_go_out;
wire _guard1222 = early_reset_static_par0_go_out;
wire _guard1223 = early_reset_static_par0_go_out;
wire _guard1224 = early_reset_static_par0_go_out;
wire _guard1225 = early_reset_static_par0_go_out;
wire _guard1226 = early_reset_static_par0_go_out;
wire _guard1227 = early_reset_static_par0_go_out;
wire _guard1228 = cond_wire30_out;
wire _guard1229 = early_reset_static_par0_go_out;
wire _guard1230 = _guard1228 & _guard1229;
wire _guard1231 = cond_wire30_out;
wire _guard1232 = early_reset_static_par0_go_out;
wire _guard1233 = _guard1231 & _guard1232;
wire _guard1234 = cond_wire31_out;
wire _guard1235 = early_reset_static_par0_go_out;
wire _guard1236 = _guard1234 & _guard1235;
wire _guard1237 = cond_wire31_out;
wire _guard1238 = early_reset_static_par0_go_out;
wire _guard1239 = _guard1237 & _guard1238;
wire _guard1240 = relu_r2_go_next_out;
wire _guard1241 = cond_wire32_out;
wire _guard1242 = _guard1240 & _guard1241;
wire _guard1243 = early_reset_static_par0_go_out;
wire _guard1244 = _guard1242 & _guard1243;
wire _guard1245 = relu_r2_go_next_out;
wire _guard1246 = cond_wire32_out;
wire _guard1247 = _guard1245 & _guard1246;
wire _guard1248 = early_reset_static_par0_go_out;
wire _guard1249 = _guard1247 & _guard1248;
wire _guard1250 = cond_wire32_out;
wire _guard1251 = early_reset_static_par0_go_out;
wire _guard1252 = _guard1250 & _guard1251;
wire _guard1253 = cond_wire32_out;
wire _guard1254 = early_reset_static_par0_go_out;
wire _guard1255 = _guard1253 & _guard1254;
wire _guard1256 = early_reset_static_par0_go_out;
wire _guard1257 = ~_guard0;
wire _guard1258 = early_reset_static_par0_go_out;
wire _guard1259 = _guard1257 & _guard1258;
wire _guard1260 = early_reset_static_par0_go_out;
wire _guard1261 = early_reset_static_par0_go_out;
wire _guard1262 = early_reset_static_par0_go_out;
wire _guard1263 = early_reset_static_par0_go_out;
wire _guard1264 = early_reset_static_par0_go_out;
wire _guard1265 = early_reset_static_par0_go_out;
wire _guard1266 = early_reset_static_par0_go_out;
wire _guard1267 = ~_guard0;
wire _guard1268 = early_reset_static_par0_go_out;
wire _guard1269 = _guard1267 & _guard1268;
wire _guard1270 = cond_wire1_out;
wire _guard1271 = early_reset_static_par0_go_out;
wire _guard1272 = _guard1270 & _guard1271;
wire _guard1273 = cond_wire1_out;
wire _guard1274 = early_reset_static_par0_go_out;
wire _guard1275 = _guard1273 & _guard1274;
wire _guard1276 = cond_wire29_out;
wire _guard1277 = early_reset_static_par0_go_out;
wire _guard1278 = _guard1276 & _guard1277;
wire _guard1279 = cond_wire28_out;
wire _guard1280 = early_reset_static_par0_go_out;
wire _guard1281 = _guard1279 & _guard1280;
wire _guard1282 = fsm_out == 1'd0;
wire _guard1283 = cond_wire28_out;
wire _guard1284 = _guard1282 & _guard1283;
wire _guard1285 = fsm_out == 1'd0;
wire _guard1286 = _guard1284 & _guard1285;
wire _guard1287 = fsm_out == 1'd0;
wire _guard1288 = cond_wire29_out;
wire _guard1289 = _guard1287 & _guard1288;
wire _guard1290 = fsm_out == 1'd0;
wire _guard1291 = _guard1289 & _guard1290;
wire _guard1292 = _guard1286 | _guard1291;
wire _guard1293 = early_reset_static_par0_go_out;
wire _guard1294 = _guard1292 & _guard1293;
wire _guard1295 = fsm_out == 1'd0;
wire _guard1296 = cond_wire28_out;
wire _guard1297 = _guard1295 & _guard1296;
wire _guard1298 = fsm_out == 1'd0;
wire _guard1299 = _guard1297 & _guard1298;
wire _guard1300 = fsm_out == 1'd0;
wire _guard1301 = cond_wire29_out;
wire _guard1302 = _guard1300 & _guard1301;
wire _guard1303 = fsm_out == 1'd0;
wire _guard1304 = _guard1302 & _guard1303;
wire _guard1305 = _guard1299 | _guard1304;
wire _guard1306 = early_reset_static_par0_go_out;
wire _guard1307 = _guard1305 & _guard1306;
wire _guard1308 = fsm_out == 1'd0;
wire _guard1309 = cond_wire28_out;
wire _guard1310 = _guard1308 & _guard1309;
wire _guard1311 = fsm_out == 1'd0;
wire _guard1312 = _guard1310 & _guard1311;
wire _guard1313 = fsm_out == 1'd0;
wire _guard1314 = cond_wire29_out;
wire _guard1315 = _guard1313 & _guard1314;
wire _guard1316 = fsm_out == 1'd0;
wire _guard1317 = _guard1315 & _guard1316;
wire _guard1318 = _guard1312 | _guard1317;
wire _guard1319 = early_reset_static_par0_go_out;
wire _guard1320 = _guard1318 & _guard1319;
wire _guard1321 = cond_wire21_out;
wire _guard1322 = early_reset_static_par0_go_out;
wire _guard1323 = _guard1321 & _guard1322;
wire _guard1324 = cond_wire21_out;
wire _guard1325 = early_reset_static_par0_go_out;
wire _guard1326 = _guard1324 & _guard1325;
wire _guard1327 = early_reset_static_par_go_out;
wire _guard1328 = early_reset_static_par0_go_out;
wire _guard1329 = _guard1327 | _guard1328;
wire _guard1330 = early_reset_static_par_go_out;
wire _guard1331 = early_reset_static_par0_go_out;
wire _guard1332 = early_reset_static_par_go_out;
wire _guard1333 = early_reset_static_par0_go_out;
wire _guard1334 = _guard1332 | _guard1333;
wire _guard1335 = early_reset_static_par0_go_out;
wire _guard1336 = early_reset_static_par_go_out;
wire _guard1337 = early_reset_static_par0_go_out;
wire _guard1338 = early_reset_static_par0_go_out;
wire _guard1339 = early_reset_static_par0_go_out;
wire _guard1340 = early_reset_static_par0_go_out;
wire _guard1341 = early_reset_static_par0_go_out;
wire _guard1342 = early_reset_static_par0_go_out;
wire _guard1343 = ~_guard0;
wire _guard1344 = early_reset_static_par0_go_out;
wire _guard1345 = _guard1343 & _guard1344;
wire _guard1346 = early_reset_static_par0_go_out;
wire _guard1347 = ~_guard0;
wire _guard1348 = early_reset_static_par0_go_out;
wire _guard1349 = _guard1347 & _guard1348;
wire _guard1350 = early_reset_static_par0_go_out;
wire _guard1351 = while_wrapper_early_reset_static_par0_done_out;
wire _guard1352 = ~_guard1351;
wire _guard1353 = fsm0_out == 2'd1;
wire _guard1354 = _guard1352 & _guard1353;
wire _guard1355 = tdcc_go_out;
wire _guard1356 = _guard1354 & _guard1355;
wire _guard1357 = cond_reg_out;
wire _guard1358 = ~_guard1357;
wire _guard1359 = fsm_out == 1'd0;
wire _guard1360 = _guard1358 & _guard1359;
assign min_depth_4_plus_5_left = min_depth_4_out;
assign min_depth_4_plus_5_right = 32'd5;
assign l1_add_left = 2'd1;
assign l1_add_right = l1_idx_out;
assign cond_wire3_in =
  _guard11 ? cond3_out :
  _guard12 ? idx_between_1_depth_plus_1_reg_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard12, _guard11})) begin
    $fatal(2, "Multiple assignment to port `cond_wire3.in'.");
end
end
assign cond_wire30_in =
  _guard13 ? idx_between_depth_plus_5_None_reg_out :
  _guard16 ? cond30_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard16, _guard13})) begin
    $fatal(2, "Multiple assignment to port `cond_wire30.in'.");
end
end
assign adder_left =
  _guard17 ? fsm_out :
  1'd0;
assign adder_right = _guard18;
assign fsm_write_en = _guard21;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard24 ? adder_out :
  _guard31 ? 1'd0 :
  _guard34 ? adder0_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard34, _guard31, _guard24})) begin
    $fatal(2, "Multiple assignment to port `fsm.in'.");
end
end
assign top_1_0_write_en = _guard37;
assign top_1_0_clk = clk;
assign top_1_0_reset = reset;
assign top_1_0_in = top_0_0_out;
assign pe_1_2_mul_ready =
  _guard43 ? 1'd1 :
  _guard46 ? 1'd0 :
  1'd0;
always_comb begin
  if(~$onehot0({_guard46, _guard43})) begin
    $fatal(2, "Multiple assignment to port `pe_1_2.mul_ready'.");
end
end
assign pe_1_2_clk = clk;
assign pe_1_2_left =
  _guard59 ? left_1_2_out :
  32'd0;
assign pe_1_2_top =
  _guard72 ? top_1_2_out :
  32'd0;
assign pe_1_2_reset = reset;
assign pe_1_2_go = _guard85;
assign index_ge_8_left = idx_add_out;
assign index_ge_8_right = 32'd8;
assign index_lt_min_depth_4_plus_2_left = idx_add_out;
assign index_lt_min_depth_4_plus_2_right = min_depth_4_plus_2_out;
assign index_lt_depth_plus_9_left = idx_add_out;
assign index_lt_depth_plus_9_right = depth_plus_9_out;
assign index_lt_min_depth_4_plus_3_left = idx_add_out;
assign index_lt_min_depth_4_plus_3_right = min_depth_4_plus_3_out;
assign index_lt_depth_plus_6_left = idx_add_out;
assign index_lt_depth_plus_6_right = depth_plus_6_out;
assign idx_between_depth_plus_5_None_reg_write_en = _guard98;
assign idx_between_depth_plus_5_None_reg_clk = clk;
assign idx_between_depth_plus_5_None_reg_reset = reset;
assign idx_between_depth_plus_5_None_reg_in =
  _guard99 ? 1'd1 :
  _guard100 ? 1'd0 :
  'x;
always_comb begin
  if(~$onehot0({_guard100, _guard99})) begin
    $fatal(2, "Multiple assignment to port `idx_between_depth_plus_5_None_reg.in'.");
end
end
assign relu_r1_cur_idx_write_en = _guard105;
assign relu_r1_cur_idx_clk = clk;
assign relu_r1_cur_idx_reset = reset;
assign relu_r1_cur_idx_in = relu_r1_incr_out;
assign relu_r2_go_next_in =
  _guard117 ? 32'd1 :
  32'd0;
assign cond_wire0_in =
  _guard118 ? idx_between_1_min_depth_4_plus_1_reg_out :
  _guard121 ? cond0_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard121, _guard118})) begin
    $fatal(2, "Multiple assignment to port `cond_wire0.in'.");
end
end
assign cond_wire4_in =
  _guard124 ? cond4_out :
  _guard125 ? idx_between_2_min_depth_4_plus_2_reg_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard125, _guard124})) begin
    $fatal(2, "Multiple assignment to port `cond_wire4.in'.");
end
end
assign cond6_write_en = _guard126;
assign cond6_clk = clk;
assign cond6_reset = reset;
assign cond6_in =
  _guard127 ? idx_between_6_depth_plus_6_reg_out :
  1'd0;
assign cond_wire13_in =
  _guard130 ? cond13_out :
  _guard131 ? idx_between_2_depth_plus_2_reg_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard131, _guard130})) begin
    $fatal(2, "Multiple assignment to port `cond_wire13.in'.");
end
end
assign cond_wire16_in =
  _guard134 ? cond16_out :
  _guard135 ? idx_between_3_depth_plus_3_reg_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard135, _guard134})) begin
    $fatal(2, "Multiple assignment to port `cond_wire16.in'.");
end
end
assign cond30_write_en = _guard136;
assign cond30_clk = clk;
assign cond30_reset = reset;
assign cond30_in =
  _guard137 ? idx_between_depth_plus_5_None_reg_out :
  1'd0;
assign cond_wire32_in =
  _guard138 ? idx_between_depth_plus_7_None_reg_out :
  _guard141 ? cond32_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard141, _guard138})) begin
    $fatal(2, "Multiple assignment to port `cond_wire32.in'.");
end
end
assign early_reset_static_par0_go_in = _guard142;
assign done = _guard143;
assign t2_addr0 =
  _guard146 ? t2_idx_out :
  2'd0;
assign l0_addr0 =
  _guard149 ? l0_idx_out :
  2'd0;
assign out_mem_1_addr0 =
  _guard152 ? relu_r1_cur_idx_out :
  32'd0;
assign out_mem_0_write_data =
  _guard158 ? relu_r0_val_mult_out :
  _guard163 ? relu_r0_cur_val_out :
  32'd0;
always_comb begin
  if(~$onehot0({_guard163, _guard158})) begin
    $fatal(2, "Multiple assignment to port `_this.out_mem_0_write_data'.");
end
end
assign l1_addr0 =
  _guard166 ? l1_idx_out :
  2'd0;
assign out_mem_2_write_data =
  _guard172 ? relu_r2_val_mult_out :
  _guard177 ? relu_r2_cur_val_out :
  32'd0;
always_comb begin
  if(~$onehot0({_guard177, _guard172})) begin
    $fatal(2, "Multiple assignment to port `_this.out_mem_2_write_data'.");
end
end
assign out_mem_1_write_data =
  _guard182 ? relu_r1_cur_val_out :
  _guard188 ? relu_r1_val_mult_out :
  32'd0;
always_comb begin
  if(~$onehot0({_guard188, _guard182})) begin
    $fatal(2, "Multiple assignment to port `_this.out_mem_1_write_data'.");
end
end
assign out_mem_1_write_en = _guard193;
assign t0_addr0 =
  _guard196 ? t0_idx_out :
  2'd0;
assign t1_addr0 =
  _guard199 ? t1_idx_out :
  2'd0;
assign out_mem_0_write_en = _guard204;
assign out_mem_2_write_en = _guard209;
assign l2_addr0 =
  _guard212 ? l2_idx_out :
  2'd0;
assign out_mem_0_addr0 =
  _guard215 ? relu_r0_cur_idx_out :
  32'd0;
assign out_mem_2_addr0 =
  _guard218 ? relu_r2_cur_idx_out :
  32'd0;
assign depth_plus_4_left =
  _guard219 ? depth :
  _guard220 ? 32'd10 :
  'x;
always_comb begin
  if(~$onehot0({_guard220, _guard219})) begin
    $fatal(2, "Multiple assignment to port `depth_plus_4.left'.");
end
end
assign depth_plus_4_right =
  _guard221 ? depth :
  _guard222 ? 32'd4 :
  'x;
always_comb begin
  if(~$onehot0({_guard222, _guard221})) begin
    $fatal(2, "Multiple assignment to port `depth_plus_4.right'.");
end
end
assign l0_idx_write_en = _guard227;
assign l0_idx_clk = clk;
assign l0_idx_reset = reset;
assign l0_idx_in =
  _guard228 ? 2'd0 :
  _guard231 ? l0_add_out :
  'x;
always_comb begin
  if(~$onehot0({_guard231, _guard228})) begin
    $fatal(2, "Multiple assignment to port `l0_idx.in'.");
end
end
assign idx_between_1_min_depth_4_plus_1_reg_write_en = _guard234;
assign idx_between_1_min_depth_4_plus_1_reg_clk = clk;
assign idx_between_1_min_depth_4_plus_1_reg_reset = reset;
assign idx_between_1_min_depth_4_plus_1_reg_in =
  _guard235 ? 1'd0 :
  _guard236 ? idx_between_1_min_depth_4_plus_1_comb_out :
  'x;
always_comb begin
  if(~$onehot0({_guard236, _guard235})) begin
    $fatal(2, "Multiple assignment to port `idx_between_1_min_depth_4_plus_1_reg.in'.");
end
end
assign idx_between_3_min_depth_4_plus_3_comb_left = index_ge_3_out;
assign idx_between_3_min_depth_4_plus_3_comb_right = index_lt_min_depth_4_plus_3_out;
assign relu_r2_val_gt_left =
  _guard241 ? relu_r2_cur_val_out :
  32'd0;
assign relu_r2_val_gt_right =
  _guard244 ? 32'd0 :
  32'd0;
assign cond11_write_en = _guard245;
assign cond11_clk = clk;
assign cond11_reset = reset;
assign cond11_in =
  _guard246 ? idx_between_1_depth_plus_1_reg_out :
  1'd0;
assign cond15_write_en = _guard247;
assign cond15_clk = clk;
assign cond15_reset = reset;
assign cond15_in =
  _guard248 ? idx_between_3_min_depth_4_plus_3_reg_out :
  1'd0;
assign cond_wire27_in =
  _guard251 ? cond27_out :
  _guard252 ? idx_between_8_depth_plus_8_reg_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard252, _guard251})) begin
    $fatal(2, "Multiple assignment to port `cond_wire27.in'.");
end
end
assign cond_write_en = _guard253;
assign cond_clk = clk;
assign cond_reset = reset;
assign cond_in =
  _guard254 ? idx_between_0_depth_plus_0_reg_out :
  1'd0;
assign pe_0_1_mul_ready =
  _guard257 ? 1'd1 :
  _guard260 ? 1'd0 :
  1'd0;
always_comb begin
  if(~$onehot0({_guard260, _guard257})) begin
    $fatal(2, "Multiple assignment to port `pe_0_1.mul_ready'.");
end
end
assign pe_0_1_clk = clk;
assign pe_0_1_left =
  _guard273 ? left_0_1_out :
  32'd0;
assign pe_0_1_top =
  _guard286 ? top_0_1_out :
  32'd0;
assign pe_0_1_reset = reset;
assign pe_0_1_go = _guard299;
assign left_1_2_write_en = _guard302;
assign left_1_2_clk = clk;
assign left_1_2_reset = reset;
assign left_1_2_in = left_1_1_out;
assign left_2_2_write_en = _guard308;
assign left_2_2_clk = clk;
assign left_2_2_reset = reset;
assign left_2_2_in = left_2_1_out;
assign index_lt_depth_plus_4_left = idx_add_out;
assign index_lt_depth_plus_4_right = depth_plus_4_out;
assign idx_between_5_depth_plus_5_comb_left = index_ge_5_out;
assign idx_between_5_depth_plus_5_comb_right = index_lt_depth_plus_5_out;
assign index_ge_3_left = idx_add_out;
assign index_ge_3_right = 32'd3;
assign relu_r2_val_mult_clk = clk;
assign relu_r2_val_mult_left = 32'd655;
assign relu_r2_val_mult_reset = reset;
assign relu_r2_val_mult_go = _guard326;
assign relu_r2_val_mult_right = relu_r2_cur_val_out;
assign cond_wire26_in =
  _guard330 ? idx_between_4_depth_plus_4_reg_out :
  _guard333 ? cond26_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard333, _guard330})) begin
    $fatal(2, "Multiple assignment to port `cond_wire26.in'.");
end
end
assign depth_plus_5_left = depth;
assign depth_plus_5_right = 32'd5;
assign pe_1_0_mul_ready =
  _guard338 ? 1'd1 :
  _guard341 ? 1'd0 :
  1'd0;
always_comb begin
  if(~$onehot0({_guard341, _guard338})) begin
    $fatal(2, "Multiple assignment to port `pe_1_0.mul_ready'.");
end
end
assign pe_1_0_clk = clk;
assign pe_1_0_left =
  _guard354 ? left_1_0_out :
  32'd0;
assign pe_1_0_top =
  _guard367 ? top_1_0_out :
  32'd0;
assign pe_1_0_reset = reset;
assign pe_1_0_go = _guard380;
assign left_1_0_write_en = _guard383;
assign left_1_0_clk = clk;
assign left_1_0_reset = reset;
assign left_1_0_in = l1_read_data;
assign left_1_1_write_en = _guard389;
assign left_1_1_clk = clk;
assign left_1_1_reset = reset;
assign left_1_1_in = left_1_0_out;
assign top_1_2_write_en = _guard395;
assign top_1_2_clk = clk;
assign top_1_2_reset = reset;
assign top_1_2_in = top_0_2_out;
assign left_2_0_write_en = _guard401;
assign left_2_0_clk = clk;
assign left_2_0_reset = reset;
assign left_2_0_in = l2_read_data;
assign t1_idx_write_en = _guard409;
assign t1_idx_clk = clk;
assign t1_idx_reset = reset;
assign t1_idx_in =
  _guard410 ? 2'd0 :
  _guard413 ? t1_add_out :
  'x;
always_comb begin
  if(~$onehot0({_guard413, _guard410})) begin
    $fatal(2, "Multiple assignment to port `t1_idx.in'.");
end
end
assign relu_r0_cur_idx_write_en = _guard418;
assign relu_r0_cur_idx_clk = clk;
assign relu_r0_cur_idx_reset = reset;
assign relu_r0_cur_idx_in = relu_r0_incr_out;
assign cond18_write_en = _guard424;
assign cond18_clk = clk;
assign cond18_reset = reset;
assign cond18_in =
  _guard425 ? idx_between_4_min_depth_4_plus_4_reg_out :
  1'd0;
assign cond_wire24_in =
  _guard426 ? idx_between_7_depth_plus_7_reg_out :
  _guard429 ? cond24_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard429, _guard426})) begin
    $fatal(2, "Multiple assignment to port `cond_wire24.in'.");
end
end
assign cond_wire31_in =
  _guard430 ? idx_between_depth_plus_6_None_reg_out :
  _guard433 ? cond31_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard433, _guard430})) begin
    $fatal(2, "Multiple assignment to port `cond_wire31.in'.");
end
end
assign wrapper_early_reset_static_par_go_in = _guard439;
assign min_depth_4_plus_2_left = min_depth_4_out;
assign min_depth_4_plus_2_right = 32'd2;
assign depth_plus_1_left = depth;
assign depth_plus_1_right = 32'd1;
assign left_0_0_write_en = _guard446;
assign left_0_0_clk = clk;
assign left_0_0_reset = reset;
assign left_0_0_in = l0_read_data;
assign index_lt_depth_plus_8_left = idx_add_out;
assign index_lt_depth_plus_8_right = depth_plus_8_out;
assign idx_between_depth_plus_7_None_reg_write_en = _guard454;
assign idx_between_depth_plus_7_None_reg_clk = clk;
assign idx_between_depth_plus_7_None_reg_reset = reset;
assign idx_between_depth_plus_7_None_reg_in =
  _guard455 ? 1'd1 :
  _guard456 ? 1'd0 :
  'x;
always_comb begin
  if(~$onehot0({_guard456, _guard455})) begin
    $fatal(2, "Multiple assignment to port `idx_between_depth_plus_7_None_reg.in'.");
end
end
assign cond_wire18_in =
  _guard459 ? cond18_out :
  _guard460 ? idx_between_4_min_depth_4_plus_4_reg_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard460, _guard459})) begin
    $fatal(2, "Multiple assignment to port `cond_wire18.in'.");
end
end
assign cond19_write_en = _guard461;
assign cond19_clk = clk;
assign cond19_reset = reset;
assign cond19_in =
  _guard462 ? idx_between_4_depth_plus_4_reg_out :
  1'd0;
assign cond_wire29_in =
  _guard465 ? cond29_out :
  _guard466 ? idx_between_9_depth_plus_9_reg_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard466, _guard465})) begin
    $fatal(2, "Multiple assignment to port `cond_wire29.in'.");
end
end
assign wrapper_early_reset_static_par_done_in = _guard469;
assign depth_plus_7_left = depth;
assign depth_plus_7_right = 32'd7;
assign pe_0_2_mul_ready =
  _guard474 ? 1'd1 :
  _guard477 ? 1'd0 :
  1'd0;
always_comb begin
  if(~$onehot0({_guard477, _guard474})) begin
    $fatal(2, "Multiple assignment to port `pe_0_2.mul_ready'.");
end
end
assign pe_0_2_clk = clk;
assign pe_0_2_left =
  _guard490 ? left_0_2_out :
  32'd0;
assign pe_0_2_top =
  _guard503 ? top_0_2_out :
  32'd0;
assign pe_0_2_reset = reset;
assign pe_0_2_go = _guard516;
assign t0_idx_write_en = _guard521;
assign t0_idx_clk = clk;
assign t0_idx_reset = reset;
assign t0_idx_in =
  _guard522 ? 2'd0 :
  _guard525 ? t0_add_out :
  'x;
always_comb begin
  if(~$onehot0({_guard525, _guard522})) begin
    $fatal(2, "Multiple assignment to port `t0_idx.in'.");
end
end
assign l1_idx_write_en = _guard530;
assign l1_idx_clk = clk;
assign l1_idx_reset = reset;
assign l1_idx_in =
  _guard533 ? l1_add_out :
  _guard534 ? 2'd0 :
  'x;
always_comb begin
  if(~$onehot0({_guard534, _guard533})) begin
    $fatal(2, "Multiple assignment to port `l1_idx.in'.");
end
end
assign idx_add_left = idx_out;
assign idx_add_right = 32'd1;
assign idx_between_2_min_depth_4_plus_2_comb_left = index_ge_2_out;
assign idx_between_2_min_depth_4_plus_2_comb_right = index_lt_min_depth_4_plus_2_out;
assign index_ge_7_left = idx_add_out;
assign index_ge_7_right = 32'd7;
assign cond3_write_en = _guard541;
assign cond3_clk = clk;
assign cond3_reset = reset;
assign cond3_in =
  _guard542 ? idx_between_1_depth_plus_1_reg_out :
  1'd0;
assign cond13_write_en = _guard543;
assign cond13_clk = clk;
assign cond13_reset = reset;
assign cond13_in =
  _guard544 ? idx_between_2_depth_plus_2_reg_out :
  1'd0;
assign cond29_write_en = _guard545;
assign cond29_clk = clk;
assign cond29_reset = reset;
assign cond29_in =
  _guard546 ? idx_between_9_depth_plus_9_reg_out :
  1'd0;
assign early_reset_static_par0_done_in = ud0_out;
assign tdcc_go_in = go;
assign pe_2_1_mul_ready =
  _guard549 ? 1'd1 :
  _guard552 ? 1'd0 :
  1'd0;
always_comb begin
  if(~$onehot0({_guard552, _guard549})) begin
    $fatal(2, "Multiple assignment to port `pe_2_1.mul_ready'.");
end
end
assign pe_2_1_clk = clk;
assign pe_2_1_left =
  _guard565 ? left_2_1_out :
  32'd0;
assign pe_2_1_top =
  _guard578 ? top_2_1_out :
  32'd0;
assign pe_2_1_reset = reset;
assign pe_2_1_go = _guard591;
assign lt_iter_limit_left =
  _guard592 ? depth :
  _guard593 ? idx_add_out :
  'x;
always_comb begin
  if(~$onehot0({_guard593, _guard592})) begin
    $fatal(2, "Multiple assignment to port `lt_iter_limit.left'.");
end
end
assign lt_iter_limit_right =
  _guard594 ? 32'd4 :
  _guard595 ? iter_limit_out :
  'x;
always_comb begin
  if(~$onehot0({_guard595, _guard594})) begin
    $fatal(2, "Multiple assignment to port `lt_iter_limit.right'.");
end
end
assign idx_between_7_depth_plus_7_comb_left = index_ge_7_out;
assign idx_between_7_depth_plus_7_comb_right = index_lt_depth_plus_7_out;
assign idx_between_depth_plus_6_None_reg_write_en = _guard600;
assign idx_between_depth_plus_6_None_reg_clk = clk;
assign idx_between_depth_plus_6_None_reg_reset = reset;
assign idx_between_depth_plus_6_None_reg_in =
  _guard601 ? 1'd1 :
  _guard602 ? 1'd0 :
  'x;
always_comb begin
  if(~$onehot0({_guard602, _guard601})) begin
    $fatal(2, "Multiple assignment to port `idx_between_depth_plus_6_None_reg.in'.");
end
end
assign idx_between_6_depth_plus_6_comb_left = index_ge_6_out;
assign idx_between_6_depth_plus_6_comb_right = index_lt_depth_plus_6_out;
assign relu_r1_cur_val_in =
  _guard609 ? pe_1_2_out :
  _guard614 ? pe_1_0_out :
  _guard619 ? pe_1_1_out :
  32'd0;
always_comb begin
  if(~$onehot0({_guard619, _guard614, _guard609})) begin
    $fatal(2, "Multiple assignment to port `relu_r1_cur_val.in'.");
end
end
assign relu_r1_go_next_in =
  _guard626 ? 32'd1 :
  32'd0;
assign relu_r1_incr_left = relu_r1_cur_idx_out;
assign relu_r1_incr_right = 32'd1;
assign cond17_write_en = _guard633;
assign cond17_clk = clk;
assign cond17_reset = reset;
assign cond17_in =
  _guard634 ? idx_between_7_depth_plus_7_reg_out :
  1'd0;
assign cond_wire22_in =
  _guard635 ? idx_between_3_min_depth_4_plus_3_reg_out :
  _guard638 ? cond22_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard638, _guard635})) begin
    $fatal(2, "Multiple assignment to port `cond_wire22.in'.");
end
end
assign cond23_write_en = _guard639;
assign cond23_clk = clk;
assign cond23_reset = reset;
assign cond23_in =
  _guard640 ? idx_between_3_depth_plus_3_reg_out :
  1'd0;
assign cond25_write_en = _guard641;
assign cond25_clk = clk;
assign cond25_reset = reset;
assign cond25_in =
  _guard642 ? idx_between_4_min_depth_4_plus_4_reg_out :
  1'd0;
assign fsm0_write_en = _guard655;
assign fsm0_clk = clk;
assign fsm0_reset = reset;
assign fsm0_in =
  _guard660 ? 2'd1 :
  _guard661 ? 2'd0 :
  _guard666 ? 2'd2 :
  2'd0;
always_comb begin
  if(~$onehot0({_guard666, _guard661, _guard660})) begin
    $fatal(2, "Multiple assignment to port `fsm0.in'.");
end
end
assign min_depth_4_plus_4_left = min_depth_4_out;
assign min_depth_4_plus_4_right = 32'd4;
assign depth_plus_8_left = depth;
assign depth_plus_8_right = 32'd8;
assign depth_plus_0_left = depth;
assign depth_plus_0_right = 32'd0;
assign top_2_0_write_en = _guard675;
assign top_2_0_clk = clk;
assign top_2_0_reset = reset;
assign top_2_0_in = top_1_0_out;
assign idx_write_en = _guard681;
assign idx_clk = clk;
assign idx_reset = reset;
assign idx_in =
  _guard682 ? 32'd0 :
  _guard683 ? idx_add_out :
  'x;
always_comb begin
  if(~$onehot0({_guard683, _guard682})) begin
    $fatal(2, "Multiple assignment to port `idx.in'.");
end
end
assign idx_between_4_depth_plus_4_comb_left = index_ge_4_out;
assign idx_between_4_depth_plus_4_comb_right = index_lt_depth_plus_4_out;
assign index_lt_min_depth_4_plus_5_left = idx_add_out;
assign index_lt_min_depth_4_plus_5_right = min_depth_4_plus_5_out;
assign idx_between_7_depth_plus_7_reg_write_en = _guard690;
assign idx_between_7_depth_plus_7_reg_clk = clk;
assign idx_between_7_depth_plus_7_reg_reset = reset;
assign idx_between_7_depth_plus_7_reg_in =
  _guard691 ? idx_between_7_depth_plus_7_comb_out :
  _guard692 ? 1'd0 :
  'x;
always_comb begin
  if(~$onehot0({_guard692, _guard691})) begin
    $fatal(2, "Multiple assignment to port `idx_between_7_depth_plus_7_reg.in'.");
end
end
assign index_lt_depth_plus_0_left = idx_add_out;
assign index_lt_depth_plus_0_right = depth_plus_0_out;
assign idx_between_9_depth_plus_9_reg_write_en = _guard697;
assign idx_between_9_depth_plus_9_reg_clk = clk;
assign idx_between_9_depth_plus_9_reg_reset = reset;
assign idx_between_9_depth_plus_9_reg_in =
  _guard698 ? 1'd0 :
  _guard699 ? idx_between_9_depth_plus_9_comb_out :
  'x;
always_comb begin
  if(~$onehot0({_guard699, _guard698})) begin
    $fatal(2, "Multiple assignment to port `idx_between_9_depth_plus_9_reg.in'.");
end
end
assign idx_between_1_depth_plus_1_comb_left = index_ge_1_out;
assign idx_between_1_depth_plus_1_comb_right = index_lt_depth_plus_1_out;
assign index_lt_depth_plus_3_left = idx_add_out;
assign index_lt_depth_plus_3_right = depth_plus_3_out;
assign relu_r0_incr_left = relu_r0_cur_idx_out;
assign relu_r0_incr_right = 32'd1;
assign relu_r0_val_mult_clk = clk;
assign relu_r0_val_mult_left = 32'd655;
assign relu_r0_val_mult_reset = reset;
assign relu_r0_val_mult_go = _guard718;
assign relu_r0_val_mult_right = relu_r0_cur_val_out;
assign cond7_write_en = _guard722;
assign cond7_clk = clk;
assign cond7_reset = reset;
assign cond7_in =
  _guard723 ? idx_between_2_depth_plus_2_reg_out :
  1'd0;
assign cond_wire8_in =
  _guard724 ? idx_between_3_min_depth_4_plus_3_reg_out :
  _guard727 ? cond8_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard727, _guard724})) begin
    $fatal(2, "Multiple assignment to port `cond_wire8.in'.");
end
end
assign cond9_write_en = _guard728;
assign cond9_clk = clk;
assign cond9_reset = reset;
assign cond9_in =
  _guard729 ? idx_between_3_depth_plus_3_reg_out :
  1'd0;
assign cond14_write_en = _guard730;
assign cond14_clk = clk;
assign cond14_reset = reset;
assign cond14_in =
  _guard731 ? idx_between_6_depth_plus_6_reg_out :
  1'd0;
assign cond_wire20_in =
  _guard734 ? cond20_out :
  _guard735 ? idx_between_8_depth_plus_8_reg_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard735, _guard734})) begin
    $fatal(2, "Multiple assignment to port `cond_wire20.in'.");
end
end
assign cond27_write_en = _guard736;
assign cond27_clk = clk;
assign cond27_reset = reset;
assign cond27_in =
  _guard737 ? idx_between_8_depth_plus_8_reg_out :
  1'd0;
assign cond32_write_en = _guard738;
assign cond32_clk = clk;
assign cond32_reset = reset;
assign cond32_in =
  _guard739 ? idx_between_depth_plus_7_None_reg_out :
  1'd0;
assign adder0_left =
  _guard740 ? fsm_out :
  1'd0;
assign adder0_right = _guard741;
assign early_reset_static_par_done_in = ud_out;
assign min_depth_4_write_en = _guard742;
assign min_depth_4_clk = clk;
assign min_depth_4_reset = reset;
assign min_depth_4_in =
  _guard745 ? depth :
  _guard749 ? 32'd4 :
  'x;
always_comb begin
  if(~$onehot0({_guard749, _guard745})) begin
    $fatal(2, "Multiple assignment to port `min_depth_4.in'.");
end
end
assign top_0_1_write_en = _guard752;
assign top_0_1_clk = clk;
assign top_0_1_reset = reset;
assign top_0_1_in = t1_read_data;
assign left_0_2_write_en = _guard758;
assign left_0_2_clk = clk;
assign left_0_2_reset = reset;
assign left_0_2_in = left_0_1_out;
assign index_ge_4_left = idx_add_out;
assign index_ge_4_right = 32'd4;
assign idx_between_5_min_depth_4_plus_5_reg_write_en = _guard766;
assign idx_between_5_min_depth_4_plus_5_reg_clk = clk;
assign idx_between_5_min_depth_4_plus_5_reg_reset = reset;
assign idx_between_5_min_depth_4_plus_5_reg_in =
  _guard767 ? 1'd0 :
  _guard768 ? idx_between_5_min_depth_4_plus_5_comb_out :
  'x;
always_comb begin
  if(~$onehot0({_guard768, _guard767})) begin
    $fatal(2, "Multiple assignment to port `idx_between_5_min_depth_4_plus_5_reg.in'.");
end
end
assign index_ge_1_left = idx_add_out;
assign index_ge_1_right = 32'd1;
assign idx_between_3_min_depth_4_plus_3_reg_write_en = _guard773;
assign idx_between_3_min_depth_4_plus_3_reg_clk = clk;
assign idx_between_3_min_depth_4_plus_3_reg_reset = reset;
assign idx_between_3_min_depth_4_plus_3_reg_in =
  _guard774 ? idx_between_3_min_depth_4_plus_3_comb_out :
  _guard775 ? 1'd0 :
  'x;
always_comb begin
  if(~$onehot0({_guard775, _guard774})) begin
    $fatal(2, "Multiple assignment to port `idx_between_3_min_depth_4_plus_3_reg.in'.");
end
end
assign cond4_write_en = _guard776;
assign cond4_clk = clk;
assign cond4_reset = reset;
assign cond4_in =
  _guard777 ? idx_between_2_min_depth_4_plus_2_reg_out :
  1'd0;
assign cond5_write_en = _guard778;
assign cond5_clk = clk;
assign cond5_reset = reset;
assign cond5_in =
  _guard779 ? idx_between_2_depth_plus_2_reg_out :
  1'd0;
assign cond_wire6_in =
  _guard782 ? cond6_out :
  _guard783 ? idx_between_6_depth_plus_6_reg_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard783, _guard782})) begin
    $fatal(2, "Multiple assignment to port `cond_wire6.in'.");
end
end
assign cond20_write_en = _guard784;
assign cond20_clk = clk;
assign cond20_reset = reset;
assign cond20_in =
  _guard785 ? idx_between_8_depth_plus_8_reg_out :
  1'd0;
assign signal_reg_write_en = _guard795;
assign signal_reg_clk = clk;
assign signal_reg_reset = reset;
assign signal_reg_in =
  _guard801 ? 1'd1 :
  _guard804 ? 1'd0 :
  1'd0;
always_comb begin
  if(~$onehot0({_guard804, _guard801})) begin
    $fatal(2, "Multiple assignment to port `signal_reg.in'.");
end
end
assign depth_plus_3_left = depth;
assign depth_plus_3_right = 32'd3;
assign top_0_2_write_en = _guard809;
assign top_0_2_clk = clk;
assign top_0_2_reset = reset;
assign top_0_2_in = t2_read_data;
assign top_1_1_write_en = _guard815;
assign top_1_1_clk = clk;
assign top_1_1_reset = reset;
assign top_1_1_in = top_0_1_out;
assign t0_add_left = 2'd1;
assign t0_add_right = t0_idx_out;
assign l0_add_left = 2'd1;
assign l0_add_right = l0_idx_out;
assign index_ge_9_left = idx_add_out;
assign index_ge_9_right = 32'd9;
assign idx_between_1_min_depth_4_plus_1_comb_left = index_ge_1_out;
assign idx_between_1_min_depth_4_plus_1_comb_right = index_lt_min_depth_4_plus_1_out;
assign relu_r0_go_next_in =
  _guard841 ? 32'd1 :
  32'd0;
assign cond_wire2_in =
  _guard844 ? cond2_out :
  _guard845 ? idx_between_5_depth_plus_5_reg_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard845, _guard844})) begin
    $fatal(2, "Multiple assignment to port `cond_wire2.in'.");
end
end
assign cond_wire9_in =
  _guard848 ? cond9_out :
  _guard849 ? idx_between_3_depth_plus_3_reg_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard849, _guard848})) begin
    $fatal(2, "Multiple assignment to port `cond_wire9.in'.");
end
end
assign cond10_write_en = _guard850;
assign cond10_clk = clk;
assign cond10_reset = reset;
assign cond10_in =
  _guard851 ? idx_between_7_depth_plus_7_reg_out :
  1'd0;
assign cond_wire11_in =
  _guard854 ? cond11_out :
  _guard855 ? idx_between_1_depth_plus_1_reg_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard855, _guard854})) begin
    $fatal(2, "Multiple assignment to port `cond_wire11.in'.");
end
end
assign cond21_write_en = _guard856;
assign cond21_clk = clk;
assign cond21_reset = reset;
assign cond21_in =
  _guard857 ? idx_between_2_depth_plus_2_reg_out :
  1'd0;
assign cond24_write_en = _guard858;
assign cond24_clk = clk;
assign cond24_reset = reset;
assign cond24_in =
  _guard859 ? idx_between_7_depth_plus_7_reg_out :
  1'd0;
assign cond31_write_en = _guard860;
assign cond31_clk = clk;
assign cond31_reset = reset;
assign cond31_in =
  _guard861 ? idx_between_depth_plus_6_None_reg_out :
  1'd0;
assign depth_plus_9_left = depth;
assign depth_plus_9_right = 32'd9;
assign depth_plus_6_left = depth;
assign depth_plus_6_right = 32'd6;
assign pe_2_0_mul_ready =
  _guard868 ? 1'd1 :
  _guard871 ? 1'd0 :
  1'd0;
always_comb begin
  if(~$onehot0({_guard871, _guard868})) begin
    $fatal(2, "Multiple assignment to port `pe_2_0.mul_ready'.");
end
end
assign pe_2_0_clk = clk;
assign pe_2_0_left =
  _guard884 ? left_2_0_out :
  32'd0;
assign pe_2_0_top =
  _guard897 ? top_2_0_out :
  32'd0;
assign pe_2_0_reset = reset;
assign pe_2_0_go = _guard910;
assign left_2_1_write_en = _guard913;
assign left_2_1_clk = clk;
assign left_2_1_reset = reset;
assign left_2_1_in = left_2_0_out;
assign top_2_2_write_en = _guard919;
assign top_2_2_clk = clk;
assign top_2_2_reset = reset;
assign top_2_2_in = top_1_2_out;
assign t1_add_left = 2'd1;
assign t1_add_right = t1_idx_out;
assign idx_between_4_min_depth_4_plus_4_comb_left = index_ge_4_out;
assign idx_between_4_min_depth_4_plus_4_comb_right = index_lt_min_depth_4_plus_4_out;
assign idx_between_5_min_depth_4_plus_5_comb_left = index_ge_5_out;
assign idx_between_5_min_depth_4_plus_5_comb_right = index_lt_min_depth_4_plus_5_out;
assign index_ge_2_left = idx_add_out;
assign index_ge_2_right = 32'd2;
assign index_lt_depth_plus_7_left = idx_add_out;
assign index_lt_depth_plus_7_right = depth_plus_7_out;
assign idx_between_0_depth_plus_0_reg_write_en = _guard939;
assign idx_between_0_depth_plus_0_reg_clk = clk;
assign idx_between_0_depth_plus_0_reg_reset = reset;
assign idx_between_0_depth_plus_0_reg_in =
  _guard940 ? 1'd1 :
  _guard941 ? index_lt_depth_plus_0_out :
  'x;
always_comb begin
  if(~$onehot0({_guard941, _guard940})) begin
    $fatal(2, "Multiple assignment to port `idx_between_0_depth_plus_0_reg.in'.");
end
end
assign idx_between_6_depth_plus_6_reg_write_en = _guard944;
assign idx_between_6_depth_plus_6_reg_clk = clk;
assign idx_between_6_depth_plus_6_reg_reset = reset;
assign idx_between_6_depth_plus_6_reg_in =
  _guard945 ? idx_between_6_depth_plus_6_comb_out :
  _guard946 ? 1'd0 :
  'x;
always_comb begin
  if(~$onehot0({_guard946, _guard945})) begin
    $fatal(2, "Multiple assignment to port `idx_between_6_depth_plus_6_reg.in'.");
end
end
assign cond_wire_in =
  _guard949 ? cond_out :
  _guard950 ? idx_between_0_depth_plus_0_reg_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard950, _guard949})) begin
    $fatal(2, "Multiple assignment to port `cond_wire.in'.");
end
end
assign cond_wire15_in =
  _guard953 ? cond15_out :
  _guard954 ? idx_between_3_min_depth_4_plus_3_reg_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard954, _guard953})) begin
    $fatal(2, "Multiple assignment to port `cond_wire15.in'.");
end
end
assign cond16_write_en = _guard955;
assign cond16_clk = clk;
assign cond16_reset = reset;
assign cond16_in =
  _guard956 ? idx_between_3_depth_plus_3_reg_out :
  1'd0;
assign depth_plus_2_left = depth;
assign depth_plus_2_right = 32'd2;
assign pe_0_0_mul_ready =
  _guard961 ? 1'd1 :
  _guard964 ? 1'd0 :
  1'd0;
always_comb begin
  if(~$onehot0({_guard964, _guard961})) begin
    $fatal(2, "Multiple assignment to port `pe_0_0.mul_ready'.");
end
end
assign pe_0_0_clk = clk;
assign pe_0_0_left =
  _guard977 ? left_0_0_out :
  32'd0;
assign pe_0_0_top =
  _guard990 ? top_0_0_out :
  32'd0;
assign pe_0_0_reset = reset;
assign pe_0_0_go = _guard1003;
assign t2_add_left = 2'd1;
assign t2_add_right = t2_idx_out;
assign idx_between_4_depth_plus_4_reg_write_en = _guard1012;
assign idx_between_4_depth_plus_4_reg_clk = clk;
assign idx_between_4_depth_plus_4_reg_reset = reset;
assign idx_between_4_depth_plus_4_reg_in =
  _guard1013 ? idx_between_4_depth_plus_4_comb_out :
  _guard1014 ? 1'd0 :
  'x;
always_comb begin
  if(~$onehot0({_guard1014, _guard1013})) begin
    $fatal(2, "Multiple assignment to port `idx_between_4_depth_plus_4_reg.in'.");
end
end
assign index_lt_min_depth_4_plus_4_left = idx_add_out;
assign index_lt_min_depth_4_plus_4_right = min_depth_4_plus_4_out;
assign idx_between_8_depth_plus_8_reg_write_en = _guard1019;
assign idx_between_8_depth_plus_8_reg_clk = clk;
assign idx_between_8_depth_plus_8_reg_reset = reset;
assign idx_between_8_depth_plus_8_reg_in =
  _guard1020 ? 1'd0 :
  _guard1021 ? idx_between_8_depth_plus_8_comb_out :
  'x;
always_comb begin
  if(~$onehot0({_guard1021, _guard1020})) begin
    $fatal(2, "Multiple assignment to port `idx_between_8_depth_plus_8_reg.in'.");
end
end
assign index_ge_5_left = idx_add_out;
assign index_ge_5_right = 32'd5;
assign idx_between_2_min_depth_4_plus_2_reg_write_en = _guard1026;
assign idx_between_2_min_depth_4_plus_2_reg_clk = clk;
assign idx_between_2_min_depth_4_plus_2_reg_reset = reset;
assign idx_between_2_min_depth_4_plus_2_reg_in =
  _guard1027 ? idx_between_2_min_depth_4_plus_2_comb_out :
  _guard1028 ? 1'd0 :
  'x;
always_comb begin
  if(~$onehot0({_guard1028, _guard1027})) begin
    $fatal(2, "Multiple assignment to port `idx_between_2_min_depth_4_plus_2_reg.in'.");
end
end
assign idx_between_3_depth_plus_3_reg_write_en = _guard1031;
assign idx_between_3_depth_plus_3_reg_clk = clk;
assign idx_between_3_depth_plus_3_reg_reset = reset;
assign idx_between_3_depth_plus_3_reg_in =
  _guard1032 ? 1'd0 :
  _guard1033 ? idx_between_3_depth_plus_3_comb_out :
  'x;
always_comb begin
  if(~$onehot0({_guard1033, _guard1032})) begin
    $fatal(2, "Multiple assignment to port `idx_between_3_depth_plus_3_reg.in'.");
end
end
assign idx_between_2_depth_plus_2_reg_write_en = _guard1036;
assign idx_between_2_depth_plus_2_reg_clk = clk;
assign idx_between_2_depth_plus_2_reg_reset = reset;
assign idx_between_2_depth_plus_2_reg_in =
  _guard1037 ? 1'd0 :
  _guard1038 ? idx_between_2_depth_plus_2_comb_out :
  'x;
always_comb begin
  if(~$onehot0({_guard1038, _guard1037})) begin
    $fatal(2, "Multiple assignment to port `idx_between_2_depth_plus_2_reg.in'.");
end
end
assign idx_between_2_depth_plus_2_comb_left = index_ge_2_out;
assign idx_between_2_depth_plus_2_comb_right = index_lt_depth_plus_2_out;
assign relu_r1_val_mult_clk = clk;
assign relu_r1_val_mult_left = 32'd655;
assign relu_r1_val_mult_reset = reset;
assign relu_r1_val_mult_go = _guard1049;
assign relu_r1_val_mult_right = relu_r1_cur_val_out;
assign cond0_write_en = _guard1053;
assign cond0_clk = clk;
assign cond0_reset = reset;
assign cond0_in =
  _guard1054 ? idx_between_1_min_depth_4_plus_1_reg_out :
  1'd0;
assign cond_wire7_in =
  _guard1057 ? cond7_out :
  _guard1058 ? idx_between_2_depth_plus_2_reg_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard1058, _guard1057})) begin
    $fatal(2, "Multiple assignment to port `cond_wire7.in'.");
end
end
assign cond_wire10_in =
  _guard1059 ? idx_between_7_depth_plus_7_reg_out :
  _guard1062 ? cond10_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard1062, _guard1059})) begin
    $fatal(2, "Multiple assignment to port `cond_wire10.in'.");
end
end
assign cond_wire12_in =
  _guard1063 ? idx_between_2_min_depth_4_plus_2_reg_out :
  _guard1066 ? cond12_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard1066, _guard1063})) begin
    $fatal(2, "Multiple assignment to port `cond_wire12.in'.");
end
end
assign cond_wire17_in =
  _guard1069 ? cond17_out :
  _guard1070 ? idx_between_7_depth_plus_7_reg_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard1070, _guard1069})) begin
    $fatal(2, "Multiple assignment to port `cond_wire17.in'.");
end
end
assign cond_wire19_in =
  _guard1073 ? cond19_out :
  _guard1074 ? idx_between_4_depth_plus_4_reg_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard1074, _guard1073})) begin
    $fatal(2, "Multiple assignment to port `cond_wire19.in'.");
end
end
assign cond_wire23_in =
  _guard1077 ? cond23_out :
  _guard1078 ? idx_between_3_depth_plus_3_reg_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard1078, _guard1077})) begin
    $fatal(2, "Multiple assignment to port `cond_wire23.in'.");
end
end
assign cond_wire25_in =
  _guard1081 ? cond25_out :
  _guard1082 ? idx_between_4_min_depth_4_plus_4_reg_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard1082, _guard1081})) begin
    $fatal(2, "Multiple assignment to port `cond_wire25.in'.");
end
end
assign tdcc_done_in = _guard1083;
assign early_reset_static_par_go_in = _guard1084;
assign top_2_1_write_en = _guard1087;
assign top_2_1_clk = clk;
assign top_2_1_reset = reset;
assign top_2_1_in = top_1_1_out;
assign t2_idx_write_en = _guard1095;
assign t2_idx_clk = clk;
assign t2_idx_reset = reset;
assign t2_idx_in =
  _guard1096 ? 2'd0 :
  _guard1099 ? t2_add_out :
  'x;
always_comb begin
  if(~$onehot0({_guard1099, _guard1096})) begin
    $fatal(2, "Multiple assignment to port `t2_idx.in'.");
end
end
assign l2_idx_write_en = _guard1104;
assign l2_idx_clk = clk;
assign l2_idx_reset = reset;
assign l2_idx_in =
  _guard1105 ? 2'd0 :
  _guard1108 ? l2_add_out :
  'x;
always_comb begin
  if(~$onehot0({_guard1108, _guard1105})) begin
    $fatal(2, "Multiple assignment to port `l2_idx.in'.");
end
end
assign index_lt_depth_plus_5_left = idx_add_out;
assign index_lt_depth_plus_5_right = depth_plus_5_out;
assign idx_between_1_depth_plus_1_reg_write_en = _guard1113;
assign idx_between_1_depth_plus_1_reg_clk = clk;
assign idx_between_1_depth_plus_1_reg_reset = reset;
assign idx_between_1_depth_plus_1_reg_in =
  _guard1114 ? idx_between_1_depth_plus_1_comb_out :
  _guard1115 ? 1'd0 :
  'x;
always_comb begin
  if(~$onehot0({_guard1115, _guard1114})) begin
    $fatal(2, "Multiple assignment to port `idx_between_1_depth_plus_1_reg.in'.");
end
end
assign index_ge_6_left = idx_add_out;
assign index_ge_6_right = 32'd6;
assign relu_r0_cur_val_in =
  _guard1122 ? pe_0_1_out :
  _guard1127 ? pe_0_2_out :
  _guard1132 ? pe_0_0_out :
  32'd0;
always_comb begin
  if(~$onehot0({_guard1132, _guard1127, _guard1122})) begin
    $fatal(2, "Multiple assignment to port `relu_r0_cur_val.in'.");
end
end
assign relu_r2_cur_val_in =
  _guard1137 ? pe_2_1_out :
  _guard1142 ? pe_2_0_out :
  _guard1147 ? pe_2_2_out :
  32'd0;
always_comb begin
  if(~$onehot0({_guard1147, _guard1142, _guard1137})) begin
    $fatal(2, "Multiple assignment to port `relu_r2_cur_val.in'.");
end
end
assign cond1_write_en = _guard1148;
assign cond1_clk = clk;
assign cond1_reset = reset;
assign cond1_in =
  _guard1149 ? idx_between_1_depth_plus_1_reg_out :
  1'd0;
assign cond2_write_en = _guard1150;
assign cond2_clk = clk;
assign cond2_reset = reset;
assign cond2_in =
  _guard1151 ? idx_between_5_depth_plus_5_reg_out :
  1'd0;
assign cond_wire5_in =
  _guard1154 ? cond5_out :
  _guard1155 ? idx_between_2_depth_plus_2_reg_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard1155, _guard1154})) begin
    $fatal(2, "Multiple assignment to port `cond_wire5.in'.");
end
end
assign cond22_write_en = _guard1156;
assign cond22_clk = clk;
assign cond22_reset = reset;
assign cond22_in =
  _guard1157 ? idx_between_3_min_depth_4_plus_3_reg_out :
  1'd0;
assign cond26_write_en = _guard1158;
assign cond26_clk = clk;
assign cond26_reset = reset;
assign cond26_in =
  _guard1159 ? idx_between_4_depth_plus_4_reg_out :
  1'd0;
assign iter_limit_write_en = _guard1160;
assign iter_limit_clk = clk;
assign iter_limit_reset = reset;
assign iter_limit_in = depth_plus_4_out;
assign min_depth_4_plus_1_left = min_depth_4_out;
assign min_depth_4_plus_1_right = 32'd1;
assign min_depth_4_plus_3_left = min_depth_4_out;
assign min_depth_4_plus_3_right = 32'd3;
assign top_0_0_write_en = _guard1168;
assign top_0_0_clk = clk;
assign top_0_0_reset = reset;
assign top_0_0_in = t0_read_data;
assign pe_1_1_mul_ready =
  _guard1174 ? 1'd1 :
  _guard1177 ? 1'd0 :
  1'd0;
always_comb begin
  if(~$onehot0({_guard1177, _guard1174})) begin
    $fatal(2, "Multiple assignment to port `pe_1_1.mul_ready'.");
end
end
assign pe_1_1_clk = clk;
assign pe_1_1_left =
  _guard1190 ? left_1_1_out :
  32'd0;
assign pe_1_1_top =
  _guard1203 ? top_1_1_out :
  32'd0;
assign pe_1_1_reset = reset;
assign pe_1_1_go = _guard1216;
assign idx_between_4_min_depth_4_plus_4_reg_write_en = _guard1219;
assign idx_between_4_min_depth_4_plus_4_reg_clk = clk;
assign idx_between_4_min_depth_4_plus_4_reg_reset = reset;
assign idx_between_4_min_depth_4_plus_4_reg_in =
  _guard1220 ? 1'd0 :
  _guard1221 ? idx_between_4_min_depth_4_plus_4_comb_out :
  'x;
always_comb begin
  if(~$onehot0({_guard1221, _guard1220})) begin
    $fatal(2, "Multiple assignment to port `idx_between_4_min_depth_4_plus_4_reg.in'.");
end
end
assign idx_between_8_depth_plus_8_comb_left = index_ge_8_out;
assign idx_between_8_depth_plus_8_comb_right = index_lt_depth_plus_8_out;
assign index_lt_min_depth_4_plus_1_left = idx_add_out;
assign index_lt_min_depth_4_plus_1_right = min_depth_4_plus_1_out;
assign index_lt_depth_plus_2_left = idx_add_out;
assign index_lt_depth_plus_2_right = depth_plus_2_out;
assign relu_r0_val_gt_left =
  _guard1230 ? relu_r0_cur_val_out :
  32'd0;
assign relu_r0_val_gt_right =
  _guard1233 ? 32'd0 :
  32'd0;
assign relu_r1_val_gt_left =
  _guard1236 ? relu_r1_cur_val_out :
  32'd0;
assign relu_r1_val_gt_right =
  _guard1239 ? 32'd0 :
  32'd0;
assign relu_r2_cur_idx_write_en = _guard1244;
assign relu_r2_cur_idx_clk = clk;
assign relu_r2_cur_idx_reset = reset;
assign relu_r2_cur_idx_in = relu_r2_incr_out;
assign relu_r2_incr_left = relu_r2_cur_idx_out;
assign relu_r2_incr_right = 32'd1;
assign cond_wire1_in =
  _guard1256 ? idx_between_1_depth_plus_1_reg_out :
  _guard1259 ? cond1_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard1259, _guard1256})) begin
    $fatal(2, "Multiple assignment to port `cond_wire1.in'.");
end
end
assign cond8_write_en = _guard1260;
assign cond8_clk = clk;
assign cond8_reset = reset;
assign cond8_in =
  _guard1261 ? idx_between_3_min_depth_4_plus_3_reg_out :
  1'd0;
assign cond12_write_en = _guard1262;
assign cond12_clk = clk;
assign cond12_reset = reset;
assign cond12_in =
  _guard1263 ? idx_between_2_min_depth_4_plus_2_reg_out :
  1'd0;
assign cond28_write_en = _guard1264;
assign cond28_clk = clk;
assign cond28_reset = reset;
assign cond28_in =
  _guard1265 ? idx_between_5_min_depth_4_plus_5_reg_out :
  1'd0;
assign cond_wire28_in =
  _guard1266 ? idx_between_5_min_depth_4_plus_5_reg_out :
  _guard1269 ? cond28_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard1269, _guard1266})) begin
    $fatal(2, "Multiple assignment to port `cond_wire28.in'.");
end
end
assign left_0_1_write_en = _guard1272;
assign left_0_1_clk = clk;
assign left_0_1_reset = reset;
assign left_0_1_in = left_0_0_out;
assign pe_2_2_mul_ready =
  _guard1278 ? 1'd1 :
  _guard1281 ? 1'd0 :
  1'd0;
always_comb begin
  if(~$onehot0({_guard1281, _guard1278})) begin
    $fatal(2, "Multiple assignment to port `pe_2_2.mul_ready'.");
end
end
assign pe_2_2_clk = clk;
assign pe_2_2_left =
  _guard1294 ? left_2_2_out :
  32'd0;
assign pe_2_2_top =
  _guard1307 ? top_2_2_out :
  32'd0;
assign pe_2_2_reset = reset;
assign pe_2_2_go = _guard1320;
assign l2_add_left = 2'd1;
assign l2_add_right = l2_idx_out;
assign cond_reg_write_en = _guard1329;
assign cond_reg_clk = clk;
assign cond_reg_reset = reset;
assign cond_reg_in =
  _guard1330 ? 1'd1 :
  _guard1331 ? lt_iter_limit_out :
  'x;
always_comb begin
  if(~$onehot0({_guard1331, _guard1330})) begin
    $fatal(2, "Multiple assignment to port `cond_reg.in'.");
end
end
assign idx_between_5_depth_plus_5_reg_write_en = _guard1334;
assign idx_between_5_depth_plus_5_reg_clk = clk;
assign idx_between_5_depth_plus_5_reg_reset = reset;
assign idx_between_5_depth_plus_5_reg_in =
  _guard1335 ? idx_between_5_depth_plus_5_comb_out :
  _guard1336 ? 1'd0 :
  'x;
always_comb begin
  if(~$onehot0({_guard1336, _guard1335})) begin
    $fatal(2, "Multiple assignment to port `idx_between_5_depth_plus_5_reg.in'.");
end
end
assign idx_between_9_depth_plus_9_comb_left = index_ge_9_out;
assign idx_between_9_depth_plus_9_comb_right = index_lt_depth_plus_9_out;
assign index_lt_depth_plus_1_left = idx_add_out;
assign index_lt_depth_plus_1_right = depth_plus_1_out;
assign idx_between_3_depth_plus_3_comb_left = index_ge_3_out;
assign idx_between_3_depth_plus_3_comb_right = index_lt_depth_plus_3_out;
assign cond_wire14_in =
  _guard1345 ? cond14_out :
  _guard1346 ? idx_between_6_depth_plus_6_reg_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard1346, _guard1345})) begin
    $fatal(2, "Multiple assignment to port `cond_wire14.in'.");
end
end
assign cond_wire21_in =
  _guard1349 ? cond21_out :
  _guard1350 ? idx_between_2_depth_plus_2_reg_out :
  1'd0;
always_comb begin
  if(~$onehot0({_guard1350, _guard1349})) begin
    $fatal(2, "Multiple assignment to port `cond_wire21.in'.");
end
end
assign while_wrapper_early_reset_static_par0_go_in = _guard1356;
assign while_wrapper_early_reset_static_par0_done_in = _guard1360;
// COMPONENT END: systolic_array_comp
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
    $readmemh({DATA, "/t1.dat"}, t1.mem);
    $readmemh({DATA, "/t2.dat"}, t2.mem);
    $readmemh({DATA, "/l0.dat"}, l0.mem);
    $readmemh({DATA, "/l1.dat"}, l1.mem);
    $readmemh({DATA, "/l2.dat"}, l2.mem);
    $readmemh({DATA, "/out_mem_0.dat"}, out_mem_0.mem);
    $readmemh({DATA, "/out_mem_1.dat"}, out_mem_1.mem);
    $readmemh({DATA, "/out_mem_2.dat"}, out_mem_2.mem);
end
final begin
    $writememh({DATA, "/t0.out"}, t0.mem);
    $writememh({DATA, "/t1.out"}, t1.mem);
    $writememh({DATA, "/t2.out"}, t2.mem);
    $writememh({DATA, "/l0.out"}, l0.mem);
    $writememh({DATA, "/l1.out"}, l1.mem);
    $writememh({DATA, "/l2.out"}, l2.mem);
    $writememh({DATA, "/out_mem_0.out"}, out_mem_0.mem);
    $writememh({DATA, "/out_mem_1.out"}, out_mem_1.mem);
    $writememh({DATA, "/out_mem_2.out"}, out_mem_2.mem);
end
logic [31:0] systolic_array_depth;
logic [31:0] systolic_array_t0_read_data;
logic [31:0] systolic_array_t1_read_data;
logic [31:0] systolic_array_t2_read_data;
logic [31:0] systolic_array_l0_read_data;
logic [31:0] systolic_array_l1_read_data;
logic [31:0] systolic_array_l2_read_data;
logic systolic_array_go;
logic systolic_array_clk;
logic systolic_array_reset;
logic [1:0] systolic_array_t0_addr0;
logic [1:0] systolic_array_t1_addr0;
logic [1:0] systolic_array_t2_addr0;
logic [1:0] systolic_array_l0_addr0;
logic [1:0] systolic_array_l1_addr0;
logic [1:0] systolic_array_l2_addr0;
logic [31:0] systolic_array_out_mem_0_addr0;
logic [31:0] systolic_array_out_mem_0_write_data;
logic systolic_array_out_mem_0_write_en;
logic [31:0] systolic_array_out_mem_1_addr0;
logic [31:0] systolic_array_out_mem_1_write_data;
logic systolic_array_out_mem_1_write_en;
logic [31:0] systolic_array_out_mem_2_addr0;
logic [31:0] systolic_array_out_mem_2_write_data;
logic systolic_array_out_mem_2_write_en;
logic systolic_array_done;
logic [1:0] t0_addr0;
logic [31:0] t0_write_data;
logic t0_write_en;
logic t0_clk;
logic t0_reset;
logic [31:0] t0_read_data;
logic t0_done;
logic [1:0] t1_addr0;
logic [31:0] t1_write_data;
logic t1_write_en;
logic t1_clk;
logic t1_reset;
logic [31:0] t1_read_data;
logic t1_done;
logic [1:0] t2_addr0;
logic [31:0] t2_write_data;
logic t2_write_en;
logic t2_clk;
logic t2_reset;
logic [31:0] t2_read_data;
logic t2_done;
logic [1:0] l0_addr0;
logic [31:0] l0_write_data;
logic l0_write_en;
logic l0_clk;
logic l0_reset;
logic [31:0] l0_read_data;
logic l0_done;
logic [1:0] l1_addr0;
logic [31:0] l1_write_data;
logic l1_write_en;
logic l1_clk;
logic l1_reset;
logic [31:0] l1_read_data;
logic l1_done;
logic [1:0] l2_addr0;
logic [31:0] l2_write_data;
logic l2_write_en;
logic l2_clk;
logic l2_reset;
logic [31:0] l2_read_data;
logic l2_done;
logic [31:0] out_mem_0_addr0;
logic [31:0] out_mem_0_write_data;
logic out_mem_0_write_en;
logic out_mem_0_clk;
logic out_mem_0_reset;
logic [31:0] out_mem_0_read_data;
logic out_mem_0_done;
logic [31:0] out_mem_1_addr0;
logic [31:0] out_mem_1_write_data;
logic out_mem_1_write_en;
logic out_mem_1_clk;
logic out_mem_1_reset;
logic [31:0] out_mem_1_read_data;
logic out_mem_1_done;
logic [31:0] out_mem_2_addr0;
logic [31:0] out_mem_2_write_data;
logic out_mem_2_write_en;
logic out_mem_2_clk;
logic out_mem_2_reset;
logic [31:0] out_mem_2_read_data;
logic out_mem_2_done;
logic invoke0_go_in;
logic invoke0_go_out;
logic invoke0_done_in;
logic invoke0_done_out;
systolic_array_comp systolic_array (
    .clk(systolic_array_clk),
    .depth(systolic_array_depth),
    .done(systolic_array_done),
    .go(systolic_array_go),
    .l0_addr0(systolic_array_l0_addr0),
    .l0_read_data(systolic_array_l0_read_data),
    .l1_addr0(systolic_array_l1_addr0),
    .l1_read_data(systolic_array_l1_read_data),
    .l2_addr0(systolic_array_l2_addr0),
    .l2_read_data(systolic_array_l2_read_data),
    .out_mem_0_addr0(systolic_array_out_mem_0_addr0),
    .out_mem_0_write_data(systolic_array_out_mem_0_write_data),
    .out_mem_0_write_en(systolic_array_out_mem_0_write_en),
    .out_mem_1_addr0(systolic_array_out_mem_1_addr0),
    .out_mem_1_write_data(systolic_array_out_mem_1_write_data),
    .out_mem_1_write_en(systolic_array_out_mem_1_write_en),
    .out_mem_2_addr0(systolic_array_out_mem_2_addr0),
    .out_mem_2_write_data(systolic_array_out_mem_2_write_data),
    .out_mem_2_write_en(systolic_array_out_mem_2_write_en),
    .reset(systolic_array_reset),
    .t0_addr0(systolic_array_t0_addr0),
    .t0_read_data(systolic_array_t0_read_data),
    .t1_addr0(systolic_array_t1_addr0),
    .t1_read_data(systolic_array_t1_read_data),
    .t2_addr0(systolic_array_t2_addr0),
    .t2_read_data(systolic_array_t2_read_data)
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
std_mem_d1 # (
    .IDX_SIZE(2),
    .SIZE(3),
    .WIDTH(32)
) t1 (
    .addr0(t1_addr0),
    .clk(t1_clk),
    .done(t1_done),
    .read_data(t1_read_data),
    .reset(t1_reset),
    .write_data(t1_write_data),
    .write_en(t1_write_en)
);
std_mem_d1 # (
    .IDX_SIZE(2),
    .SIZE(3),
    .WIDTH(32)
) t2 (
    .addr0(t2_addr0),
    .clk(t2_clk),
    .done(t2_done),
    .read_data(t2_read_data),
    .reset(t2_reset),
    .write_data(t2_write_data),
    .write_en(t2_write_en)
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
std_mem_d1 # (
    .IDX_SIZE(2),
    .SIZE(3),
    .WIDTH(32)
) l1 (
    .addr0(l1_addr0),
    .clk(l1_clk),
    .done(l1_done),
    .read_data(l1_read_data),
    .reset(l1_reset),
    .write_data(l1_write_data),
    .write_en(l1_write_en)
);
std_mem_d1 # (
    .IDX_SIZE(2),
    .SIZE(3),
    .WIDTH(32)
) l2 (
    .addr0(l2_addr0),
    .clk(l2_clk),
    .done(l2_done),
    .read_data(l2_read_data),
    .reset(l2_reset),
    .write_data(l2_write_data),
    .write_en(l2_write_en)
);
std_mem_d1 # (
    .IDX_SIZE(32),
    .SIZE(3),
    .WIDTH(32)
) out_mem_0 (
    .addr0(out_mem_0_addr0),
    .clk(out_mem_0_clk),
    .done(out_mem_0_done),
    .read_data(out_mem_0_read_data),
    .reset(out_mem_0_reset),
    .write_data(out_mem_0_write_data),
    .write_en(out_mem_0_write_en)
);
std_mem_d1 # (
    .IDX_SIZE(32),
    .SIZE(3),
    .WIDTH(32)
) out_mem_1 (
    .addr0(out_mem_1_addr0),
    .clk(out_mem_1_clk),
    .done(out_mem_1_done),
    .read_data(out_mem_1_read_data),
    .reset(out_mem_1_reset),
    .write_data(out_mem_1_write_data),
    .write_en(out_mem_1_write_en)
);
std_mem_d1 # (
    .IDX_SIZE(32),
    .SIZE(3),
    .WIDTH(32)
) out_mem_2 (
    .addr0(out_mem_2_addr0),
    .clk(out_mem_2_clk),
    .done(out_mem_2_done),
    .read_data(out_mem_2_read_data),
    .reset(out_mem_2_reset),
    .write_data(out_mem_2_write_data),
    .write_en(out_mem_2_write_en)
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
wire _guard0 = 1;
wire _guard1 = invoke0_go_out;
wire _guard2 = invoke0_go_out;
wire _guard3 = invoke0_done_out;
wire _guard4 = invoke0_go_out;
wire _guard5 = invoke0_go_out;
wire _guard6 = invoke0_go_out;
wire _guard7 = invoke0_go_out;
wire _guard8 = invoke0_go_out;
wire _guard9 = invoke0_go_out;
wire _guard10 = invoke0_go_out;
wire _guard11 = invoke0_go_out;
wire _guard12 = invoke0_go_out;
wire _guard13 = invoke0_go_out;
wire _guard14 = invoke0_go_out;
wire _guard15 = invoke0_go_out;
wire _guard16 = invoke0_go_out;
wire _guard17 = invoke0_go_out;
wire _guard18 = invoke0_go_out;
wire _guard19 = invoke0_go_out;
wire _guard20 = invoke0_go_out;
wire _guard21 = invoke0_go_out;
wire _guard22 = invoke0_go_out;
wire _guard23 = invoke0_go_out;
wire _guard24 = invoke0_go_out;
assign l1_clk = clk;
assign l1_addr0 =
  _guard1 ? systolic_array_l1_addr0 :
  2'd0;
assign l1_reset = reset;
assign l2_clk = clk;
assign l2_addr0 =
  _guard2 ? systolic_array_l2_addr0 :
  2'd0;
assign l2_reset = reset;
assign done = _guard3;
assign t2_clk = clk;
assign t2_addr0 =
  _guard4 ? systolic_array_t2_addr0 :
  2'd0;
assign t2_reset = reset;
assign l0_clk = clk;
assign l0_addr0 =
  _guard5 ? systolic_array_l0_addr0 :
  2'd0;
assign l0_reset = reset;
assign t1_clk = clk;
assign t1_addr0 =
  _guard6 ? systolic_array_t1_addr0 :
  2'd0;
assign t1_reset = reset;
assign invoke0_go_in = go;
assign invoke0_done_in = systolic_array_done;
assign t0_clk = clk;
assign t0_addr0 =
  _guard7 ? systolic_array_t0_addr0 :
  2'd0;
assign t0_reset = reset;
assign systolic_array_l1_read_data =
  _guard8 ? l1_read_data :
  32'd0;
assign systolic_array_l2_read_data =
  _guard9 ? l2_read_data :
  32'd0;
assign systolic_array_depth =
  _guard10 ? 32'd3 :
  32'd0;
assign systolic_array_clk = clk;
assign systolic_array_l0_read_data =
  _guard11 ? l0_read_data :
  32'd0;
assign systolic_array_reset = reset;
assign systolic_array_go = _guard12;
assign systolic_array_t1_read_data =
  _guard13 ? t1_read_data :
  32'd0;
assign systolic_array_t0_read_data =
  _guard14 ? t0_read_data :
  32'd0;
assign systolic_array_t2_read_data =
  _guard15 ? t2_read_data :
  32'd0;
assign out_mem_1_write_en =
  _guard16 ? systolic_array_out_mem_1_write_en :
  1'd0;
assign out_mem_1_clk = clk;
assign out_mem_1_addr0 =
  _guard17 ? systolic_array_out_mem_1_addr0 :
  32'd0;
assign out_mem_1_reset = reset;
assign out_mem_1_write_data = systolic_array_out_mem_1_write_data;
assign out_mem_0_write_en =
  _guard19 ? systolic_array_out_mem_0_write_en :
  1'd0;
assign out_mem_0_clk = clk;
assign out_mem_0_addr0 =
  _guard20 ? systolic_array_out_mem_0_addr0 :
  32'd0;
assign out_mem_0_reset = reset;
assign out_mem_0_write_data = systolic_array_out_mem_0_write_data;
assign out_mem_2_write_en =
  _guard22 ? systolic_array_out_mem_2_write_en :
  1'd0;
assign out_mem_2_clk = clk;
assign out_mem_2_addr0 =
  _guard23 ? systolic_array_out_mem_2_addr0 :
  32'd0;
assign out_mem_2_reset = reset;
assign out_mem_2_write_data = systolic_array_out_mem_2_write_data;
// COMPONENT END: main
endmodule

