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

    always @* begin
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

// Signed extension
module std_signext #(
  parameter IN_WIDTH  = 32,
  parameter OUT_WIDTH = 32
) (
  input wire logic [IN_WIDTH-1:0]  in,
  output logic     [OUT_WIDTH-1:0] out
);
  localparam EXTEND = OUT_WIDTH - IN_WIDTH;
  assign out = { {EXTEND {in[IN_WIDTH-1]}}, in};

  `ifdef VERILATOR
    always_comb begin
      if (IN_WIDTH > OUT_WIDTH)
        $error(
          "std_signext: Output width less than input width\n",
          "IN_WIDTH: %0d", IN_WIDTH,
          "OUT_WIDTH: %0d", OUT_WIDTH
        );
    end
  `endif
endmodule

module std_const_mult #(
    parameter WIDTH = 32,
    parameter VALUE = 1
) (
    input  signed [WIDTH-1:0] in,
    output signed [WIDTH-1:0] out
);
  assign out = in * VALUE;
endmodule

module comb_mem_d1 #(
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
          "comb_mem_d1: Out of bounds access\n",
          "addr0: %0d\n", addr0,
          "SIZE: %0d", SIZE
        );
    end
  `endif
endmodule

module comb_mem_d2 #(
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
          "comb_mem_d2: Out of bounds access\n",
          "addr0: %0d\n", addr0,
          "D0_SIZE: %0d", D0_SIZE
        );
      if (addr1 >= D1_SIZE)
        $error(
          "comb_mem_d2: Out of bounds access\n",
          "addr1: %0d\n", addr1,
          "D1_SIZE: %0d", D1_SIZE
        );
    end
  `endif
endmodule

module comb_mem_d3 #(
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
          "comb_mem_d3: Out of bounds access\n",
          "addr0: %0d\n", addr0,
          "D0_SIZE: %0d", D0_SIZE
        );
      if (addr1 >= D1_SIZE)
        $error(
          "comb_mem_d3: Out of bounds access\n",
          "addr1: %0d\n", addr1,
          "D1_SIZE: %0d", D1_SIZE
        );
      if (addr2 >= D2_SIZE)
        $error(
          "comb_mem_d3: Out of bounds access\n",
          "addr2: %0d\n", addr2,
          "D2_SIZE: %0d", D2_SIZE
        );
    end
  `endif
endmodule

module comb_mem_d4 #(
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
          "comb_mem_d4: Out of bounds access\n",
          "addr0: %0d\n", addr0,
          "D0_SIZE: %0d", D0_SIZE
        );
      if (addr1 >= D1_SIZE)
        $error(
          "comb_mem_d4: Out of bounds access\n",
          "addr1: %0d\n", addr1,
          "D1_SIZE: %0d", D1_SIZE
        );
      if (addr2 >= D2_SIZE)
        $error(
          "comb_mem_d4: Out of bounds access\n",
          "addr2: %0d\n", addr2,
          "D2_SIZE: %0d", D2_SIZE
        );
      if (addr3 >= D3_SIZE)
        $error(
          "comb_mem_d4: Out of bounds access\n",
          "addr3: %0d\n", addr3,
          "D3_SIZE: %0d", D3_SIZE
        );
    end
  `endif
endmodule

/**
 * Core primitives for Calyx.
 * Implements core primitives used by the compiler.
 *
 * Conventions:
 * - All parameter names must be SNAKE_CASE and all caps.
 * - Port names must be snake_case, no caps.
 */

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

module std_bit_slice #(
    parameter IN_WIDTH = 32,
    parameter START_IDX = 0,
    parameter END_IDX = 31,
    parameter OUT_WIDTH = 32
)(
   input wire logic [IN_WIDTH-1:0] in,
   output logic [OUT_WIDTH-1:0] out
);
  assign out = in[END_IDX:START_IDX];

  `ifdef VERILATOR
    always_comb begin
      if (START_IDX < 0 || END_IDX > IN_WIDTH-1)
        $error(
          "std_bit_slice: Slice range out of bounds\n",
          "IN_WIDTH: %0d", IN_WIDTH,
          "START_IDX: %0d", START_IDX,
          "END_IDX: %0d", END_IDX,
        );
    end
  `endif

endmodule

module std_skid_buffer #(
    parameter WIDTH = 32
)(
    input wire logic [WIDTH-1:0] in,
    input wire logic i_valid,
    input wire logic i_ready,
    input wire logic clk,
    input wire logic reset,
    output logic [WIDTH-1:0] out,
    output logic o_valid,
    output logic o_ready
);
  logic [WIDTH-1:0] val;
  logic bypass_rg;
  always @(posedge clk) begin
    // Reset  
    if (reset) begin      
      // Internal Registers
      val <= '0;     
      bypass_rg <= 1'b1;
    end   
    // Out of reset
    else begin      
      // Bypass state      
      if (bypass_rg) begin         
        if (!i_ready && i_valid) begin
          val <= in;          // Data skid happened, store to buffer
          bypass_rg <= 1'b0;  // To skid mode  
        end 
      end 
      // Skid state
      else begin         
        if (i_ready) begin
          bypass_rg <= 1'b1;  // Back to bypass mode           
        end
      end
    end
  end

  assign o_ready = bypass_rg;
  assign out = bypass_rg ? in : val;
  assign o_valid = bypass_rg ? i_valid : 1'b1;
endmodule

module std_bypass_reg #(
    parameter WIDTH = 32
)(
    input wire logic [WIDTH-1:0] in,
    input wire logic write_en,
    input wire logic clk,
    input wire logic reset,
    output logic [WIDTH-1:0] out,
    output logic done
);
  logic [WIDTH-1:0] val;
  assign out = write_en ? in : val;

  always_ff @(posedge clk) begin
    if (reset) begin
      val <= 0;
      done <= 0;
    end else if (write_en) begin
      val <= in;
      done <= 1'd1;
    end else done <= 1'd0;
  end
endmodule

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
   input wire logic [WIDTH-1:0] in,
   output logic [WIDTH-1:0] out
);
assign out = in;
endmodule

module std_add #(
    parameter WIDTH = 32
) (
   input wire logic [WIDTH-1:0] left,
   input wire logic [WIDTH-1:0] right,
   output logic [WIDTH-1:0] out
);
assign out = left + right;
endmodule

module std_lsh #(
    parameter WIDTH = 32
) (
   input wire logic [WIDTH-1:0] left,
   input wire logic [WIDTH-1:0] right,
   output logic [WIDTH-1:0] out
);
assign out = left << right;
endmodule

module std_reg #(
    parameter WIDTH = 32
) (
   input wire logic [WIDTH-1:0] in,
   input wire logic write_en,
   input wire logic clk,
   input wire logic reset,
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

module init_one_reg #(
    parameter WIDTH = 32
) (
   input wire logic [WIDTH-1:0] in,
   input wire logic write_en,
   input wire logic clk,
   input wire logic reset,
   output logic [WIDTH-1:0] out,
   output logic done
);
always_ff @(posedge clk) begin
    if (reset) begin
       out <= 1;
       done <= 0;
    end else if (write_en) begin
      out <= in;
      done <= 1'd1;
    end else done <= 1'd0;
  end
endmodule

module pipelined_mac(
  input logic data_valid,
  input logic [31:0] a,
  input logic [31:0] b,
  input logic [31:0] c,
  output logic [31:0] out,
  output logic output_valid,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: pipelined_mac
logic mult_pipe_clk;
logic mult_pipe_reset;
logic mult_pipe_go;
logic [31:0] mult_pipe_left;
logic [31:0] mult_pipe_right;
logic [31:0] mult_pipe_out;
logic mult_pipe_done;
logic [31:0] add_left;
logic [31:0] add_right;
logic [31:0] add_out;
logic [31:0] pipe1_in;
logic pipe1_write_en;
logic pipe1_clk;
logic pipe1_reset;
logic [31:0] pipe1_out;
logic pipe1_done;
logic [31:0] pipe2_in;
logic pipe2_write_en;
logic pipe2_clk;
logic pipe2_reset;
logic [31:0] pipe2_out;
logic pipe2_done;
logic stage2_valid_in;
logic stage2_valid_write_en;
logic stage2_valid_clk;
logic stage2_valid_reset;
logic stage2_valid_out;
logic stage2_valid_done;
logic out_valid_in;
logic out_valid_write_en;
logic out_valid_clk;
logic out_valid_reset;
logic out_valid_out;
logic out_valid_done;
logic data_valid_reg_in;
logic data_valid_reg_write_en;
logic data_valid_reg_clk;
logic data_valid_reg_reset;
logic data_valid_reg_out;
logic data_valid_reg_done;
logic cond_in;
logic cond_write_en;
logic cond_clk;
logic cond_reset;
logic cond_out;
logic cond_done;
logic cond_wire_in;
logic cond_wire_out;
logic [2:0] fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic [2:0] fsm_out;
logic fsm_done;
logic [2:0] adder_left;
logic [2:0] adder_right;
logic [2:0] adder_out;
logic sig_reg_in;
logic sig_reg_write_en;
logic sig_reg_clk;
logic sig_reg_reset;
logic sig_reg_out;
logic sig_reg_done;
std_mult_pipe # (
    .WIDTH(32)
) mult_pipe (
    .clk(mult_pipe_clk),
    .done(mult_pipe_done),
    .go(mult_pipe_go),
    .left(mult_pipe_left),
    .out(mult_pipe_out),
    .reset(mult_pipe_reset),
    .right(mult_pipe_right)
);
std_add # (
    .WIDTH(32)
) add (
    .left(add_left),
    .out(add_out),
    .right(add_right)
);
std_reg # (
    .WIDTH(32)
) pipe1 (
    .clk(pipe1_clk),
    .done(pipe1_done),
    .in(pipe1_in),
    .out(pipe1_out),
    .reset(pipe1_reset),
    .write_en(pipe1_write_en)
);
std_reg # (
    .WIDTH(32)
) pipe2 (
    .clk(pipe2_clk),
    .done(pipe2_done),
    .in(pipe2_in),
    .out(pipe2_out),
    .reset(pipe2_reset),
    .write_en(pipe2_write_en)
);
std_reg # (
    .WIDTH(1)
) stage2_valid (
    .clk(stage2_valid_clk),
    .done(stage2_valid_done),
    .in(stage2_valid_in),
    .out(stage2_valid_out),
    .reset(stage2_valid_reset),
    .write_en(stage2_valid_write_en)
);
std_reg # (
    .WIDTH(1)
) out_valid (
    .clk(out_valid_clk),
    .done(out_valid_done),
    .in(out_valid_in),
    .out(out_valid_out),
    .reset(out_valid_reset),
    .write_en(out_valid_write_en)
);
std_reg # (
    .WIDTH(1)
) data_valid_reg (
    .clk(data_valid_reg_clk),
    .done(data_valid_reg_done),
    .in(data_valid_reg_in),
    .out(data_valid_reg_out),
    .reset(data_valid_reg_reset),
    .write_en(data_valid_reg_write_en)
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
    .WIDTH(3)
) fsm (
    .clk(fsm_clk),
    .done(fsm_done),
    .in(fsm_in),
    .out(fsm_out),
    .reset(fsm_reset),
    .write_en(fsm_write_en)
);
std_add # (
    .WIDTH(3)
) adder (
    .left(adder_left),
    .out(adder_out),
    .right(adder_right)
);
std_reg # (
    .WIDTH(1)
) sig_reg (
    .clk(sig_reg_clk),
    .done(sig_reg_done),
    .in(sig_reg_in),
    .out(sig_reg_out),
    .reset(sig_reg_reset),
    .write_en(sig_reg_write_en)
);
wire _guard0 = 1;
wire _guard1 = fsm_out == 3'd0;
wire _guard2 = _guard1 & _guard0;
wire _guard3 = sig_reg_out;
wire _guard4 = _guard2 & _guard3;
wire _guard5 = fsm_out == 3'd0;
wire _guard6 = go;
wire _guard7 = _guard5 & _guard6;
wire _guard8 = fsm_out != 3'd0;
wire _guard9 = fsm_out != 3'd5;
wire _guard10 = _guard8 & _guard9;
wire _guard11 = _guard7 | _guard10;
wire _guard12 = fsm_out == 3'd5;
wire _guard13 = _guard11 | _guard12;
wire _guard14 = fsm_out == 3'd0;
wire _guard15 = go;
wire _guard16 = _guard14 & _guard15;
wire _guard17 = fsm_out != 3'd0;
wire _guard18 = fsm_out != 3'd5;
wire _guard19 = _guard17 & _guard18;
wire _guard20 = _guard16 | _guard19;
wire _guard21 = fsm_out == 3'd5;
wire _guard22 = fsm_out == 3'd1;
wire _guard23 = fsm_out >= 3'd1;
wire _guard24 = fsm_out < 3'd5;
wire _guard25 = _guard23 & _guard24;
wire _guard26 = stage2_valid_out;
wire _guard27 = fsm_out == 3'd1;
wire _guard28 = _guard26 & _guard27;
wire _guard29 = stage2_valid_out;
wire _guard30 = fsm_out == 3'd1;
wire _guard31 = _guard29 & _guard30;
wire _guard32 = stage2_valid_out;
wire _guard33 = fsm_out == 3'd5;
wire _guard34 = _guard32 & _guard33;
wire _guard35 = stage2_valid_out;
wire _guard36 = ~_guard35;
wire _guard37 = fsm_out == 3'd5;
wire _guard38 = _guard36 & _guard37;
wire _guard39 = _guard34 | _guard38;
wire _guard40 = stage2_valid_out;
wire _guard41 = fsm_out == 3'd5;
wire _guard42 = _guard40 & _guard41;
wire _guard43 = stage2_valid_out;
wire _guard44 = ~_guard43;
wire _guard45 = fsm_out == 3'd5;
wire _guard46 = _guard44 & _guard45;
wire _guard47 = cond_wire_out;
wire _guard48 = fsm_out >= 3'd1;
wire _guard49 = fsm_out < 3'd4;
wire _guard50 = _guard48 & _guard49;
wire _guard51 = _guard47 & _guard50;
wire _guard52 = cond_wire_out;
wire _guard53 = fsm_out >= 3'd1;
wire _guard54 = fsm_out < 3'd4;
wire _guard55 = _guard53 & _guard54;
wire _guard56 = _guard52 & _guard55;
wire _guard57 = cond_wire_out;
wire _guard58 = fsm_out >= 3'd1;
wire _guard59 = fsm_out < 3'd4;
wire _guard60 = _guard58 & _guard59;
wire _guard61 = _guard57 & _guard60;
wire _guard62 = go;
wire _guard63 = fsm_out == 3'd0;
wire _guard64 = _guard62 & _guard63;
wire _guard65 = go;
wire _guard66 = fsm_out == 3'd0;
wire _guard67 = _guard65 & _guard66;
wire _guard68 = stage2_valid_out;
wire _guard69 = fsm_out == 3'd1;
wire _guard70 = _guard68 & _guard69;
wire _guard71 = stage2_valid_out;
wire _guard72 = fsm_out == 3'd1;
wire _guard73 = _guard71 & _guard72;
wire _guard74 = data_valid_reg_out;
wire _guard75 = fsm_out == 3'd5;
wire _guard76 = _guard74 & _guard75;
wire _guard77 = data_valid_reg_out;
wire _guard78 = ~_guard77;
wire _guard79 = fsm_out == 3'd5;
wire _guard80 = _guard78 & _guard79;
wire _guard81 = _guard76 | _guard80;
wire _guard82 = data_valid_reg_out;
wire _guard83 = fsm_out == 3'd5;
wire _guard84 = _guard82 & _guard83;
wire _guard85 = data_valid_reg_out;
wire _guard86 = ~_guard85;
wire _guard87 = fsm_out == 3'd5;
wire _guard88 = _guard86 & _guard87;
wire _guard89 = fsm_out >= 3'd2;
wire _guard90 = fsm_out < 3'd5;
wire _guard91 = _guard89 & _guard90;
wire _guard92 = fsm_out == 3'd1;
wire _guard93 = cond_wire_out;
wire _guard94 = fsm_out == 3'd4;
wire _guard95 = _guard93 & _guard94;
wire _guard96 = cond_wire_out;
wire _guard97 = fsm_out == 3'd4;
wire _guard98 = _guard96 & _guard97;
wire _guard99 = fsm_out == 3'd0;
wire _guard100 = _guard99 & _guard0;
wire _guard101 = go;
wire _guard102 = go;
wire _guard103 = ~_guard102;
assign done = _guard4;
assign out = pipe2_out;
assign output_valid = out_valid_out;
assign fsm_write_en = _guard13;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard20 ? adder_out :
  _guard21 ? 3'd0 :
  3'd0;
assign adder_left = fsm_out;
assign adder_right = 3'd1;
assign cond_write_en = _guard22;
assign cond_clk = clk;
assign cond_reset = reset;
assign cond_in =
  _guard25 ? data_valid_reg_out :
  1'd0;
assign pipe2_write_en = _guard28;
assign pipe2_clk = clk;
assign pipe2_reset = reset;
assign pipe2_in = add_out;
assign out_valid_write_en = _guard39;
assign out_valid_clk = clk;
assign out_valid_reset = reset;
assign out_valid_in =
  _guard42 ? 1'd1 :
  _guard46 ? 1'd0 :
  'x;
assign mult_pipe_clk = clk;
assign mult_pipe_left = a;
assign mult_pipe_go = _guard56;
assign mult_pipe_reset = reset;
assign mult_pipe_right = b;
assign data_valid_reg_write_en = _guard64;
assign data_valid_reg_clk = clk;
assign data_valid_reg_reset = reset;
assign data_valid_reg_in = data_valid;
assign add_left = pipe1_out;
assign add_right = c;
assign stage2_valid_write_en = _guard81;
assign stage2_valid_clk = clk;
assign stage2_valid_reset = reset;
assign stage2_valid_in =
  _guard84 ? 1'd1 :
  _guard88 ? 1'd0 :
  'x;
assign cond_wire_in =
  _guard91 ? cond_out :
  _guard92 ? data_valid_reg_out :
  1'd0;
assign pipe1_write_en = _guard95;
assign pipe1_clk = clk;
assign pipe1_reset = reset;
assign pipe1_in = mult_pipe_out;
assign sig_reg_write_en = _guard100;
assign sig_reg_clk = clk;
assign sig_reg_reset = reset;
assign sig_reg_in =
  _guard101 ? 1'd1 :
  _guard103 ? 1'd0 :
  1'd0;
// COMPONENT END: pipelined_mac
endmodule
module main(
  input logic go,
  input logic clk,
  input logic reset,
  output logic done,
  output logic [3:0] a_addr0,
  output logic [31:0] a_write_data,
  output logic a_write_en,
  output logic a_clk,
  output logic a_reset,
  input logic [31:0] a_read_data,
  input logic a_done,
  output logic [3:0] b_addr0,
  output logic [31:0] b_write_data,
  output logic b_write_en,
  output logic b_clk,
  output logic b_reset,
  input logic [31:0] b_read_data,
  input logic b_done,
  output logic out_addr0,
  output logic [31:0] out_write_data,
  output logic out_write_en,
  output logic out_clk,
  output logic out_reset,
  input logic [31:0] out_read_data,
  input logic out_done
);
// COMPONENT START: main
logic [31:0] read_a_in;
logic read_a_write_en;
logic read_a_clk;
logic read_a_reset;
logic [31:0] read_a_out;
logic read_a_done;
logic [31:0] read_b_in;
logic read_b_write_en;
logic read_b_clk;
logic read_b_reset;
logic [31:0] read_b_out;
logic read_b_done;
logic [3:0] idx0_in;
logic idx0_write_en;
logic idx0_clk;
logic idx0_reset;
logic [3:0] idx0_out;
logic idx0_done;
logic [3:0] add0_left;
logic [3:0] add0_right;
logic [3:0] add0_out;
logic [3:0] lt0_left;
logic [3:0] lt0_right;
logic lt0_out;
logic mac_data_valid;
logic [31:0] mac_a;
logic [31:0] mac_b;
logic [31:0] mac_c;
logic [31:0] mac_out;
logic mac_output_valid;
logic mac_go;
logic mac_clk;
logic mac_reset;
logic mac_done;
logic comb_reg_in;
logic comb_reg_write_en;
logic comb_reg_clk;
logic comb_reg_reset;
logic comb_reg_out;
logic comb_reg_done;
logic [3:0] fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic [3:0] fsm_out;
logic fsm_done;
logic [3:0] adder_left;
logic [3:0] adder_right;
logic [3:0] adder_out;
logic ud_out;
logic ud0_out;
logic [3:0] adder0_left;
logic [3:0] adder0_right;
logic [3:0] adder0_out;
logic ud1_out;
logic [3:0] adder1_left;
logic [3:0] adder1_right;
logic [3:0] adder1_out;
logic ud2_out;
logic signal_reg_in;
logic signal_reg_write_en;
logic signal_reg_clk;
logic signal_reg_reset;
logic signal_reg_out;
logic signal_reg_done;
logic [2:0] fsm0_in;
logic fsm0_write_en;
logic fsm0_clk;
logic fsm0_reset;
logic [2:0] fsm0_out;
logic fsm0_done;
logic early_reset_static_par_thread0_go_in;
logic early_reset_static_par_thread0_go_out;
logic early_reset_static_par_thread0_done_in;
logic early_reset_static_par_thread0_done_out;
logic early_reset_in_range0_go_in;
logic early_reset_in_range0_go_out;
logic early_reset_in_range0_done_in;
logic early_reset_in_range0_done_out;
logic early_reset_static_seq1_go_in;
logic early_reset_static_seq1_go_out;
logic early_reset_static_seq1_done_in;
logic early_reset_static_seq1_done_out;
logic early_reset_static_seq4_go_in;
logic early_reset_static_seq4_go_out;
logic early_reset_static_seq4_done_in;
logic early_reset_static_seq4_done_out;
logic wrapper_early_reset_static_par_thread0_go_in;
logic wrapper_early_reset_static_par_thread0_go_out;
logic wrapper_early_reset_static_par_thread0_done_in;
logic wrapper_early_reset_static_par_thread0_done_out;
logic wrapper_early_reset_in_range0_go_in;
logic wrapper_early_reset_in_range0_go_out;
logic wrapper_early_reset_in_range0_done_in;
logic wrapper_early_reset_in_range0_done_out;
logic while_wrapper_early_reset_static_seq1_go_in;
logic while_wrapper_early_reset_static_seq1_go_out;
logic while_wrapper_early_reset_static_seq1_done_in;
logic while_wrapper_early_reset_static_seq1_done_out;
logic wrapper_early_reset_static_seq4_go_in;
logic wrapper_early_reset_static_seq4_go_out;
logic wrapper_early_reset_static_seq4_done_in;
logic wrapper_early_reset_static_seq4_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
std_reg # (
    .WIDTH(32)
) read_a (
    .clk(read_a_clk),
    .done(read_a_done),
    .in(read_a_in),
    .out(read_a_out),
    .reset(read_a_reset),
    .write_en(read_a_write_en)
);
std_reg # (
    .WIDTH(32)
) read_b (
    .clk(read_b_clk),
    .done(read_b_done),
    .in(read_b_in),
    .out(read_b_out),
    .reset(read_b_reset),
    .write_en(read_b_write_en)
);
std_reg # (
    .WIDTH(4)
) idx0 (
    .clk(idx0_clk),
    .done(idx0_done),
    .in(idx0_in),
    .out(idx0_out),
    .reset(idx0_reset),
    .write_en(idx0_write_en)
);
std_add # (
    .WIDTH(4)
) add0 (
    .left(add0_left),
    .out(add0_out),
    .right(add0_right)
);
std_lt # (
    .WIDTH(4)
) lt0 (
    .left(lt0_left),
    .out(lt0_out),
    .right(lt0_right)
);
pipelined_mac mac (
    .a(mac_a),
    .b(mac_b),
    .c(mac_c),
    .clk(mac_clk),
    .data_valid(mac_data_valid),
    .done(mac_done),
    .go(mac_go),
    .out(mac_out),
    .output_valid(mac_output_valid),
    .reset(mac_reset)
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
) fsm (
    .clk(fsm_clk),
    .done(fsm_done),
    .in(fsm_in),
    .out(fsm_out),
    .reset(fsm_reset),
    .write_en(fsm_write_en)
);
std_add # (
    .WIDTH(4)
) adder (
    .left(adder_left),
    .out(adder_out),
    .right(adder_right)
);
undef # (
    .WIDTH(1)
) ud (
    .out(ud_out)
);
undef # (
    .WIDTH(1)
) ud0 (
    .out(ud0_out)
);
std_add # (
    .WIDTH(4)
) adder0 (
    .left(adder0_left),
    .out(adder0_out),
    .right(adder0_right)
);
undef # (
    .WIDTH(1)
) ud1 (
    .out(ud1_out)
);
std_add # (
    .WIDTH(4)
) adder1 (
    .left(adder1_left),
    .out(adder1_out),
    .right(adder1_right)
);
undef # (
    .WIDTH(1)
) ud2 (
    .out(ud2_out)
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
    .WIDTH(3)
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
) early_reset_static_par_thread0_go (
    .in(early_reset_static_par_thread0_go_in),
    .out(early_reset_static_par_thread0_go_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_static_par_thread0_done (
    .in(early_reset_static_par_thread0_done_in),
    .out(early_reset_static_par_thread0_done_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_in_range0_go (
    .in(early_reset_in_range0_go_in),
    .out(early_reset_in_range0_go_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_in_range0_done (
    .in(early_reset_in_range0_done_in),
    .out(early_reset_in_range0_done_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_static_seq1_go (
    .in(early_reset_static_seq1_go_in),
    .out(early_reset_static_seq1_go_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_static_seq1_done (
    .in(early_reset_static_seq1_done_in),
    .out(early_reset_static_seq1_done_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_static_seq4_go (
    .in(early_reset_static_seq4_go_in),
    .out(early_reset_static_seq4_go_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_static_seq4_done (
    .in(early_reset_static_seq4_done_in),
    .out(early_reset_static_seq4_done_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_static_par_thread0_go (
    .in(wrapper_early_reset_static_par_thread0_go_in),
    .out(wrapper_early_reset_static_par_thread0_go_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_static_par_thread0_done (
    .in(wrapper_early_reset_static_par_thread0_done_in),
    .out(wrapper_early_reset_static_par_thread0_done_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_in_range0_go (
    .in(wrapper_early_reset_in_range0_go_in),
    .out(wrapper_early_reset_in_range0_go_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_in_range0_done (
    .in(wrapper_early_reset_in_range0_done_in),
    .out(wrapper_early_reset_in_range0_done_out)
);
std_wire # (
    .WIDTH(1)
) while_wrapper_early_reset_static_seq1_go (
    .in(while_wrapper_early_reset_static_seq1_go_in),
    .out(while_wrapper_early_reset_static_seq1_go_out)
);
std_wire # (
    .WIDTH(1)
) while_wrapper_early_reset_static_seq1_done (
    .in(while_wrapper_early_reset_static_seq1_done_in),
    .out(while_wrapper_early_reset_static_seq1_done_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_static_seq4_go (
    .in(wrapper_early_reset_static_seq4_go_in),
    .out(wrapper_early_reset_static_seq4_go_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_static_seq4_done (
    .in(wrapper_early_reset_static_seq4_done_in),
    .out(wrapper_early_reset_static_seq4_done_out)
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
wire _guard1 = early_reset_static_seq4_go_out;
wire _guard2 = early_reset_static_seq4_go_out;
wire _guard3 = signal_reg_out;
wire _guard4 = tdcc_done_out;
wire _guard5 = fsm_out == 4'd6;
wire _guard6 = early_reset_static_seq4_go_out;
wire _guard7 = _guard5 & _guard6;
wire _guard8 = fsm_out == 4'd6;
wire _guard9 = early_reset_static_seq4_go_out;
wire _guard10 = _guard8 & _guard9;
wire _guard11 = fsm_out == 4'd1;
wire _guard12 = early_reset_static_par_thread0_go_out;
wire _guard13 = _guard11 & _guard12;
wire _guard14 = fsm_out == 4'd0;
wire _guard15 = early_reset_static_seq1_go_out;
wire _guard16 = _guard14 & _guard15;
wire _guard17 = _guard13 | _guard16;
wire _guard18 = fsm_out == 4'd6;
wire _guard19 = early_reset_static_seq4_go_out;
wire _guard20 = _guard18 & _guard19;
wire _guard21 = fsm_out == 4'd1;
wire _guard22 = early_reset_static_par_thread0_go_out;
wire _guard23 = _guard21 & _guard22;
wire _guard24 = fsm_out == 4'd0;
wire _guard25 = early_reset_static_seq1_go_out;
wire _guard26 = _guard24 & _guard25;
wire _guard27 = _guard23 | _guard26;
wire _guard28 = fsm_out != 4'd7;
wire _guard29 = early_reset_static_par_thread0_go_out;
wire _guard30 = _guard28 & _guard29;
wire _guard31 = fsm_out == 4'd7;
wire _guard32 = early_reset_static_par_thread0_go_out;
wire _guard33 = _guard31 & _guard32;
wire _guard34 = _guard30 | _guard33;
wire _guard35 = fsm_out != 4'd7;
wire _guard36 = early_reset_static_seq1_go_out;
wire _guard37 = _guard35 & _guard36;
wire _guard38 = _guard34 | _guard37;
wire _guard39 = fsm_out == 4'd7;
wire _guard40 = early_reset_static_seq1_go_out;
wire _guard41 = _guard39 & _guard40;
wire _guard42 = _guard38 | _guard41;
wire _guard43 = fsm_out != 4'd6;
wire _guard44 = early_reset_static_seq4_go_out;
wire _guard45 = _guard43 & _guard44;
wire _guard46 = _guard42 | _guard45;
wire _guard47 = fsm_out == 4'd6;
wire _guard48 = early_reset_static_seq4_go_out;
wire _guard49 = _guard47 & _guard48;
wire _guard50 = _guard46 | _guard49;
wire _guard51 = fsm_out != 4'd6;
wire _guard52 = early_reset_static_seq4_go_out;
wire _guard53 = _guard51 & _guard52;
wire _guard54 = fsm_out != 4'd7;
wire _guard55 = early_reset_static_par_thread0_go_out;
wire _guard56 = _guard54 & _guard55;
wire _guard57 = fsm_out == 4'd7;
wire _guard58 = early_reset_static_par_thread0_go_out;
wire _guard59 = _guard57 & _guard58;
wire _guard60 = fsm_out == 4'd7;
wire _guard61 = early_reset_static_seq1_go_out;
wire _guard62 = _guard60 & _guard61;
wire _guard63 = _guard59 | _guard62;
wire _guard64 = fsm_out == 4'd6;
wire _guard65 = early_reset_static_seq4_go_out;
wire _guard66 = _guard64 & _guard65;
wire _guard67 = _guard63 | _guard66;
wire _guard68 = fsm_out != 4'd7;
wire _guard69 = early_reset_static_seq1_go_out;
wire _guard70 = _guard68 & _guard69;
wire _guard71 = early_reset_static_par_thread0_go_out;
wire _guard72 = early_reset_static_par_thread0_go_out;
wire _guard73 = signal_reg_out;
wire _guard74 = fsm_out == 4'd2;
wire _guard75 = early_reset_static_par_thread0_go_out;
wire _guard76 = _guard74 & _guard75;
wire _guard77 = fsm_out == 4'd1;
wire _guard78 = early_reset_static_seq1_go_out;
wire _guard79 = _guard77 & _guard78;
wire _guard80 = _guard76 | _guard79;
wire _guard81 = fsm_out == 4'd2;
wire _guard82 = early_reset_static_par_thread0_go_out;
wire _guard83 = _guard81 & _guard82;
wire _guard84 = fsm_out == 4'd1;
wire _guard85 = early_reset_static_seq1_go_out;
wire _guard86 = _guard84 & _guard85;
wire _guard87 = _guard83 | _guard86;
wire _guard88 = early_reset_in_range0_go_out;
wire _guard89 = fsm_out == 4'd7;
wire _guard90 = early_reset_static_seq1_go_out;
wire _guard91 = _guard89 & _guard90;
wire _guard92 = _guard88 | _guard91;
wire _guard93 = early_reset_in_range0_go_out;
wire _guard94 = fsm_out == 4'd7;
wire _guard95 = early_reset_static_seq1_go_out;
wire _guard96 = _guard94 & _guard95;
wire _guard97 = _guard93 | _guard96;
wire _guard98 = wrapper_early_reset_static_seq4_go_out;
wire _guard99 = while_wrapper_early_reset_static_seq1_done_out;
wire _guard100 = ~_guard99;
wire _guard101 = fsm0_out == 3'd2;
wire _guard102 = _guard100 & _guard101;
wire _guard103 = tdcc_go_out;
wire _guard104 = _guard102 & _guard103;
wire _guard105 = fsm_out == 4'd1;
wire _guard106 = early_reset_static_par_thread0_go_out;
wire _guard107 = _guard105 & _guard106;
wire _guard108 = fsm_out == 4'd0;
wire _guard109 = early_reset_static_seq1_go_out;
wire _guard110 = _guard108 & _guard109;
wire _guard111 = _guard107 | _guard110;
wire _guard112 = fsm_out == 4'd1;
wire _guard113 = early_reset_static_par_thread0_go_out;
wire _guard114 = _guard112 & _guard113;
wire _guard115 = fsm_out == 4'd0;
wire _guard116 = early_reset_static_seq1_go_out;
wire _guard117 = _guard115 & _guard116;
wire _guard118 = _guard114 | _guard117;
wire _guard119 = fsm_out == 4'd0;
wire _guard120 = fsm_out == 4'd2;
wire _guard121 = _guard119 | _guard120;
wire _guard122 = early_reset_static_par_thread0_go_out;
wire _guard123 = _guard121 & _guard122;
wire _guard124 = fsm_out == 4'd1;
wire _guard125 = early_reset_static_seq1_go_out;
wire _guard126 = _guard124 & _guard125;
wire _guard127 = _guard123 | _guard126;
wire _guard128 = fsm_out == 4'd2;
wire _guard129 = early_reset_static_par_thread0_go_out;
wire _guard130 = _guard128 & _guard129;
wire _guard131 = fsm_out == 4'd1;
wire _guard132 = early_reset_static_seq1_go_out;
wire _guard133 = _guard131 & _guard132;
wire _guard134 = _guard130 | _guard133;
wire _guard135 = fsm_out == 4'd0;
wire _guard136 = early_reset_static_par_thread0_go_out;
wire _guard137 = _guard135 & _guard136;
wire _guard138 = while_wrapper_early_reset_static_seq1_go_out;
wire _guard139 = fsm0_out == 3'd4;
wire _guard140 = fsm0_out == 3'd0;
wire _guard141 = wrapper_early_reset_static_par_thread0_done_out;
wire _guard142 = _guard140 & _guard141;
wire _guard143 = tdcc_go_out;
wire _guard144 = _guard142 & _guard143;
wire _guard145 = _guard139 | _guard144;
wire _guard146 = fsm0_out == 3'd1;
wire _guard147 = wrapper_early_reset_in_range0_done_out;
wire _guard148 = _guard146 & _guard147;
wire _guard149 = tdcc_go_out;
wire _guard150 = _guard148 & _guard149;
wire _guard151 = _guard145 | _guard150;
wire _guard152 = fsm0_out == 3'd2;
wire _guard153 = while_wrapper_early_reset_static_seq1_done_out;
wire _guard154 = _guard152 & _guard153;
wire _guard155 = tdcc_go_out;
wire _guard156 = _guard154 & _guard155;
wire _guard157 = _guard151 | _guard156;
wire _guard158 = fsm0_out == 3'd3;
wire _guard159 = wrapper_early_reset_static_seq4_done_out;
wire _guard160 = _guard158 & _guard159;
wire _guard161 = tdcc_go_out;
wire _guard162 = _guard160 & _guard161;
wire _guard163 = _guard157 | _guard162;
wire _guard164 = fsm0_out == 3'd1;
wire _guard165 = wrapper_early_reset_in_range0_done_out;
wire _guard166 = _guard164 & _guard165;
wire _guard167 = tdcc_go_out;
wire _guard168 = _guard166 & _guard167;
wire _guard169 = fsm0_out == 3'd3;
wire _guard170 = wrapper_early_reset_static_seq4_done_out;
wire _guard171 = _guard169 & _guard170;
wire _guard172 = tdcc_go_out;
wire _guard173 = _guard171 & _guard172;
wire _guard174 = fsm0_out == 3'd0;
wire _guard175 = wrapper_early_reset_static_par_thread0_done_out;
wire _guard176 = _guard174 & _guard175;
wire _guard177 = tdcc_go_out;
wire _guard178 = _guard176 & _guard177;
wire _guard179 = fsm0_out == 3'd4;
wire _guard180 = fsm0_out == 3'd2;
wire _guard181 = while_wrapper_early_reset_static_seq1_done_out;
wire _guard182 = _guard180 & _guard181;
wire _guard183 = tdcc_go_out;
wire _guard184 = _guard182 & _guard183;
wire _guard185 = early_reset_static_seq1_go_out;
wire _guard186 = early_reset_static_seq1_go_out;
wire _guard187 = wrapper_early_reset_static_par_thread0_done_out;
wire _guard188 = ~_guard187;
wire _guard189 = fsm0_out == 3'd0;
wire _guard190 = _guard188 & _guard189;
wire _guard191 = tdcc_go_out;
wire _guard192 = _guard190 & _guard191;
wire _guard193 = fsm_out == 4'd1;
wire _guard194 = early_reset_static_par_thread0_go_out;
wire _guard195 = _guard193 & _guard194;
wire _guard196 = fsm_out == 4'd0;
wire _guard197 = early_reset_static_seq1_go_out;
wire _guard198 = _guard196 & _guard197;
wire _guard199 = _guard195 | _guard198;
wire _guard200 = fsm_out == 4'd1;
wire _guard201 = early_reset_static_par_thread0_go_out;
wire _guard202 = _guard200 & _guard201;
wire _guard203 = fsm_out == 4'd0;
wire _guard204 = early_reset_static_seq1_go_out;
wire _guard205 = _guard203 & _guard204;
wire _guard206 = _guard202 | _guard205;
wire _guard207 = signal_reg_out;
wire _guard208 = fsm_out == 4'd7;
wire _guard209 = _guard208 & _guard0;
wire _guard210 = signal_reg_out;
wire _guard211 = ~_guard210;
wire _guard212 = _guard209 & _guard211;
wire _guard213 = wrapper_early_reset_static_par_thread0_go_out;
wire _guard214 = _guard212 & _guard213;
wire _guard215 = _guard207 | _guard214;
wire _guard216 = _guard0 & _guard0;
wire _guard217 = signal_reg_out;
wire _guard218 = ~_guard217;
wire _guard219 = _guard216 & _guard218;
wire _guard220 = wrapper_early_reset_in_range0_go_out;
wire _guard221 = _guard219 & _guard220;
wire _guard222 = _guard215 | _guard221;
wire _guard223 = fsm_out == 4'd6;
wire _guard224 = _guard223 & _guard0;
wire _guard225 = signal_reg_out;
wire _guard226 = ~_guard225;
wire _guard227 = _guard224 & _guard226;
wire _guard228 = wrapper_early_reset_static_seq4_go_out;
wire _guard229 = _guard227 & _guard228;
wire _guard230 = _guard222 | _guard229;
wire _guard231 = fsm_out == 4'd7;
wire _guard232 = _guard231 & _guard0;
wire _guard233 = signal_reg_out;
wire _guard234 = ~_guard233;
wire _guard235 = _guard232 & _guard234;
wire _guard236 = wrapper_early_reset_static_par_thread0_go_out;
wire _guard237 = _guard235 & _guard236;
wire _guard238 = _guard0 & _guard0;
wire _guard239 = signal_reg_out;
wire _guard240 = ~_guard239;
wire _guard241 = _guard238 & _guard240;
wire _guard242 = wrapper_early_reset_in_range0_go_out;
wire _guard243 = _guard241 & _guard242;
wire _guard244 = _guard237 | _guard243;
wire _guard245 = fsm_out == 4'd6;
wire _guard246 = _guard245 & _guard0;
wire _guard247 = signal_reg_out;
wire _guard248 = ~_guard247;
wire _guard249 = _guard246 & _guard248;
wire _guard250 = wrapper_early_reset_static_seq4_go_out;
wire _guard251 = _guard249 & _guard250;
wire _guard252 = _guard244 | _guard251;
wire _guard253 = signal_reg_out;
wire _guard254 = signal_reg_out;
wire _guard255 = wrapper_early_reset_static_par_thread0_go_out;
wire _guard256 = wrapper_early_reset_in_range0_done_out;
wire _guard257 = ~_guard256;
wire _guard258 = fsm0_out == 3'd1;
wire _guard259 = _guard257 & _guard258;
wire _guard260 = tdcc_go_out;
wire _guard261 = _guard259 & _guard260;
wire _guard262 = fsm0_out == 3'd4;
wire _guard263 = fsm_out >= 4'd2;
wire _guard264 = fsm_out < 4'd8;
wire _guard265 = _guard263 & _guard264;
wire _guard266 = early_reset_static_par_thread0_go_out;
wire _guard267 = _guard265 & _guard266;
wire _guard268 = fsm_out >= 4'd1;
wire _guard269 = fsm_out < 4'd7;
wire _guard270 = _guard268 & _guard269;
wire _guard271 = early_reset_static_seq1_go_out;
wire _guard272 = _guard270 & _guard271;
wire _guard273 = _guard267 | _guard272;
wire _guard274 = fsm_out >= 4'd2;
wire _guard275 = fsm_out < 4'd8;
wire _guard276 = _guard274 & _guard275;
wire _guard277 = early_reset_static_par_thread0_go_out;
wire _guard278 = _guard276 & _guard277;
wire _guard279 = fsm_out >= 4'd1;
wire _guard280 = fsm_out < 4'd7;
wire _guard281 = _guard279 & _guard280;
wire _guard282 = early_reset_static_seq1_go_out;
wire _guard283 = _guard281 & _guard282;
wire _guard284 = _guard278 | _guard283;
wire _guard285 = fsm_out >= 4'd2;
wire _guard286 = fsm_out < 4'd8;
wire _guard287 = _guard285 & _guard286;
wire _guard288 = early_reset_static_par_thread0_go_out;
wire _guard289 = _guard287 & _guard288;
wire _guard290 = fsm_out >= 4'd1;
wire _guard291 = fsm_out < 4'd7;
wire _guard292 = _guard290 & _guard291;
wire _guard293 = early_reset_static_seq1_go_out;
wire _guard294 = _guard292 & _guard293;
wire _guard295 = _guard289 | _guard294;
wire _guard296 = fsm_out >= 4'd2;
wire _guard297 = fsm_out < 4'd8;
wire _guard298 = _guard296 & _guard297;
wire _guard299 = early_reset_static_par_thread0_go_out;
wire _guard300 = _guard298 & _guard299;
wire _guard301 = fsm_out >= 4'd1;
wire _guard302 = fsm_out < 4'd7;
wire _guard303 = _guard301 & _guard302;
wire _guard304 = early_reset_static_seq1_go_out;
wire _guard305 = _guard303 & _guard304;
wire _guard306 = _guard300 | _guard305;
wire _guard307 = fsm_out < 4'd6;
wire _guard308 = early_reset_static_seq4_go_out;
wire _guard309 = _guard307 & _guard308;
wire _guard310 = _guard306 | _guard309;
wire _guard311 = fsm_out >= 4'd1;
wire _guard312 = fsm_out < 4'd7;
wire _guard313 = _guard311 & _guard312;
wire _guard314 = early_reset_static_seq1_go_out;
wire _guard315 = _guard313 & _guard314;
wire _guard316 = fsm_out < 4'd6;
wire _guard317 = early_reset_static_seq4_go_out;
wire _guard318 = _guard316 & _guard317;
wire _guard319 = _guard315 | _guard318;
wire _guard320 = wrapper_early_reset_in_range0_go_out;
wire _guard321 = wrapper_early_reset_static_seq4_done_out;
wire _guard322 = ~_guard321;
wire _guard323 = fsm0_out == 3'd3;
wire _guard324 = _guard322 & _guard323;
wire _guard325 = tdcc_go_out;
wire _guard326 = _guard324 & _guard325;
wire _guard327 = early_reset_in_range0_go_out;
wire _guard328 = fsm_out == 4'd7;
wire _guard329 = early_reset_static_seq1_go_out;
wire _guard330 = _guard328 & _guard329;
wire _guard331 = _guard327 | _guard330;
wire _guard332 = early_reset_in_range0_go_out;
wire _guard333 = fsm_out == 4'd7;
wire _guard334 = early_reset_static_seq1_go_out;
wire _guard335 = _guard333 & _guard334;
wire _guard336 = _guard332 | _guard335;
wire _guard337 = comb_reg_out;
wire _guard338 = ~_guard337;
wire _guard339 = fsm_out == 4'd0;
wire _guard340 = _guard339 & _guard0;
wire _guard341 = _guard338 & _guard340;
assign adder1_left =
  _guard1 ? fsm_out :
  4'd0;
assign adder1_right =
  _guard2 ? 4'd1 :
  4'd0;
assign wrapper_early_reset_in_range0_done_in = _guard3;
assign done = _guard4;
assign out_write_en = _guard7;
assign out_write_data = mac_out;
assign out_reset = reset;
assign b_clk = clk;
assign a_addr0 =
  _guard17 ? idx0_out :
  4'd0;
assign a_write_en = 1'd0;
assign out_addr0 =
  _guard20 ? 1'd0 :
  1'd0;
assign a_reset = reset;
assign b_write_en = 1'd0;
assign b_reset = reset;
assign a_clk = clk;
assign b_addr0 =
  _guard27 ? idx0_out :
  4'd0;
assign out_clk = clk;
assign fsm_write_en = _guard50;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard53 ? adder1_out :
  _guard56 ? adder_out :
  _guard67 ? 4'd0 :
  _guard70 ? adder0_out :
  4'd0;
assign adder_left =
  _guard71 ? fsm_out :
  4'd0;
assign adder_right =
  _guard72 ? 4'd1 :
  4'd0;
assign wrapper_early_reset_static_seq4_done_in = _guard73;
assign add0_left = 4'd1;
assign add0_right = idx0_out;
assign comb_reg_write_en = _guard92;
assign comb_reg_clk = clk;
assign comb_reg_reset = reset;
assign comb_reg_in =
  _guard97 ? lt0_out :
  1'd0;
assign early_reset_static_par_thread0_done_in = ud_out;
assign early_reset_static_seq4_go_in = _guard98;
assign while_wrapper_early_reset_static_seq1_go_in = _guard104;
assign read_b_write_en = _guard111;
assign read_b_clk = clk;
assign read_b_reset = reset;
assign read_b_in = b_read_data;
assign idx0_write_en = _guard127;
assign idx0_clk = clk;
assign idx0_reset = reset;
assign idx0_in =
  _guard134 ? add0_out :
  _guard137 ? 4'd0 :
  'x;
assign early_reset_static_seq1_done_in = ud1_out;
assign early_reset_static_seq1_go_in = _guard138;
assign tdcc_go_in = go;
assign fsm0_write_en = _guard163;
assign fsm0_clk = clk;
assign fsm0_reset = reset;
assign fsm0_in =
  _guard168 ? 3'd2 :
  _guard173 ? 3'd4 :
  _guard178 ? 3'd1 :
  _guard179 ? 3'd0 :
  _guard184 ? 3'd3 :
  3'd0;
assign adder0_left =
  _guard185 ? fsm_out :
  4'd0;
assign adder0_right =
  _guard186 ? 4'd1 :
  4'd0;
assign wrapper_early_reset_static_par_thread0_go_in = _guard192;
assign read_a_write_en = _guard199;
assign read_a_clk = clk;
assign read_a_reset = reset;
assign read_a_in = a_read_data;
assign signal_reg_write_en = _guard230;
assign signal_reg_clk = clk;
assign signal_reg_reset = reset;
assign signal_reg_in =
  _guard252 ? 1'd1 :
  _guard253 ? 1'd0 :
  1'd0;
assign wrapper_early_reset_static_par_thread0_done_in = _guard254;
assign early_reset_static_par_thread0_go_in = _guard255;
assign early_reset_static_seq4_done_in = ud2_out;
assign wrapper_early_reset_in_range0_go_in = _guard261;
assign tdcc_done_in = _guard262;
assign mac_b =
  _guard273 ? read_b_out :
  32'd0;
assign mac_data_valid = _guard284;
assign mac_clk = clk;
assign mac_a =
  _guard295 ? read_a_out :
  32'd0;
assign mac_go = _guard310;
assign mac_reset = reset;
assign mac_c =
  _guard319 ? mac_out :
  32'd0;
assign early_reset_in_range0_go_in = _guard320;
assign wrapper_early_reset_static_seq4_go_in = _guard326;
assign lt0_left =
  _guard331 ? idx0_out :
  4'd0;
assign lt0_right =
  _guard336 ? 4'd10 :
  4'd0;
assign early_reset_in_range0_done_in = ud0_out;
assign while_wrapper_early_reset_static_seq1_done_in = _guard341;
// COMPONENT END: main
endmodule
