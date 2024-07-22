/**
Implements a memory with sequential reads and writes.
- Both reads and writes are not guaranteed to have a given latency.
- Attempting to read and write at the same time is an error.
- The out signal is registered to the last value requested by the read_en signal.
- The out signal is undefined once write_en is asserted.

NOTE(nate): In practice we expect this implementation to be single cycle,
but should not be relied on as such.
In particular we probably eventually want to have `dyn_mems` exist as "virtual operators."
Which have a flexible latency, where the compiler can decide upon actual latency.

See #2111 (PR introducing this: https://github.com/calyxir/calyx/pull/2111)
and a more in depth discussion #1151 (https://github.com/calyxir/calyx/issues/1151)
*/
module dyn_mem_d1 #(
    parameter WIDTH = 32,
    parameter SIZE = 16,
    parameter IDX_SIZE = 4
) (
   // Common signals
   input wire logic clk,
   input wire logic reset,
   input wire logic [IDX_SIZE-1:0] addr0,
   input wire logic content_en,
   output logic done,

   // Read signal
   output logic [ WIDTH-1:0] read_data,

   // Write signals
   input wire logic [ WIDTH-1:0] write_data,
   input wire logic write_en
);
  // Internal memory
  logic [WIDTH-1:0] mem[SIZE-1:0];

  // Register for the read output
  logic [WIDTH-1:0] read_out;
  assign read_data = read_out;

  // Read value from the memory
  always_ff @(posedge clk) begin
    if (reset) begin
      read_out <= '0;
    end else if (content_en && !write_en) begin
      /* verilator lint_off WIDTH */
      read_out <= mem[addr0];
    end else if (content_en && write_en) begin
      // Explicitly clobber the read output when a write is performed
      read_out <= 'x;
    end else begin
      read_out <= read_out;
    end
  end

  // Propagate the done signal
  always_ff @(posedge clk) begin
    if (reset) begin
      done <= '0;
    end else if (content_en) begin
      done <= '1;
    end else begin
      done <= '0;
    end
  end

  // Write value to the memory
  always_ff @(posedge clk) begin
    if (!reset && content_en && write_en)
      mem[addr0] <= write_data;
  end

  // Check for out of bounds access
  `ifdef VERILATOR
    always_comb begin
      if (content_en && !write_en)
        if (addr0 >= SIZE)
          $error(
            "comb_mem_d1: Out of bounds access\n",
            "addr0: %0d\n", addr0,
            "SIZE: %0d", SIZE
          );
    end
  `endif
endmodule

module dyn_mem_d2 #(
    parameter WIDTH = 32,
    parameter D0_SIZE = 16,
    parameter D1_SIZE = 16,
    parameter D0_IDX_SIZE = 4,
    parameter D1_IDX_SIZE = 4
) (
   // Common signals
   input wire logic clk,
   input wire logic reset,
   input wire logic [D0_IDX_SIZE-1:0] addr0,
   input wire logic [D1_IDX_SIZE-1:0] addr1,
   input wire logic content_en,
   output logic done,

   // Read signal
   output logic [WIDTH-1:0] read_data,

   // Write signals
   input wire logic write_en,
   input wire logic [ WIDTH-1:0] write_data
);
  wire [D0_IDX_SIZE+D1_IDX_SIZE-1:0] addr;
  assign addr = addr0 * D1_SIZE + addr1;

  dyn_mem_d1 #(.WIDTH(WIDTH), .SIZE(D0_SIZE * D1_SIZE), .IDX_SIZE(D0_IDX_SIZE+D1_IDX_SIZE)) mem
     (.clk(clk), .reset(reset), .addr0(addr),
    .content_en(content_en), .read_data(read_data), .write_data(write_data), .write_en(write_en),
    .done(done));
endmodule

module dyn_mem_d3 #(
    parameter WIDTH = 32,
    parameter D0_SIZE = 16,
    parameter D1_SIZE = 16,
    parameter D2_SIZE = 16,
    parameter D0_IDX_SIZE = 4,
    parameter D1_IDX_SIZE = 4,
    parameter D2_IDX_SIZE = 4
) (
   // Common signals
   input wire logic clk,
   input wire logic reset,
   input wire logic [D0_IDX_SIZE-1:0] addr0,
   input wire logic [D1_IDX_SIZE-1:0] addr1,
   input wire logic [D2_IDX_SIZE-1:0] addr2,
   input wire logic content_en,
   output logic done,

   // Read signal
   output logic [WIDTH-1:0] read_data,

   // Write signals
   input wire logic write_en,
   input wire logic [ WIDTH-1:0] write_data
);
  wire [D0_IDX_SIZE+D1_IDX_SIZE+D2_IDX_SIZE-1:0] addr;
  assign addr = addr0 * (D1_SIZE * D2_SIZE) + addr1 * (D2_SIZE) + addr2;

  dyn_mem_d1 #(.WIDTH(WIDTH), .SIZE(D0_SIZE * D1_SIZE * D2_SIZE), .IDX_SIZE(D0_IDX_SIZE+D1_IDX_SIZE+D2_IDX_SIZE)) mem
     (.clk(clk), .reset(reset), .addr0(addr),
    .content_en(content_en), .read_data(read_data), .write_data(write_data), .write_en(write_en),
    .done(done));
endmodule

module dyn_mem_d4 #(
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
   // Common signals
   input wire logic clk,
   input wire logic reset,
   input wire logic [D0_IDX_SIZE-1:0] addr0,
   input wire logic [D1_IDX_SIZE-1:0] addr1,
   input wire logic [D2_IDX_SIZE-1:0] addr2,
   input wire logic [D3_IDX_SIZE-1:0] addr3,
   input wire logic content_en,
   output logic done,

   // Read signal
   output logic [WIDTH-1:0] read_data,

   // Write signals
   input wire logic write_en,
   input wire logic [ WIDTH-1:0] write_data
);
  wire [D0_IDX_SIZE+D1_IDX_SIZE+D2_IDX_SIZE+D3_IDX_SIZE-1:0] addr;
  assign addr = addr0 * (D1_SIZE * D2_SIZE * D3_SIZE) + addr1 * (D2_SIZE * D3_SIZE) + addr2 * (D3_SIZE) + addr3;

  dyn_mem_d1 #(.WIDTH(WIDTH), .SIZE(D0_SIZE * D1_SIZE * D2_SIZE * D3_SIZE), .IDX_SIZE(D0_IDX_SIZE+D1_IDX_SIZE+D2_IDX_SIZE+D3_IDX_SIZE)) mem
     (.clk(clk), .reset(reset), .addr0(addr),
    .content_en(content_en), .read_data(read_data), .write_data(write_data), .write_en(write_en),
    .done(done));
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

module m_ar_channel_A0(
  input logic ARESETn,
  input logic ARREADY,
  input logic [63:0] axi_address,
  output logic ARVALID,
  output logic [63:0] ARADDR,
  output logic [2:0] ARSIZE,
  output logic [7:0] ARLEN,
  output logic [1:0] ARBURST,
  output logic [2:0] ARPROT,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: m_ar_channel_A0
logic arvalid_in;
logic arvalid_write_en;
logic arvalid_clk;
logic arvalid_reset;
logic arvalid_out;
logic arvalid_done;
logic ar_handshake_occurred_in;
logic ar_handshake_occurred_write_en;
logic ar_handshake_occurred_clk;
logic ar_handshake_occurred_reset;
logic ar_handshake_occurred_out;
logic ar_handshake_occurred_done;
logic bt_reg_in;
logic bt_reg_write_en;
logic bt_reg_clk;
logic bt_reg_reset;
logic bt_reg_out;
logic bt_reg_done;
logic fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic fsm_out;
logic fsm_done;
logic adder_left;
logic adder_right;
logic adder_out;
logic ud_out;
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
logic do_ar_transfer_go_in;
logic do_ar_transfer_go_out;
logic do_ar_transfer_done_in;
logic do_ar_transfer_done_out;
logic invoke2_go_in;
logic invoke2_go_out;
logic invoke2_done_in;
logic invoke2_done_out;
logic early_reset_static_par_go_in;
logic early_reset_static_par_go_out;
logic early_reset_static_par_done_in;
logic early_reset_static_par_done_out;
logic wrapper_early_reset_static_par_go_in;
logic wrapper_early_reset_static_par_go_out;
logic wrapper_early_reset_static_par_done_in;
logic wrapper_early_reset_static_par_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
std_reg # (
    .WIDTH(1)
) arvalid (
    .clk(arvalid_clk),
    .done(arvalid_done),
    .in(arvalid_in),
    .out(arvalid_out),
    .reset(arvalid_reset),
    .write_en(arvalid_write_en)
);
std_reg # (
    .WIDTH(1)
) ar_handshake_occurred (
    .clk(ar_handshake_occurred_clk),
    .done(ar_handshake_occurred_done),
    .in(ar_handshake_occurred_in),
    .out(ar_handshake_occurred_out),
    .reset(ar_handshake_occurred_reset),
    .write_en(ar_handshake_occurred_write_en)
);
std_reg # (
    .WIDTH(1)
) bt_reg (
    .clk(bt_reg_clk),
    .done(bt_reg_done),
    .in(bt_reg_in),
    .out(bt_reg_out),
    .reset(bt_reg_reset),
    .write_en(bt_reg_write_en)
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
std_add # (
    .WIDTH(1)
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
) do_ar_transfer_go (
    .in(do_ar_transfer_go_in),
    .out(do_ar_transfer_go_out)
);
std_wire # (
    .WIDTH(1)
) do_ar_transfer_done (
    .in(do_ar_transfer_done_in),
    .out(do_ar_transfer_done_out)
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
wire _guard1 = do_ar_transfer_done_out;
wire _guard2 = ~_guard1;
wire _guard3 = fsm0_out == 2'd1;
wire _guard4 = _guard2 & _guard3;
wire _guard5 = tdcc_go_out;
wire _guard6 = _guard4 & _guard5;
wire _guard7 = tdcc_done_out;
wire _guard8 = do_ar_transfer_go_out;
wire _guard9 = do_ar_transfer_go_out;
wire _guard10 = do_ar_transfer_go_out;
wire _guard11 = do_ar_transfer_go_out;
wire _guard12 = do_ar_transfer_go_out;
wire _guard13 = early_reset_static_par_go_out;
wire _guard14 = fsm_out == 1'd0;
wire _guard15 = ~_guard14;
wire _guard16 = early_reset_static_par_go_out;
wire _guard17 = _guard15 & _guard16;
wire _guard18 = fsm_out == 1'd0;
wire _guard19 = early_reset_static_par_go_out;
wire _guard20 = _guard18 & _guard19;
wire _guard21 = early_reset_static_par_go_out;
wire _guard22 = early_reset_static_par_go_out;
wire _guard23 = ar_handshake_occurred_out;
wire _guard24 = ~_guard23;
wire _guard25 = do_ar_transfer_go_out;
wire _guard26 = _guard24 & _guard25;
wire _guard27 = early_reset_static_par_go_out;
wire _guard28 = _guard26 | _guard27;
wire _guard29 = arvalid_out;
wire _guard30 = ARREADY;
wire _guard31 = _guard29 & _guard30;
wire _guard32 = do_ar_transfer_go_out;
wire _guard33 = _guard31 & _guard32;
wire _guard34 = early_reset_static_par_go_out;
wire _guard35 = invoke2_done_out;
wire _guard36 = ~_guard35;
wire _guard37 = fsm0_out == 2'd2;
wire _guard38 = _guard36 & _guard37;
wire _guard39 = tdcc_go_out;
wire _guard40 = _guard38 & _guard39;
wire _guard41 = wrapper_early_reset_static_par_done_out;
wire _guard42 = ~_guard41;
wire _guard43 = fsm0_out == 2'd0;
wire _guard44 = _guard42 & _guard43;
wire _guard45 = tdcc_go_out;
wire _guard46 = _guard44 & _guard45;
wire _guard47 = fsm_out == 1'd0;
wire _guard48 = signal_reg_out;
wire _guard49 = _guard47 & _guard48;
wire _guard50 = fsm0_out == 2'd3;
wire _guard51 = fsm0_out == 2'd0;
wire _guard52 = wrapper_early_reset_static_par_done_out;
wire _guard53 = _guard51 & _guard52;
wire _guard54 = tdcc_go_out;
wire _guard55 = _guard53 & _guard54;
wire _guard56 = _guard50 | _guard55;
wire _guard57 = fsm0_out == 2'd1;
wire _guard58 = do_ar_transfer_done_out;
wire _guard59 = _guard57 & _guard58;
wire _guard60 = tdcc_go_out;
wire _guard61 = _guard59 & _guard60;
wire _guard62 = _guard56 | _guard61;
wire _guard63 = fsm0_out == 2'd2;
wire _guard64 = invoke2_done_out;
wire _guard65 = _guard63 & _guard64;
wire _guard66 = tdcc_go_out;
wire _guard67 = _guard65 & _guard66;
wire _guard68 = _guard62 | _guard67;
wire _guard69 = fsm0_out == 2'd0;
wire _guard70 = wrapper_early_reset_static_par_done_out;
wire _guard71 = _guard69 & _guard70;
wire _guard72 = tdcc_go_out;
wire _guard73 = _guard71 & _guard72;
wire _guard74 = fsm0_out == 2'd3;
wire _guard75 = fsm0_out == 2'd2;
wire _guard76 = invoke2_done_out;
wire _guard77 = _guard75 & _guard76;
wire _guard78 = tdcc_go_out;
wire _guard79 = _guard77 & _guard78;
wire _guard80 = fsm0_out == 2'd1;
wire _guard81 = do_ar_transfer_done_out;
wire _guard82 = _guard80 & _guard81;
wire _guard83 = tdcc_go_out;
wire _guard84 = _guard82 & _guard83;
wire _guard85 = do_ar_transfer_go_out;
wire _guard86 = early_reset_static_par_go_out;
wire _guard87 = _guard85 | _guard86;
wire _guard88 = ARREADY;
wire _guard89 = arvalid_out;
wire _guard90 = _guard88 & _guard89;
wire _guard91 = do_ar_transfer_go_out;
wire _guard92 = _guard90 & _guard91;
wire _guard93 = ARREADY;
wire _guard94 = arvalid_out;
wire _guard95 = _guard93 & _guard94;
wire _guard96 = ~_guard95;
wire _guard97 = do_ar_transfer_go_out;
wire _guard98 = _guard96 & _guard97;
wire _guard99 = early_reset_static_par_go_out;
wire _guard100 = _guard98 | _guard99;
wire _guard101 = fsm_out == 1'd0;
wire _guard102 = signal_reg_out;
wire _guard103 = _guard101 & _guard102;
wire _guard104 = fsm_out == 1'd0;
wire _guard105 = signal_reg_out;
wire _guard106 = ~_guard105;
wire _guard107 = _guard104 & _guard106;
wire _guard108 = wrapper_early_reset_static_par_go_out;
wire _guard109 = _guard107 & _guard108;
wire _guard110 = _guard103 | _guard109;
wire _guard111 = fsm_out == 1'd0;
wire _guard112 = signal_reg_out;
wire _guard113 = ~_guard112;
wire _guard114 = _guard111 & _guard113;
wire _guard115 = wrapper_early_reset_static_par_go_out;
wire _guard116 = _guard114 & _guard115;
wire _guard117 = fsm_out == 1'd0;
wire _guard118 = signal_reg_out;
wire _guard119 = _guard117 & _guard118;
wire _guard120 = do_ar_transfer_go_out;
wire _guard121 = invoke2_go_out;
wire _guard122 = _guard120 | _guard121;
wire _guard123 = arvalid_out;
wire _guard124 = ARREADY;
wire _guard125 = _guard123 & _guard124;
wire _guard126 = ~_guard125;
wire _guard127 = ar_handshake_occurred_out;
wire _guard128 = ~_guard127;
wire _guard129 = _guard126 & _guard128;
wire _guard130 = do_ar_transfer_go_out;
wire _guard131 = _guard129 & _guard130;
wire _guard132 = arvalid_out;
wire _guard133 = ARREADY;
wire _guard134 = _guard132 & _guard133;
wire _guard135 = ar_handshake_occurred_out;
wire _guard136 = _guard134 | _guard135;
wire _guard137 = do_ar_transfer_go_out;
wire _guard138 = _guard136 & _guard137;
wire _guard139 = invoke2_go_out;
wire _guard140 = _guard138 | _guard139;
wire _guard141 = fsm0_out == 2'd3;
wire _guard142 = wrapper_early_reset_static_par_go_out;
assign do_ar_transfer_go_in = _guard6;
assign done = _guard7;
assign ARPROT =
  _guard8 ? 3'd6 :
  3'd0;
assign ARSIZE =
  _guard9 ? 3'd2 :
  3'd0;
assign ARLEN =
  _guard10 ? 8'd0 :
  8'd0;
assign ARADDR =
  _guard11 ? axi_address :
  64'd0;
assign ARBURST =
  _guard12 ? 2'd1 :
  2'd0;
assign ARVALID = arvalid_out;
assign fsm_write_en = _guard13;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard17 ? adder_out :
  _guard20 ? 1'd0 :
  1'd0;
assign adder_left =
  _guard21 ? fsm_out :
  1'd0;
assign adder_right = _guard22;
assign ar_handshake_occurred_write_en = _guard28;
assign ar_handshake_occurred_clk = clk;
assign ar_handshake_occurred_reset = reset;
assign ar_handshake_occurred_in =
  _guard33 ? 1'd1 :
  _guard34 ? 1'd0 :
  'x;
assign invoke2_go_in = _guard40;
assign wrapper_early_reset_static_par_go_in = _guard46;
assign wrapper_early_reset_static_par_done_in = _guard49;
assign tdcc_go_in = go;
assign fsm0_write_en = _guard68;
assign fsm0_clk = clk;
assign fsm0_reset = reset;
assign fsm0_in =
  _guard73 ? 2'd1 :
  _guard74 ? 2'd0 :
  _guard79 ? 2'd3 :
  _guard84 ? 2'd2 :
  2'd0;
assign early_reset_static_par_done_in = ud_out;
assign bt_reg_write_en = _guard87;
assign bt_reg_clk = clk;
assign bt_reg_reset = reset;
assign bt_reg_in =
  _guard92 ? 1'd1 :
  _guard100 ? 1'd0 :
  'x;
assign signal_reg_write_en = _guard110;
assign signal_reg_clk = clk;
assign signal_reg_reset = reset;
assign signal_reg_in =
  _guard116 ? 1'd1 :
  _guard119 ? 1'd0 :
  1'd0;
assign invoke2_done_in = arvalid_done;
assign arvalid_write_en = _guard122;
assign arvalid_clk = clk;
assign arvalid_reset = reset;
assign arvalid_in =
  _guard131 ? 1'd1 :
  _guard140 ? 1'd0 :
  'x;
assign tdcc_done_in = _guard141;
assign early_reset_static_par_go_in = _guard142;
assign do_ar_transfer_done_in = bt_reg_out;
// COMPONENT END: m_ar_channel_A0
endmodule
module m_aw_channel_A0(
  input logic ARESETn,
  input logic AWREADY,
  input logic [63:0] axi_address,
  output logic AWVALID,
  output logic [63:0] AWADDR,
  output logic [2:0] AWSIZE,
  output logic [7:0] AWLEN,
  output logic [1:0] AWBURST,
  output logic [2:0] AWPROT,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: m_aw_channel_A0
logic awvalid_in;
logic awvalid_write_en;
logic awvalid_clk;
logic awvalid_reset;
logic awvalid_out;
logic awvalid_done;
logic aw_handshake_occurred_in;
logic aw_handshake_occurred_write_en;
logic aw_handshake_occurred_clk;
logic aw_handshake_occurred_reset;
logic aw_handshake_occurred_out;
logic aw_handshake_occurred_done;
logic bt_reg_in;
logic bt_reg_write_en;
logic bt_reg_clk;
logic bt_reg_reset;
logic bt_reg_out;
logic bt_reg_done;
logic fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic fsm_out;
logic fsm_done;
logic adder_left;
logic adder_right;
logic adder_out;
logic ud_out;
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
logic do_aw_transfer_go_in;
logic do_aw_transfer_go_out;
logic do_aw_transfer_done_in;
logic do_aw_transfer_done_out;
logic invoke2_go_in;
logic invoke2_go_out;
logic invoke2_done_in;
logic invoke2_done_out;
logic early_reset_static_par_go_in;
logic early_reset_static_par_go_out;
logic early_reset_static_par_done_in;
logic early_reset_static_par_done_out;
logic wrapper_early_reset_static_par_go_in;
logic wrapper_early_reset_static_par_go_out;
logic wrapper_early_reset_static_par_done_in;
logic wrapper_early_reset_static_par_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
std_reg # (
    .WIDTH(1)
) awvalid (
    .clk(awvalid_clk),
    .done(awvalid_done),
    .in(awvalid_in),
    .out(awvalid_out),
    .reset(awvalid_reset),
    .write_en(awvalid_write_en)
);
std_reg # (
    .WIDTH(1)
) aw_handshake_occurred (
    .clk(aw_handshake_occurred_clk),
    .done(aw_handshake_occurred_done),
    .in(aw_handshake_occurred_in),
    .out(aw_handshake_occurred_out),
    .reset(aw_handshake_occurred_reset),
    .write_en(aw_handshake_occurred_write_en)
);
std_reg # (
    .WIDTH(1)
) bt_reg (
    .clk(bt_reg_clk),
    .done(bt_reg_done),
    .in(bt_reg_in),
    .out(bt_reg_out),
    .reset(bt_reg_reset),
    .write_en(bt_reg_write_en)
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
std_add # (
    .WIDTH(1)
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
) do_aw_transfer_go (
    .in(do_aw_transfer_go_in),
    .out(do_aw_transfer_go_out)
);
std_wire # (
    .WIDTH(1)
) do_aw_transfer_done (
    .in(do_aw_transfer_done_in),
    .out(do_aw_transfer_done_out)
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
wire _guard1 = tdcc_done_out;
wire _guard2 = do_aw_transfer_go_out;
wire _guard3 = do_aw_transfer_go_out;
wire _guard4 = do_aw_transfer_go_out;
wire _guard5 = do_aw_transfer_go_out;
wire _guard6 = do_aw_transfer_go_out;
wire _guard7 = early_reset_static_par_go_out;
wire _guard8 = fsm_out == 1'd0;
wire _guard9 = ~_guard8;
wire _guard10 = early_reset_static_par_go_out;
wire _guard11 = _guard9 & _guard10;
wire _guard12 = fsm_out == 1'd0;
wire _guard13 = early_reset_static_par_go_out;
wire _guard14 = _guard12 & _guard13;
wire _guard15 = early_reset_static_par_go_out;
wire _guard16 = early_reset_static_par_go_out;
wire _guard17 = invoke2_done_out;
wire _guard18 = ~_guard17;
wire _guard19 = fsm0_out == 2'd2;
wire _guard20 = _guard18 & _guard19;
wire _guard21 = tdcc_go_out;
wire _guard22 = _guard20 & _guard21;
wire _guard23 = wrapper_early_reset_static_par_done_out;
wire _guard24 = ~_guard23;
wire _guard25 = fsm0_out == 2'd0;
wire _guard26 = _guard24 & _guard25;
wire _guard27 = tdcc_go_out;
wire _guard28 = _guard26 & _guard27;
wire _guard29 = fsm_out == 1'd0;
wire _guard30 = signal_reg_out;
wire _guard31 = _guard29 & _guard30;
wire _guard32 = fsm0_out == 2'd3;
wire _guard33 = fsm0_out == 2'd0;
wire _guard34 = wrapper_early_reset_static_par_done_out;
wire _guard35 = _guard33 & _guard34;
wire _guard36 = tdcc_go_out;
wire _guard37 = _guard35 & _guard36;
wire _guard38 = _guard32 | _guard37;
wire _guard39 = fsm0_out == 2'd1;
wire _guard40 = do_aw_transfer_done_out;
wire _guard41 = _guard39 & _guard40;
wire _guard42 = tdcc_go_out;
wire _guard43 = _guard41 & _guard42;
wire _guard44 = _guard38 | _guard43;
wire _guard45 = fsm0_out == 2'd2;
wire _guard46 = invoke2_done_out;
wire _guard47 = _guard45 & _guard46;
wire _guard48 = tdcc_go_out;
wire _guard49 = _guard47 & _guard48;
wire _guard50 = _guard44 | _guard49;
wire _guard51 = fsm0_out == 2'd0;
wire _guard52 = wrapper_early_reset_static_par_done_out;
wire _guard53 = _guard51 & _guard52;
wire _guard54 = tdcc_go_out;
wire _guard55 = _guard53 & _guard54;
wire _guard56 = fsm0_out == 2'd3;
wire _guard57 = fsm0_out == 2'd2;
wire _guard58 = invoke2_done_out;
wire _guard59 = _guard57 & _guard58;
wire _guard60 = tdcc_go_out;
wire _guard61 = _guard59 & _guard60;
wire _guard62 = fsm0_out == 2'd1;
wire _guard63 = do_aw_transfer_done_out;
wire _guard64 = _guard62 & _guard63;
wire _guard65 = tdcc_go_out;
wire _guard66 = _guard64 & _guard65;
wire _guard67 = do_aw_transfer_done_out;
wire _guard68 = ~_guard67;
wire _guard69 = fsm0_out == 2'd1;
wire _guard70 = _guard68 & _guard69;
wire _guard71 = tdcc_go_out;
wire _guard72 = _guard70 & _guard71;
wire _guard73 = do_aw_transfer_go_out;
wire _guard74 = early_reset_static_par_go_out;
wire _guard75 = _guard73 | _guard74;
wire _guard76 = AWREADY;
wire _guard77 = awvalid_out;
wire _guard78 = _guard76 & _guard77;
wire _guard79 = do_aw_transfer_go_out;
wire _guard80 = _guard78 & _guard79;
wire _guard81 = AWREADY;
wire _guard82 = awvalid_out;
wire _guard83 = _guard81 & _guard82;
wire _guard84 = ~_guard83;
wire _guard85 = do_aw_transfer_go_out;
wire _guard86 = _guard84 & _guard85;
wire _guard87 = early_reset_static_par_go_out;
wire _guard88 = _guard86 | _guard87;
wire _guard89 = fsm_out == 1'd0;
wire _guard90 = signal_reg_out;
wire _guard91 = _guard89 & _guard90;
wire _guard92 = fsm_out == 1'd0;
wire _guard93 = signal_reg_out;
wire _guard94 = ~_guard93;
wire _guard95 = _guard92 & _guard94;
wire _guard96 = wrapper_early_reset_static_par_go_out;
wire _guard97 = _guard95 & _guard96;
wire _guard98 = _guard91 | _guard97;
wire _guard99 = fsm_out == 1'd0;
wire _guard100 = signal_reg_out;
wire _guard101 = ~_guard100;
wire _guard102 = _guard99 & _guard101;
wire _guard103 = wrapper_early_reset_static_par_go_out;
wire _guard104 = _guard102 & _guard103;
wire _guard105 = fsm_out == 1'd0;
wire _guard106 = signal_reg_out;
wire _guard107 = _guard105 & _guard106;
wire _guard108 = aw_handshake_occurred_out;
wire _guard109 = ~_guard108;
wire _guard110 = do_aw_transfer_go_out;
wire _guard111 = _guard109 & _guard110;
wire _guard112 = early_reset_static_par_go_out;
wire _guard113 = _guard111 | _guard112;
wire _guard114 = awvalid_out;
wire _guard115 = AWREADY;
wire _guard116 = _guard114 & _guard115;
wire _guard117 = do_aw_transfer_go_out;
wire _guard118 = _guard116 & _guard117;
wire _guard119 = early_reset_static_par_go_out;
wire _guard120 = fsm0_out == 2'd3;
wire _guard121 = do_aw_transfer_go_out;
wire _guard122 = invoke2_go_out;
wire _guard123 = _guard121 | _guard122;
wire _guard124 = awvalid_out;
wire _guard125 = AWREADY;
wire _guard126 = _guard124 & _guard125;
wire _guard127 = ~_guard126;
wire _guard128 = aw_handshake_occurred_out;
wire _guard129 = ~_guard128;
wire _guard130 = _guard127 & _guard129;
wire _guard131 = do_aw_transfer_go_out;
wire _guard132 = _guard130 & _guard131;
wire _guard133 = awvalid_out;
wire _guard134 = AWREADY;
wire _guard135 = _guard133 & _guard134;
wire _guard136 = aw_handshake_occurred_out;
wire _guard137 = _guard135 | _guard136;
wire _guard138 = do_aw_transfer_go_out;
wire _guard139 = _guard137 & _guard138;
wire _guard140 = invoke2_go_out;
wire _guard141 = _guard139 | _guard140;
wire _guard142 = wrapper_early_reset_static_par_go_out;
assign done = _guard1;
assign AWADDR =
  _guard2 ? axi_address :
  64'd0;
assign AWPROT =
  _guard3 ? 3'd6 :
  3'd0;
assign AWSIZE =
  _guard4 ? 3'd2 :
  3'd0;
assign AWVALID = awvalid_out;
assign AWBURST =
  _guard5 ? 2'd1 :
  2'd0;
assign AWLEN =
  _guard6 ? 8'd0 :
  8'd0;
assign fsm_write_en = _guard7;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard11 ? adder_out :
  _guard14 ? 1'd0 :
  1'd0;
assign adder_left =
  _guard15 ? fsm_out :
  1'd0;
assign adder_right = _guard16;
assign invoke2_go_in = _guard22;
assign wrapper_early_reset_static_par_go_in = _guard28;
assign wrapper_early_reset_static_par_done_in = _guard31;
assign tdcc_go_in = go;
assign fsm0_write_en = _guard50;
assign fsm0_clk = clk;
assign fsm0_reset = reset;
assign fsm0_in =
  _guard55 ? 2'd1 :
  _guard56 ? 2'd0 :
  _guard61 ? 2'd3 :
  _guard66 ? 2'd2 :
  2'd0;
assign do_aw_transfer_go_in = _guard72;
assign do_aw_transfer_done_in = bt_reg_out;
assign early_reset_static_par_done_in = ud_out;
assign bt_reg_write_en = _guard75;
assign bt_reg_clk = clk;
assign bt_reg_reset = reset;
assign bt_reg_in =
  _guard80 ? 1'd1 :
  _guard88 ? 1'd0 :
  'x;
assign signal_reg_write_en = _guard98;
assign signal_reg_clk = clk;
assign signal_reg_reset = reset;
assign signal_reg_in =
  _guard104 ? 1'd1 :
  _guard107 ? 1'd0 :
  1'd0;
assign invoke2_done_in = awvalid_done;
assign aw_handshake_occurred_write_en = _guard113;
assign aw_handshake_occurred_clk = clk;
assign aw_handshake_occurred_reset = reset;
assign aw_handshake_occurred_in =
  _guard118 ? 1'd1 :
  _guard119 ? 1'd0 :
  'x;
assign tdcc_done_in = _guard120;
assign awvalid_write_en = _guard123;
assign awvalid_clk = clk;
assign awvalid_reset = reset;
assign awvalid_in =
  _guard132 ? 1'd1 :
  _guard141 ? 1'd0 :
  'x;
assign early_reset_static_par_go_in = _guard142;
// COMPONENT END: m_aw_channel_A0
endmodule
module m_read_channel_A0(
  input logic ARESETn,
  input logic RVALID,
  input logic RLAST,
  input logic [31:0] RDATA,
  input logic [1:0] RRESP,
  output logic RREADY,
  output logic [31:0] read_data,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: m_read_channel_A0
logic [31:0] read_reg_in;
logic read_reg_write_en;
logic read_reg_clk;
logic read_reg_reset;
logic [31:0] read_reg_out;
logic read_reg_done;
logic rready_in;
logic rready_write_en;
logic rready_clk;
logic rready_reset;
logic rready_out;
logic rready_done;
logic n_RLAST_in;
logic n_RLAST_write_en;
logic n_RLAST_clk;
logic n_RLAST_reset;
logic n_RLAST_out;
logic n_RLAST_done;
logic [1:0] fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic [1:0] fsm_out;
logic fsm_done;
logic block_transfer_go_in;
logic block_transfer_go_out;
logic block_transfer_done_in;
logic block_transfer_done_out;
logic invoke0_go_in;
logic invoke0_go_out;
logic invoke0_done_in;
logic invoke0_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
std_reg # (
    .WIDTH(32)
) read_reg (
    .clk(read_reg_clk),
    .done(read_reg_done),
    .in(read_reg_in),
    .out(read_reg_out),
    .reset(read_reg_reset),
    .write_en(read_reg_write_en)
);
std_reg # (
    .WIDTH(1)
) rready (
    .clk(rready_clk),
    .done(rready_done),
    .in(rready_in),
    .out(rready_out),
    .reset(rready_reset),
    .write_en(rready_write_en)
);
std_reg # (
    .WIDTH(1)
) n_RLAST (
    .clk(n_RLAST_clk),
    .done(n_RLAST_done),
    .in(n_RLAST_in),
    .out(n_RLAST_out),
    .reset(n_RLAST_reset),
    .write_en(n_RLAST_write_en)
);
std_reg # (
    .WIDTH(2)
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
) block_transfer_go (
    .in(block_transfer_go_in),
    .out(block_transfer_go_out)
);
std_wire # (
    .WIDTH(1)
) block_transfer_done (
    .in(block_transfer_done_in),
    .out(block_transfer_done_out)
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
wire _guard1 = tdcc_done_out;
wire _guard2 = fsm_out == 2'd2;
wire _guard3 = fsm_out == 2'd0;
wire _guard4 = invoke0_done_out;
wire _guard5 = n_RLAST_out;
wire _guard6 = _guard4 & _guard5;
wire _guard7 = _guard3 & _guard6;
wire _guard8 = tdcc_go_out;
wire _guard9 = _guard7 & _guard8;
wire _guard10 = _guard2 | _guard9;
wire _guard11 = fsm_out == 2'd1;
wire _guard12 = block_transfer_done_out;
wire _guard13 = n_RLAST_out;
wire _guard14 = _guard12 & _guard13;
wire _guard15 = _guard11 & _guard14;
wire _guard16 = tdcc_go_out;
wire _guard17 = _guard15 & _guard16;
wire _guard18 = _guard10 | _guard17;
wire _guard19 = fsm_out == 2'd0;
wire _guard20 = invoke0_done_out;
wire _guard21 = n_RLAST_out;
wire _guard22 = ~_guard21;
wire _guard23 = _guard20 & _guard22;
wire _guard24 = _guard19 & _guard23;
wire _guard25 = tdcc_go_out;
wire _guard26 = _guard24 & _guard25;
wire _guard27 = _guard18 | _guard26;
wire _guard28 = fsm_out == 2'd1;
wire _guard29 = block_transfer_done_out;
wire _guard30 = n_RLAST_out;
wire _guard31 = ~_guard30;
wire _guard32 = _guard29 & _guard31;
wire _guard33 = _guard28 & _guard32;
wire _guard34 = tdcc_go_out;
wire _guard35 = _guard33 & _guard34;
wire _guard36 = _guard27 | _guard35;
wire _guard37 = fsm_out == 2'd0;
wire _guard38 = invoke0_done_out;
wire _guard39 = n_RLAST_out;
wire _guard40 = _guard38 & _guard39;
wire _guard41 = _guard37 & _guard40;
wire _guard42 = tdcc_go_out;
wire _guard43 = _guard41 & _guard42;
wire _guard44 = fsm_out == 2'd1;
wire _guard45 = block_transfer_done_out;
wire _guard46 = n_RLAST_out;
wire _guard47 = _guard45 & _guard46;
wire _guard48 = _guard44 & _guard47;
wire _guard49 = tdcc_go_out;
wire _guard50 = _guard48 & _guard49;
wire _guard51 = _guard43 | _guard50;
wire _guard52 = fsm_out == 2'd2;
wire _guard53 = fsm_out == 2'd0;
wire _guard54 = invoke0_done_out;
wire _guard55 = n_RLAST_out;
wire _guard56 = ~_guard55;
wire _guard57 = _guard54 & _guard56;
wire _guard58 = _guard53 & _guard57;
wire _guard59 = tdcc_go_out;
wire _guard60 = _guard58 & _guard59;
wire _guard61 = fsm_out == 2'd1;
wire _guard62 = block_transfer_done_out;
wire _guard63 = n_RLAST_out;
wire _guard64 = ~_guard63;
wire _guard65 = _guard62 & _guard64;
wire _guard66 = _guard61 & _guard65;
wire _guard67 = tdcc_go_out;
wire _guard68 = _guard66 & _guard67;
wire _guard69 = _guard60 | _guard68;
wire _guard70 = rready_out;
wire _guard71 = RVALID;
wire _guard72 = _guard70 & _guard71;
wire _guard73 = block_transfer_go_out;
wire _guard74 = _guard72 & _guard73;
wire _guard75 = rready_out;
wire _guard76 = RVALID;
wire _guard77 = _guard75 & _guard76;
wire _guard78 = ~_guard77;
wire _guard79 = block_transfer_go_out;
wire _guard80 = _guard78 & _guard79;
wire _guard81 = block_transfer_go_out;
wire _guard82 = invoke0_done_out;
wire _guard83 = ~_guard82;
wire _guard84 = fsm_out == 2'd0;
wire _guard85 = _guard83 & _guard84;
wire _guard86 = tdcc_go_out;
wire _guard87 = _guard85 & _guard86;
wire _guard88 = block_transfer_go_out;
wire _guard89 = invoke0_go_out;
wire _guard90 = _guard88 | _guard89;
wire _guard91 = RLAST;
wire _guard92 = ~_guard91;
wire _guard93 = block_transfer_go_out;
wire _guard94 = _guard92 & _guard93;
wire _guard95 = invoke0_go_out;
wire _guard96 = _guard94 | _guard95;
wire _guard97 = RLAST;
wire _guard98 = block_transfer_go_out;
wire _guard99 = _guard97 & _guard98;
wire _guard100 = fsm_out == 2'd2;
wire _guard101 = block_transfer_done_out;
wire _guard102 = ~_guard101;
wire _guard103 = fsm_out == 2'd1;
wire _guard104 = _guard102 & _guard103;
wire _guard105 = tdcc_go_out;
wire _guard106 = _guard104 & _guard105;
wire _guard107 = block_transfer_go_out;
wire _guard108 = rready_out;
wire _guard109 = RVALID;
wire _guard110 = _guard108 & _guard109;
wire _guard111 = ~_guard110;
wire _guard112 = block_transfer_go_out;
wire _guard113 = _guard111 & _guard112;
wire _guard114 = rready_out;
wire _guard115 = RVALID;
wire _guard116 = _guard114 & _guard115;
wire _guard117 = block_transfer_go_out;
wire _guard118 = _guard116 & _guard117;
assign done = _guard1;
assign RREADY = rready_out;
assign read_data = read_reg_out;
assign fsm_write_en = _guard36;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard51 ? 2'd1 :
  _guard52 ? 2'd0 :
  _guard69 ? 2'd2 :
  2'd0;
assign block_transfer_done_in = read_reg_done;
assign read_reg_write_en =
  _guard74 ? 1'd1 :
  _guard80 ? 1'd0 :
  1'd0;
assign read_reg_clk = clk;
assign read_reg_reset = reset;
assign read_reg_in = RDATA;
assign tdcc_go_in = go;
assign invoke0_go_in = _guard87;
assign n_RLAST_write_en = _guard90;
assign n_RLAST_clk = clk;
assign n_RLAST_reset = reset;
assign n_RLAST_in =
  _guard96 ? 1'd1 :
  _guard99 ? 1'd0 :
  'x;
assign invoke0_done_in = n_RLAST_done;
assign tdcc_done_in = _guard100;
assign block_transfer_go_in = _guard106;
assign rready_write_en = _guard107;
assign rready_clk = clk;
assign rready_reset = reset;
assign rready_in =
  _guard113 ? 1'd1 :
  _guard118 ? 1'd0 :
  'x;
// COMPONENT END: m_read_channel_A0
endmodule
module m_write_channel_A0(
  input logic ARESETn,
  input logic WREADY,
  input logic [31:0] write_data,
  output logic WVALID,
  output logic WLAST,
  output logic [31:0] WDATA,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: m_write_channel_A0
logic wvalid_in;
logic wvalid_write_en;
logic wvalid_clk;
logic wvalid_reset;
logic wvalid_out;
logic wvalid_done;
logic w_handshake_occurred_in;
logic w_handshake_occurred_write_en;
logic w_handshake_occurred_clk;
logic w_handshake_occurred_reset;
logic w_handshake_occurred_out;
logic w_handshake_occurred_done;
logic bt_reg_in;
logic bt_reg_write_en;
logic bt_reg_clk;
logic bt_reg_reset;
logic bt_reg_out;
logic bt_reg_done;
logic fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic fsm_out;
logic fsm_done;
logic adder_left;
logic adder_right;
logic adder_out;
logic ud_out;
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
logic service_write_transfer_go_in;
logic service_write_transfer_go_out;
logic service_write_transfer_done_in;
logic service_write_transfer_done_out;
logic early_reset_static_par_go_in;
logic early_reset_static_par_go_out;
logic early_reset_static_par_done_in;
logic early_reset_static_par_done_out;
logic wrapper_early_reset_static_par_go_in;
logic wrapper_early_reset_static_par_go_out;
logic wrapper_early_reset_static_par_done_in;
logic wrapper_early_reset_static_par_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
std_reg # (
    .WIDTH(1)
) wvalid (
    .clk(wvalid_clk),
    .done(wvalid_done),
    .in(wvalid_in),
    .out(wvalid_out),
    .reset(wvalid_reset),
    .write_en(wvalid_write_en)
);
std_reg # (
    .WIDTH(1)
) w_handshake_occurred (
    .clk(w_handshake_occurred_clk),
    .done(w_handshake_occurred_done),
    .in(w_handshake_occurred_in),
    .out(w_handshake_occurred_out),
    .reset(w_handshake_occurred_reset),
    .write_en(w_handshake_occurred_write_en)
);
std_reg # (
    .WIDTH(1)
) bt_reg (
    .clk(bt_reg_clk),
    .done(bt_reg_done),
    .in(bt_reg_in),
    .out(bt_reg_out),
    .reset(bt_reg_reset),
    .write_en(bt_reg_write_en)
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
std_add # (
    .WIDTH(1)
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
) service_write_transfer_go (
    .in(service_write_transfer_go_in),
    .out(service_write_transfer_go_out)
);
std_wire # (
    .WIDTH(1)
) service_write_transfer_done (
    .in(service_write_transfer_done_in),
    .out(service_write_transfer_done_out)
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
wire _guard1 = tdcc_done_out;
wire _guard2 = service_write_transfer_go_out;
wire _guard3 = service_write_transfer_go_out;
wire _guard4 = early_reset_static_par_go_out;
wire _guard5 = fsm_out == 1'd0;
wire _guard6 = ~_guard5;
wire _guard7 = early_reset_static_par_go_out;
wire _guard8 = _guard6 & _guard7;
wire _guard9 = fsm_out == 1'd0;
wire _guard10 = early_reset_static_par_go_out;
wire _guard11 = _guard9 & _guard10;
wire _guard12 = early_reset_static_par_go_out;
wire _guard13 = early_reset_static_par_go_out;
wire _guard14 = service_write_transfer_go_out;
wire _guard15 = wvalid_out;
wire _guard16 = WREADY;
wire _guard17 = _guard15 & _guard16;
wire _guard18 = ~_guard17;
wire _guard19 = w_handshake_occurred_out;
wire _guard20 = ~_guard19;
wire _guard21 = _guard18 & _guard20;
wire _guard22 = service_write_transfer_go_out;
wire _guard23 = _guard21 & _guard22;
wire _guard24 = wvalid_out;
wire _guard25 = WREADY;
wire _guard26 = _guard24 & _guard25;
wire _guard27 = w_handshake_occurred_out;
wire _guard28 = _guard26 | _guard27;
wire _guard29 = service_write_transfer_go_out;
wire _guard30 = _guard28 & _guard29;
wire _guard31 = wrapper_early_reset_static_par_done_out;
wire _guard32 = ~_guard31;
wire _guard33 = fsm0_out == 2'd0;
wire _guard34 = _guard32 & _guard33;
wire _guard35 = tdcc_go_out;
wire _guard36 = _guard34 & _guard35;
wire _guard37 = fsm_out == 1'd0;
wire _guard38 = signal_reg_out;
wire _guard39 = _guard37 & _guard38;
wire _guard40 = fsm0_out == 2'd2;
wire _guard41 = fsm0_out == 2'd0;
wire _guard42 = wrapper_early_reset_static_par_done_out;
wire _guard43 = _guard41 & _guard42;
wire _guard44 = tdcc_go_out;
wire _guard45 = _guard43 & _guard44;
wire _guard46 = _guard40 | _guard45;
wire _guard47 = fsm0_out == 2'd1;
wire _guard48 = service_write_transfer_done_out;
wire _guard49 = _guard47 & _guard48;
wire _guard50 = tdcc_go_out;
wire _guard51 = _guard49 & _guard50;
wire _guard52 = _guard46 | _guard51;
wire _guard53 = fsm0_out == 2'd0;
wire _guard54 = wrapper_early_reset_static_par_done_out;
wire _guard55 = _guard53 & _guard54;
wire _guard56 = tdcc_go_out;
wire _guard57 = _guard55 & _guard56;
wire _guard58 = fsm0_out == 2'd2;
wire _guard59 = fsm0_out == 2'd1;
wire _guard60 = service_write_transfer_done_out;
wire _guard61 = _guard59 & _guard60;
wire _guard62 = tdcc_go_out;
wire _guard63 = _guard61 & _guard62;
wire _guard64 = service_write_transfer_done_out;
wire _guard65 = ~_guard64;
wire _guard66 = fsm0_out == 2'd1;
wire _guard67 = _guard65 & _guard66;
wire _guard68 = tdcc_go_out;
wire _guard69 = _guard67 & _guard68;
wire _guard70 = service_write_transfer_go_out;
wire _guard71 = early_reset_static_par_go_out;
wire _guard72 = _guard70 | _guard71;
wire _guard73 = wvalid_out;
wire _guard74 = WREADY;
wire _guard75 = _guard73 & _guard74;
wire _guard76 = service_write_transfer_go_out;
wire _guard77 = _guard75 & _guard76;
wire _guard78 = wvalid_out;
wire _guard79 = WREADY;
wire _guard80 = _guard78 & _guard79;
wire _guard81 = ~_guard80;
wire _guard82 = service_write_transfer_go_out;
wire _guard83 = _guard81 & _guard82;
wire _guard84 = early_reset_static_par_go_out;
wire _guard85 = _guard83 | _guard84;
wire _guard86 = fsm_out == 1'd0;
wire _guard87 = signal_reg_out;
wire _guard88 = _guard86 & _guard87;
wire _guard89 = fsm_out == 1'd0;
wire _guard90 = signal_reg_out;
wire _guard91 = ~_guard90;
wire _guard92 = _guard89 & _guard91;
wire _guard93 = wrapper_early_reset_static_par_go_out;
wire _guard94 = _guard92 & _guard93;
wire _guard95 = _guard88 | _guard94;
wire _guard96 = fsm_out == 1'd0;
wire _guard97 = signal_reg_out;
wire _guard98 = ~_guard97;
wire _guard99 = _guard96 & _guard98;
wire _guard100 = wrapper_early_reset_static_par_go_out;
wire _guard101 = _guard99 & _guard100;
wire _guard102 = fsm_out == 1'd0;
wire _guard103 = signal_reg_out;
wire _guard104 = _guard102 & _guard103;
wire _guard105 = w_handshake_occurred_out;
wire _guard106 = ~_guard105;
wire _guard107 = service_write_transfer_go_out;
wire _guard108 = _guard106 & _guard107;
wire _guard109 = early_reset_static_par_go_out;
wire _guard110 = _guard108 | _guard109;
wire _guard111 = wvalid_out;
wire _guard112 = WREADY;
wire _guard113 = _guard111 & _guard112;
wire _guard114 = service_write_transfer_go_out;
wire _guard115 = _guard113 & _guard114;
wire _guard116 = wvalid_out;
wire _guard117 = WREADY;
wire _guard118 = _guard116 & _guard117;
wire _guard119 = ~_guard118;
wire _guard120 = service_write_transfer_go_out;
wire _guard121 = _guard119 & _guard120;
wire _guard122 = early_reset_static_par_go_out;
wire _guard123 = _guard121 | _guard122;
wire _guard124 = fsm0_out == 2'd2;
wire _guard125 = wrapper_early_reset_static_par_go_out;
assign done = _guard1;
assign WVALID = wvalid_out;
assign WDATA =
  _guard2 ? write_data :
  32'd0;
assign WLAST = _guard3;
assign fsm_write_en = _guard4;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard8 ? adder_out :
  _guard11 ? 1'd0 :
  1'd0;
assign adder_left =
  _guard12 ? fsm_out :
  1'd0;
assign adder_right = _guard13;
assign wvalid_write_en = _guard14;
assign wvalid_clk = clk;
assign wvalid_reset = reset;
assign wvalid_in =
  _guard23 ? 1'd1 :
  _guard30 ? 1'd0 :
  'x;
assign wrapper_early_reset_static_par_go_in = _guard36;
assign wrapper_early_reset_static_par_done_in = _guard39;
assign tdcc_go_in = go;
assign service_write_transfer_done_in = bt_reg_out;
assign fsm0_write_en = _guard52;
assign fsm0_clk = clk;
assign fsm0_reset = reset;
assign fsm0_in =
  _guard57 ? 2'd1 :
  _guard58 ? 2'd0 :
  _guard63 ? 2'd2 :
  2'd0;
assign service_write_transfer_go_in = _guard69;
assign early_reset_static_par_done_in = ud_out;
assign bt_reg_write_en = _guard72;
assign bt_reg_clk = clk;
assign bt_reg_reset = reset;
assign bt_reg_in =
  _guard77 ? 1'd1 :
  _guard85 ? 1'd0 :
  'x;
assign signal_reg_write_en = _guard95;
assign signal_reg_clk = clk;
assign signal_reg_reset = reset;
assign signal_reg_in =
  _guard101 ? 1'd1 :
  _guard104 ? 1'd0 :
  1'd0;
assign w_handshake_occurred_write_en = _guard110;
assign w_handshake_occurred_clk = clk;
assign w_handshake_occurred_reset = reset;
assign w_handshake_occurred_in =
  _guard115 ? 1'd1 :
  _guard123 ? 1'd0 :
  'x;
assign tdcc_done_in = _guard124;
assign early_reset_static_par_go_in = _guard125;
// COMPONENT END: m_write_channel_A0
endmodule
module m_bresp_channel_A0(
  input logic ARESETn,
  input logic BVALID,
  output logic BREADY,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: m_bresp_channel_A0
logic bready_in;
logic bready_write_en;
logic bready_clk;
logic bready_reset;
logic bready_out;
logic bready_done;
logic bt_reg_in;
logic bt_reg_write_en;
logic bt_reg_clk;
logic bt_reg_reset;
logic bt_reg_out;
logic bt_reg_done;
logic [1:0] fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic [1:0] fsm_out;
logic fsm_done;
logic block_transfer_go_in;
logic block_transfer_go_out;
logic block_transfer_done_in;
logic block_transfer_done_out;
logic invoke0_go_in;
logic invoke0_go_out;
logic invoke0_done_in;
logic invoke0_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
std_reg # (
    .WIDTH(1)
) bready (
    .clk(bready_clk),
    .done(bready_done),
    .in(bready_in),
    .out(bready_out),
    .reset(bready_reset),
    .write_en(bready_write_en)
);
std_reg # (
    .WIDTH(1)
) bt_reg (
    .clk(bt_reg_clk),
    .done(bt_reg_done),
    .in(bt_reg_in),
    .out(bt_reg_out),
    .reset(bt_reg_reset),
    .write_en(bt_reg_write_en)
);
std_reg # (
    .WIDTH(2)
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
) block_transfer_go (
    .in(block_transfer_go_in),
    .out(block_transfer_go_out)
);
std_wire # (
    .WIDTH(1)
) block_transfer_done (
    .in(block_transfer_done_in),
    .out(block_transfer_done_out)
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
wire _guard1 = tdcc_done_out;
wire _guard2 = fsm_out == 2'd2;
wire _guard3 = fsm_out == 2'd0;
wire _guard4 = invoke0_done_out;
wire _guard5 = _guard3 & _guard4;
wire _guard6 = tdcc_go_out;
wire _guard7 = _guard5 & _guard6;
wire _guard8 = _guard2 | _guard7;
wire _guard9 = fsm_out == 2'd1;
wire _guard10 = block_transfer_done_out;
wire _guard11 = _guard9 & _guard10;
wire _guard12 = tdcc_go_out;
wire _guard13 = _guard11 & _guard12;
wire _guard14 = _guard8 | _guard13;
wire _guard15 = fsm_out == 2'd0;
wire _guard16 = invoke0_done_out;
wire _guard17 = _guard15 & _guard16;
wire _guard18 = tdcc_go_out;
wire _guard19 = _guard17 & _guard18;
wire _guard20 = fsm_out == 2'd2;
wire _guard21 = fsm_out == 2'd1;
wire _guard22 = block_transfer_done_out;
wire _guard23 = _guard21 & _guard22;
wire _guard24 = tdcc_go_out;
wire _guard25 = _guard23 & _guard24;
wire _guard26 = block_transfer_go_out;
wire _guard27 = bready_out;
wire _guard28 = BVALID;
wire _guard29 = _guard27 & _guard28;
wire _guard30 = ~_guard29;
wire _guard31 = block_transfer_go_out;
wire _guard32 = _guard30 & _guard31;
wire _guard33 = bready_out;
wire _guard34 = BVALID;
wire _guard35 = _guard33 & _guard34;
wire _guard36 = block_transfer_go_out;
wire _guard37 = _guard35 & _guard36;
wire _guard38 = invoke0_done_out;
wire _guard39 = ~_guard38;
wire _guard40 = fsm_out == 2'd0;
wire _guard41 = _guard39 & _guard40;
wire _guard42 = tdcc_go_out;
wire _guard43 = _guard41 & _guard42;
wire _guard44 = block_transfer_go_out;
wire _guard45 = invoke0_go_out;
wire _guard46 = _guard44 | _guard45;
wire _guard47 = bready_out;
wire _guard48 = BVALID;
wire _guard49 = _guard47 & _guard48;
wire _guard50 = block_transfer_go_out;
wire _guard51 = _guard49 & _guard50;
wire _guard52 = bready_out;
wire _guard53 = BVALID;
wire _guard54 = _guard52 & _guard53;
wire _guard55 = ~_guard54;
wire _guard56 = block_transfer_go_out;
wire _guard57 = _guard55 & _guard56;
wire _guard58 = invoke0_go_out;
wire _guard59 = _guard57 | _guard58;
wire _guard60 = fsm_out == 2'd2;
wire _guard61 = block_transfer_done_out;
wire _guard62 = ~_guard61;
wire _guard63 = fsm_out == 2'd1;
wire _guard64 = _guard62 & _guard63;
wire _guard65 = tdcc_go_out;
wire _guard66 = _guard64 & _guard65;
assign done = _guard1;
assign BREADY = bready_out;
assign fsm_write_en = _guard14;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard19 ? 2'd1 :
  _guard20 ? 2'd0 :
  _guard25 ? 2'd2 :
  2'd0;
assign block_transfer_done_in = bt_reg_out;
assign bready_write_en = _guard26;
assign bready_clk = clk;
assign bready_reset = reset;
assign bready_in =
  _guard32 ? 1'd1 :
  _guard37 ? 1'd0 :
  'x;
assign tdcc_go_in = go;
assign invoke0_go_in = _guard43;
assign invoke0_done_in = bt_reg_done;
assign bt_reg_write_en = _guard46;
assign bt_reg_clk = clk;
assign bt_reg_reset = reset;
assign bt_reg_in =
  _guard51 ? 1'd1 :
  _guard59 ? 1'd0 :
  'x;
assign tdcc_done_in = _guard60;
assign block_transfer_go_in = _guard66;
// COMPONENT END: m_bresp_channel_A0
endmodule
module address_translator_A0(
  input logic [2:0] calyx_mem_addr,
  output logic [63:0] axi_address
);
// COMPONENT START: address_translator_A0
logic [63:0] mul_A0_in;
logic [63:0] mul_A0_out;
logic [2:0] pad_input_addr_in;
logic [63:0] pad_input_addr_out;
std_const_mult # (
    .VALUE(4),
    .WIDTH(64)
) mul_A0 (
    .in(mul_A0_in),
    .out(mul_A0_out)
);
std_pad # (
    .IN_WIDTH(3),
    .OUT_WIDTH(64)
) pad_input_addr (
    .in(pad_input_addr_in),
    .out(pad_input_addr_out)
);
wire _guard0 = 1;
assign axi_address = mul_A0_out;
assign mul_A0_in = pad_input_addr_out;
assign pad_input_addr_in = calyx_mem_addr;
// COMPONENT END: address_translator_A0
endmodule
module read_controller_A0(
  input logic [63:0] axi_address,
  input logic ARESETn,
  input logic ARREADY,
  input logic RVALID,
  input logic RLAST,
  input logic [31:0] RDATA,
  input logic [1:0] RRESP,
  output logic ARVALID,
  output logic [63:0] ARADDR,
  output logic [2:0] ARSIZE,
  output logic [7:0] ARLEN,
  output logic [1:0] ARBURST,
  output logic [2:0] ARPROT,
  output logic RREADY,
  output logic [31:0] read_data,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: read_controller_A0
logic ar_channel_A0_ARESETn;
logic ar_channel_A0_ARREADY;
logic [63:0] ar_channel_A0_axi_address;
logic ar_channel_A0_ARVALID;
logic [63:0] ar_channel_A0_ARADDR;
logic [2:0] ar_channel_A0_ARSIZE;
logic [7:0] ar_channel_A0_ARLEN;
logic [1:0] ar_channel_A0_ARBURST;
logic [2:0] ar_channel_A0_ARPROT;
logic ar_channel_A0_go;
logic ar_channel_A0_clk;
logic ar_channel_A0_reset;
logic ar_channel_A0_done;
logic read_channel_A0_ARESETn;
logic read_channel_A0_RVALID;
logic read_channel_A0_RLAST;
logic [31:0] read_channel_A0_RDATA;
logic [1:0] read_channel_A0_RRESP;
logic read_channel_A0_RREADY;
logic [31:0] read_channel_A0_read_data;
logic read_channel_A0_go;
logic read_channel_A0_clk;
logic read_channel_A0_reset;
logic read_channel_A0_done;
logic [1:0] fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic [1:0] fsm_out;
logic fsm_done;
logic invoke0_go_in;
logic invoke0_go_out;
logic invoke0_done_in;
logic invoke0_done_out;
logic invoke1_go_in;
logic invoke1_go_out;
logic invoke1_done_in;
logic invoke1_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
m_ar_channel_A0 ar_channel_A0 (
    .ARADDR(ar_channel_A0_ARADDR),
    .ARBURST(ar_channel_A0_ARBURST),
    .ARESETn(ar_channel_A0_ARESETn),
    .ARLEN(ar_channel_A0_ARLEN),
    .ARPROT(ar_channel_A0_ARPROT),
    .ARREADY(ar_channel_A0_ARREADY),
    .ARSIZE(ar_channel_A0_ARSIZE),
    .ARVALID(ar_channel_A0_ARVALID),
    .axi_address(ar_channel_A0_axi_address),
    .clk(ar_channel_A0_clk),
    .done(ar_channel_A0_done),
    .go(ar_channel_A0_go),
    .reset(ar_channel_A0_reset)
);
m_read_channel_A0 read_channel_A0 (
    .ARESETn(read_channel_A0_ARESETn),
    .RDATA(read_channel_A0_RDATA),
    .RLAST(read_channel_A0_RLAST),
    .RREADY(read_channel_A0_RREADY),
    .RRESP(read_channel_A0_RRESP),
    .RVALID(read_channel_A0_RVALID),
    .clk(read_channel_A0_clk),
    .done(read_channel_A0_done),
    .go(read_channel_A0_go),
    .read_data(read_channel_A0_read_data),
    .reset(read_channel_A0_reset)
);
std_reg # (
    .WIDTH(2)
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
wire _guard1 = tdcc_done_out;
wire _guard2 = invoke0_go_out;
wire _guard3 = invoke0_go_out;
wire _guard4 = invoke1_go_out;
wire _guard5 = invoke0_go_out;
wire _guard6 = invoke0_go_out;
wire _guard7 = invoke0_go_out;
wire _guard8 = invoke0_go_out;
wire _guard9 = fsm_out == 2'd2;
wire _guard10 = fsm_out == 2'd0;
wire _guard11 = invoke0_done_out;
wire _guard12 = _guard10 & _guard11;
wire _guard13 = tdcc_go_out;
wire _guard14 = _guard12 & _guard13;
wire _guard15 = _guard9 | _guard14;
wire _guard16 = fsm_out == 2'd1;
wire _guard17 = invoke1_done_out;
wire _guard18 = _guard16 & _guard17;
wire _guard19 = tdcc_go_out;
wire _guard20 = _guard18 & _guard19;
wire _guard21 = _guard15 | _guard20;
wire _guard22 = fsm_out == 2'd0;
wire _guard23 = invoke0_done_out;
wire _guard24 = _guard22 & _guard23;
wire _guard25 = tdcc_go_out;
wire _guard26 = _guard24 & _guard25;
wire _guard27 = fsm_out == 2'd2;
wire _guard28 = fsm_out == 2'd1;
wire _guard29 = invoke1_done_out;
wire _guard30 = _guard28 & _guard29;
wire _guard31 = tdcc_go_out;
wire _guard32 = _guard30 & _guard31;
wire _guard33 = invoke1_go_out;
wire _guard34 = invoke1_go_out;
wire _guard35 = invoke1_go_out;
wire _guard36 = invoke1_go_out;
wire _guard37 = invoke1_go_out;
wire _guard38 = invoke1_go_out;
wire _guard39 = invoke0_done_out;
wire _guard40 = ~_guard39;
wire _guard41 = fsm_out == 2'd0;
wire _guard42 = _guard40 & _guard41;
wire _guard43 = tdcc_go_out;
wire _guard44 = _guard42 & _guard43;
wire _guard45 = invoke0_go_out;
wire _guard46 = invoke0_go_out;
wire _guard47 = invoke0_go_out;
wire _guard48 = invoke0_go_out;
wire _guard49 = invoke1_done_out;
wire _guard50 = ~_guard49;
wire _guard51 = fsm_out == 2'd1;
wire _guard52 = _guard50 & _guard51;
wire _guard53 = tdcc_go_out;
wire _guard54 = _guard52 & _guard53;
wire _guard55 = fsm_out == 2'd2;
assign done = _guard1;
assign ARPROT =
  _guard2 ? ar_channel_A0_ARPROT :
  3'd0;
assign ARSIZE =
  _guard3 ? ar_channel_A0_ARSIZE :
  3'd0;
assign RREADY =
  _guard4 ? read_channel_A0_RREADY :
  1'd0;
assign read_data = read_channel_A0_read_data;
assign ARLEN =
  _guard5 ? ar_channel_A0_ARLEN :
  8'd0;
assign ARADDR =
  _guard6 ? ar_channel_A0_ARADDR :
  64'd0;
assign ARBURST =
  _guard7 ? ar_channel_A0_ARBURST :
  2'd0;
assign ARVALID =
  _guard8 ? ar_channel_A0_ARVALID :
  1'd0;
assign fsm_write_en = _guard21;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard26 ? 2'd1 :
  _guard27 ? 2'd0 :
  _guard32 ? 2'd2 :
  2'd0;
assign read_channel_A0_RVALID =
  _guard33 ? RVALID :
  1'd0;
assign read_channel_A0_RLAST =
  _guard34 ? RLAST :
  1'd0;
assign read_channel_A0_RDATA =
  _guard35 ? RDATA :
  32'd0;
assign read_channel_A0_clk = clk;
assign read_channel_A0_go = _guard36;
assign read_channel_A0_reset = reset;
assign read_channel_A0_RRESP =
  _guard37 ? RRESP :
  2'd0;
assign read_channel_A0_ARESETn =
  _guard38 ? ARESETn :
  1'd0;
assign tdcc_go_in = go;
assign invoke0_go_in = _guard44;
assign ar_channel_A0_clk = clk;
assign ar_channel_A0_axi_address =
  _guard45 ? axi_address :
  64'd0;
assign ar_channel_A0_go = _guard46;
assign ar_channel_A0_reset = reset;
assign ar_channel_A0_ARREADY =
  _guard47 ? ARREADY :
  1'd0;
assign ar_channel_A0_ARESETn =
  _guard48 ? ARESETn :
  1'd0;
assign invoke0_done_in = ar_channel_A0_done;
assign invoke1_go_in = _guard54;
assign tdcc_done_in = _guard55;
assign invoke1_done_in = read_channel_A0_done;
// COMPONENT END: read_controller_A0
endmodule
module write_controller_A0(
  input logic [63:0] axi_address,
  input logic [31:0] write_data,
  input logic ARESETn,
  input logic AWREADY,
  input logic WREADY,
  input logic BVALID,
  output logic AWVALID,
  output logic [63:0] AWADDR,
  output logic [2:0] AWSIZE,
  output logic [7:0] AWLEN,
  output logic [1:0] AWBURST,
  output logic [2:0] AWPROT,
  output logic WVALID,
  output logic WLAST,
  output logic [31:0] WDATA,
  output logic BREADY,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: write_controller_A0
logic aw_channel_A0_ARESETn;
logic aw_channel_A0_AWREADY;
logic [63:0] aw_channel_A0_axi_address;
logic aw_channel_A0_AWVALID;
logic [63:0] aw_channel_A0_AWADDR;
logic [2:0] aw_channel_A0_AWSIZE;
logic [7:0] aw_channel_A0_AWLEN;
logic [1:0] aw_channel_A0_AWBURST;
logic [2:0] aw_channel_A0_AWPROT;
logic aw_channel_A0_go;
logic aw_channel_A0_clk;
logic aw_channel_A0_reset;
logic aw_channel_A0_done;
logic write_channel_A0_ARESETn;
logic write_channel_A0_WREADY;
logic [31:0] write_channel_A0_write_data;
logic write_channel_A0_WVALID;
logic write_channel_A0_WLAST;
logic [31:0] write_channel_A0_WDATA;
logic write_channel_A0_go;
logic write_channel_A0_clk;
logic write_channel_A0_reset;
logic write_channel_A0_done;
logic bresp_channel_A0_ARESETn;
logic bresp_channel_A0_BVALID;
logic bresp_channel_A0_BREADY;
logic bresp_channel_A0_go;
logic bresp_channel_A0_clk;
logic bresp_channel_A0_reset;
logic bresp_channel_A0_done;
logic [1:0] fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic [1:0] fsm_out;
logic fsm_done;
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
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
m_aw_channel_A0 aw_channel_A0 (
    .ARESETn(aw_channel_A0_ARESETn),
    .AWADDR(aw_channel_A0_AWADDR),
    .AWBURST(aw_channel_A0_AWBURST),
    .AWLEN(aw_channel_A0_AWLEN),
    .AWPROT(aw_channel_A0_AWPROT),
    .AWREADY(aw_channel_A0_AWREADY),
    .AWSIZE(aw_channel_A0_AWSIZE),
    .AWVALID(aw_channel_A0_AWVALID),
    .axi_address(aw_channel_A0_axi_address),
    .clk(aw_channel_A0_clk),
    .done(aw_channel_A0_done),
    .go(aw_channel_A0_go),
    .reset(aw_channel_A0_reset)
);
m_write_channel_A0 write_channel_A0 (
    .ARESETn(write_channel_A0_ARESETn),
    .WDATA(write_channel_A0_WDATA),
    .WLAST(write_channel_A0_WLAST),
    .WREADY(write_channel_A0_WREADY),
    .WVALID(write_channel_A0_WVALID),
    .clk(write_channel_A0_clk),
    .done(write_channel_A0_done),
    .go(write_channel_A0_go),
    .reset(write_channel_A0_reset),
    .write_data(write_channel_A0_write_data)
);
m_bresp_channel_A0 bresp_channel_A0 (
    .ARESETn(bresp_channel_A0_ARESETn),
    .BREADY(bresp_channel_A0_BREADY),
    .BVALID(bresp_channel_A0_BVALID),
    .clk(bresp_channel_A0_clk),
    .done(bresp_channel_A0_done),
    .go(bresp_channel_A0_go),
    .reset(bresp_channel_A0_reset)
);
std_reg # (
    .WIDTH(2)
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
wire _guard1 = tdcc_done_out;
wire _guard2 = invoke0_go_out;
wire _guard3 = invoke0_go_out;
wire _guard4 = invoke1_go_out;
wire _guard5 = invoke1_go_out;
wire _guard6 = invoke0_go_out;
wire _guard7 = invoke0_go_out;
wire _guard8 = invoke0_go_out;
wire _guard9 = invoke1_go_out;
wire _guard10 = invoke2_go_out;
wire _guard11 = invoke0_go_out;
wire _guard12 = fsm_out == 2'd3;
wire _guard13 = fsm_out == 2'd0;
wire _guard14 = invoke0_done_out;
wire _guard15 = _guard13 & _guard14;
wire _guard16 = tdcc_go_out;
wire _guard17 = _guard15 & _guard16;
wire _guard18 = _guard12 | _guard17;
wire _guard19 = fsm_out == 2'd1;
wire _guard20 = invoke1_done_out;
wire _guard21 = _guard19 & _guard20;
wire _guard22 = tdcc_go_out;
wire _guard23 = _guard21 & _guard22;
wire _guard24 = _guard18 | _guard23;
wire _guard25 = fsm_out == 2'd2;
wire _guard26 = invoke2_done_out;
wire _guard27 = _guard25 & _guard26;
wire _guard28 = tdcc_go_out;
wire _guard29 = _guard27 & _guard28;
wire _guard30 = _guard24 | _guard29;
wire _guard31 = fsm_out == 2'd0;
wire _guard32 = invoke0_done_out;
wire _guard33 = _guard31 & _guard32;
wire _guard34 = tdcc_go_out;
wire _guard35 = _guard33 & _guard34;
wire _guard36 = fsm_out == 2'd3;
wire _guard37 = fsm_out == 2'd2;
wire _guard38 = invoke2_done_out;
wire _guard39 = _guard37 & _guard38;
wire _guard40 = tdcc_go_out;
wire _guard41 = _guard39 & _guard40;
wire _guard42 = fsm_out == 2'd1;
wire _guard43 = invoke1_done_out;
wire _guard44 = _guard42 & _guard43;
wire _guard45 = tdcc_go_out;
wire _guard46 = _guard44 & _guard45;
wire _guard47 = invoke2_done_out;
wire _guard48 = ~_guard47;
wire _guard49 = fsm_out == 2'd2;
wire _guard50 = _guard48 & _guard49;
wire _guard51 = tdcc_go_out;
wire _guard52 = _guard50 & _guard51;
wire _guard53 = invoke0_done_out;
wire _guard54 = ~_guard53;
wire _guard55 = fsm_out == 2'd0;
wire _guard56 = _guard54 & _guard55;
wire _guard57 = tdcc_go_out;
wire _guard58 = _guard56 & _guard57;
wire _guard59 = invoke0_go_out;
wire _guard60 = invoke0_go_out;
wire _guard61 = invoke0_go_out;
wire _guard62 = invoke0_go_out;
wire _guard63 = invoke1_done_out;
wire _guard64 = ~_guard63;
wire _guard65 = fsm_out == 2'd1;
wire _guard66 = _guard64 & _guard65;
wire _guard67 = tdcc_go_out;
wire _guard68 = _guard66 & _guard67;
wire _guard69 = invoke1_go_out;
wire _guard70 = invoke1_go_out;
wire _guard71 = invoke1_go_out;
wire _guard72 = invoke1_go_out;
wire _guard73 = invoke2_go_out;
wire _guard74 = invoke2_go_out;
wire _guard75 = fsm_out == 2'd3;
assign done = _guard1;
assign AWADDR =
  _guard2 ? aw_channel_A0_AWADDR :
  64'd0;
assign AWPROT =
  _guard3 ? aw_channel_A0_AWPROT :
  3'd0;
assign WVALID =
  _guard4 ? write_channel_A0_WVALID :
  1'd0;
assign WDATA =
  _guard5 ? write_channel_A0_WDATA :
  32'd0;
assign AWSIZE =
  _guard6 ? aw_channel_A0_AWSIZE :
  3'd0;
assign AWVALID =
  _guard7 ? aw_channel_A0_AWVALID :
  1'd0;
assign AWBURST =
  _guard8 ? aw_channel_A0_AWBURST :
  2'd0;
assign WLAST =
  _guard9 ? write_channel_A0_WLAST :
  1'd0;
assign BREADY =
  _guard10 ? bresp_channel_A0_BREADY :
  1'd0;
assign AWLEN =
  _guard11 ? aw_channel_A0_AWLEN :
  8'd0;
assign fsm_write_en = _guard30;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard35 ? 2'd1 :
  _guard36 ? 2'd0 :
  _guard41 ? 2'd3 :
  _guard46 ? 2'd2 :
  2'd0;
assign invoke2_go_in = _guard52;
assign tdcc_go_in = go;
assign invoke0_go_in = _guard58;
assign aw_channel_A0_clk = clk;
assign aw_channel_A0_axi_address =
  _guard59 ? axi_address :
  64'd0;
assign aw_channel_A0_AWREADY =
  _guard60 ? AWREADY :
  1'd0;
assign aw_channel_A0_go = _guard61;
assign aw_channel_A0_reset = reset;
assign aw_channel_A0_ARESETn =
  _guard62 ? ARESETn :
  1'd0;
assign invoke0_done_in = aw_channel_A0_done;
assign invoke1_go_in = _guard68;
assign write_channel_A0_WREADY =
  _guard69 ? WREADY :
  1'd0;
assign write_channel_A0_clk = clk;
assign write_channel_A0_go = _guard70;
assign write_channel_A0_reset = reset;
assign write_channel_A0_write_data =
  _guard71 ? write_data :
  32'd0;
assign write_channel_A0_ARESETn =
  _guard72 ? ARESETn :
  1'd0;
assign invoke2_done_in = bresp_channel_A0_done;
assign bresp_channel_A0_clk = clk;
assign bresp_channel_A0_go = _guard73;
assign bresp_channel_A0_reset = reset;
assign bresp_channel_A0_BVALID =
  _guard74 ? BVALID :
  1'd0;
assign tdcc_done_in = _guard75;
assign invoke1_done_in = write_channel_A0_done;
// COMPONENT END: write_controller_A0
endmodule
module axi_dyn_mem_A0(
  input logic [2:0] addr0,
  input logic content_en,
  input logic write_en,
  input logic [31:0] write_data,
  input logic ARESETn,
  input logic ARREADY,
  input logic RVALID,
  input logic RLAST,
  input logic [31:0] RDATA,
  input logic [1:0] RRESP,
  input logic AWREADY,
  input logic WREADY,
  input logic BVALID,
  input logic [1:0] BRESP,
  output logic [31:0] read_data,
  output logic ARVALID,
  output logic [63:0] ARADDR,
  output logic [2:0] ARSIZE,
  output logic [7:0] ARLEN,
  output logic [1:0] ARBURST,
  output logic [2:0] ARPROT,
  output logic RREADY,
  output logic AWVALID,
  output logic [63:0] AWADDR,
  output logic [2:0] AWSIZE,
  output logic [7:0] AWLEN,
  output logic [1:0] AWBURST,
  output logic [2:0] AWPROT,
  output logic WVALID,
  output logic WLAST,
  output logic [31:0] WDATA,
  output logic BREADY,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: axi_dyn_mem_A0
logic [2:0] address_translator_A0_calyx_mem_addr;
logic [63:0] address_translator_A0_axi_address;
logic [63:0] read_controller_A0_axi_address;
logic read_controller_A0_ARESETn;
logic read_controller_A0_ARREADY;
logic read_controller_A0_RVALID;
logic read_controller_A0_RLAST;
logic [31:0] read_controller_A0_RDATA;
logic [1:0] read_controller_A0_RRESP;
logic read_controller_A0_ARVALID;
logic [63:0] read_controller_A0_ARADDR;
logic [2:0] read_controller_A0_ARSIZE;
logic [7:0] read_controller_A0_ARLEN;
logic [1:0] read_controller_A0_ARBURST;
logic [2:0] read_controller_A0_ARPROT;
logic read_controller_A0_RREADY;
logic [31:0] read_controller_A0_read_data;
logic read_controller_A0_go;
logic read_controller_A0_clk;
logic read_controller_A0_reset;
logic read_controller_A0_done;
logic [63:0] write_controller_A0_axi_address;
logic [31:0] write_controller_A0_write_data;
logic write_controller_A0_ARESETn;
logic write_controller_A0_AWREADY;
logic write_controller_A0_WREADY;
logic write_controller_A0_BVALID;
logic write_controller_A0_AWVALID;
logic [63:0] write_controller_A0_AWADDR;
logic [2:0] write_controller_A0_AWSIZE;
logic [7:0] write_controller_A0_AWLEN;
logic [1:0] write_controller_A0_AWBURST;
logic [2:0] write_controller_A0_AWPROT;
logic write_controller_A0_WVALID;
logic write_controller_A0_WLAST;
logic [31:0] write_controller_A0_WDATA;
logic write_controller_A0_BREADY;
logic write_controller_A0_go;
logic write_controller_A0_clk;
logic write_controller_A0_reset;
logic write_controller_A0_done;
logic [1:0] fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic [1:0] fsm_out;
logic fsm_done;
logic invoke0_go_in;
logic invoke0_go_out;
logic invoke0_done_in;
logic invoke0_done_out;
logic invoke1_go_in;
logic invoke1_go_out;
logic invoke1_done_in;
logic invoke1_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
address_translator_A0 address_translator_A0 (
    .axi_address(address_translator_A0_axi_address),
    .calyx_mem_addr(address_translator_A0_calyx_mem_addr)
);
read_controller_A0 read_controller_A0 (
    .ARADDR(read_controller_A0_ARADDR),
    .ARBURST(read_controller_A0_ARBURST),
    .ARESETn(read_controller_A0_ARESETn),
    .ARLEN(read_controller_A0_ARLEN),
    .ARPROT(read_controller_A0_ARPROT),
    .ARREADY(read_controller_A0_ARREADY),
    .ARSIZE(read_controller_A0_ARSIZE),
    .ARVALID(read_controller_A0_ARVALID),
    .RDATA(read_controller_A0_RDATA),
    .RLAST(read_controller_A0_RLAST),
    .RREADY(read_controller_A0_RREADY),
    .RRESP(read_controller_A0_RRESP),
    .RVALID(read_controller_A0_RVALID),
    .axi_address(read_controller_A0_axi_address),
    .clk(read_controller_A0_clk),
    .done(read_controller_A0_done),
    .go(read_controller_A0_go),
    .read_data(read_controller_A0_read_data),
    .reset(read_controller_A0_reset)
);
write_controller_A0 write_controller_A0 (
    .ARESETn(write_controller_A0_ARESETn),
    .AWADDR(write_controller_A0_AWADDR),
    .AWBURST(write_controller_A0_AWBURST),
    .AWLEN(write_controller_A0_AWLEN),
    .AWPROT(write_controller_A0_AWPROT),
    .AWREADY(write_controller_A0_AWREADY),
    .AWSIZE(write_controller_A0_AWSIZE),
    .AWVALID(write_controller_A0_AWVALID),
    .BREADY(write_controller_A0_BREADY),
    .BVALID(write_controller_A0_BVALID),
    .WDATA(write_controller_A0_WDATA),
    .WLAST(write_controller_A0_WLAST),
    .WREADY(write_controller_A0_WREADY),
    .WVALID(write_controller_A0_WVALID),
    .axi_address(write_controller_A0_axi_address),
    .clk(write_controller_A0_clk),
    .done(write_controller_A0_done),
    .go(write_controller_A0_go),
    .reset(write_controller_A0_reset),
    .write_data(write_controller_A0_write_data)
);
std_reg # (
    .WIDTH(2)
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
wire _guard1 = tdcc_done_out;
wire _guard2 = invoke1_go_out;
wire _guard3 = invoke0_go_out;
wire _guard4 = invoke0_go_out;
wire _guard5 = invoke0_go_out;
wire _guard6 = invoke0_go_out;
wire _guard7 = invoke1_go_out;
wire _guard8 = invoke1_go_out;
wire _guard9 = invoke1_go_out;
wire _guard10 = invoke0_go_out;
wire _guard11 = invoke0_go_out;
wire _guard12 = invoke1_go_out;
wire _guard13 = invoke0_go_out;
wire _guard14 = invoke0_go_out;
wire _guard15 = invoke0_go_out;
wire _guard16 = invoke0_go_out;
wire _guard17 = invoke1_go_out;
wire _guard18 = invoke1_go_out;
wire _guard19 = fsm_out == 2'd3;
wire _guard20 = fsm_out == 2'd0;
wire _guard21 = write_en;
wire _guard22 = _guard20 & _guard21;
wire _guard23 = tdcc_go_out;
wire _guard24 = _guard22 & _guard23;
wire _guard25 = _guard19 | _guard24;
wire _guard26 = fsm_out == 2'd0;
wire _guard27 = write_en;
wire _guard28 = ~_guard27;
wire _guard29 = _guard26 & _guard28;
wire _guard30 = tdcc_go_out;
wire _guard31 = _guard29 & _guard30;
wire _guard32 = _guard25 | _guard31;
wire _guard33 = fsm_out == 2'd1;
wire _guard34 = invoke0_done_out;
wire _guard35 = _guard33 & _guard34;
wire _guard36 = tdcc_go_out;
wire _guard37 = _guard35 & _guard36;
wire _guard38 = _guard32 | _guard37;
wire _guard39 = fsm_out == 2'd2;
wire _guard40 = invoke1_done_out;
wire _guard41 = _guard39 & _guard40;
wire _guard42 = tdcc_go_out;
wire _guard43 = _guard41 & _guard42;
wire _guard44 = _guard38 | _guard43;
wire _guard45 = fsm_out == 2'd0;
wire _guard46 = write_en;
wire _guard47 = _guard45 & _guard46;
wire _guard48 = tdcc_go_out;
wire _guard49 = _guard47 & _guard48;
wire _guard50 = fsm_out == 2'd3;
wire _guard51 = fsm_out == 2'd1;
wire _guard52 = invoke0_done_out;
wire _guard53 = _guard51 & _guard52;
wire _guard54 = tdcc_go_out;
wire _guard55 = _guard53 & _guard54;
wire _guard56 = fsm_out == 2'd2;
wire _guard57 = invoke1_done_out;
wire _guard58 = _guard56 & _guard57;
wire _guard59 = tdcc_go_out;
wire _guard60 = _guard58 & _guard59;
wire _guard61 = _guard55 | _guard60;
wire _guard62 = fsm_out == 2'd0;
wire _guard63 = write_en;
wire _guard64 = ~_guard63;
wire _guard65 = _guard62 & _guard64;
wire _guard66 = tdcc_go_out;
wire _guard67 = _guard65 & _guard66;
wire _guard68 = invoke1_go_out;
wire _guard69 = invoke1_go_out;
wire _guard70 = invoke1_go_out;
wire _guard71 = invoke1_go_out;
wire _guard72 = invoke1_go_out;
wire _guard73 = invoke1_go_out;
wire _guard74 = invoke1_go_out;
wire _guard75 = invoke1_go_out;
wire _guard76 = invoke0_done_out;
wire _guard77 = ~_guard76;
wire _guard78 = fsm_out == 2'd1;
wire _guard79 = _guard77 & _guard78;
wire _guard80 = tdcc_go_out;
wire _guard81 = _guard79 & _guard80;
wire _guard82 = invoke1_done_out;
wire _guard83 = ~_guard82;
wire _guard84 = fsm_out == 2'd2;
wire _guard85 = _guard83 & _guard84;
wire _guard86 = tdcc_go_out;
wire _guard87 = _guard85 & _guard86;
wire _guard88 = invoke0_go_out;
wire _guard89 = invoke0_go_out;
wire _guard90 = invoke0_go_out;
wire _guard91 = invoke0_go_out;
wire _guard92 = invoke0_go_out;
wire _guard93 = invoke0_go_out;
wire _guard94 = invoke0_go_out;
wire _guard95 = fsm_out == 2'd3;
assign done = _guard1;
assign ARPROT =
  _guard2 ? read_controller_A0_ARPROT :
  3'd0;
assign AWADDR =
  _guard3 ? write_controller_A0_AWADDR :
  64'd0;
assign AWPROT =
  _guard4 ? write_controller_A0_AWPROT :
  3'd0;
assign WVALID =
  _guard5 ? write_controller_A0_WVALID :
  1'd0;
assign WDATA =
  _guard6 ? write_controller_A0_WDATA :
  32'd0;
assign ARSIZE =
  _guard7 ? read_controller_A0_ARSIZE :
  3'd0;
assign RREADY =
  _guard8 ? read_controller_A0_RREADY :
  1'd0;
assign read_data = read_controller_A0_read_data;
assign ARLEN =
  _guard9 ? read_controller_A0_ARLEN :
  8'd0;
assign AWSIZE =
  _guard10 ? write_controller_A0_AWSIZE :
  3'd0;
assign AWVALID =
  _guard11 ? write_controller_A0_AWVALID :
  1'd0;
assign ARADDR =
  _guard12 ? read_controller_A0_ARADDR :
  64'd0;
assign AWBURST =
  _guard13 ? write_controller_A0_AWBURST :
  2'd0;
assign WLAST =
  _guard14 ? write_controller_A0_WLAST :
  1'd0;
assign BREADY =
  _guard15 ? write_controller_A0_BREADY :
  1'd0;
assign AWLEN =
  _guard16 ? write_controller_A0_AWLEN :
  8'd0;
assign ARBURST =
  _guard17 ? read_controller_A0_ARBURST :
  2'd0;
assign ARVALID =
  _guard18 ? read_controller_A0_ARVALID :
  1'd0;
assign fsm_write_en = _guard44;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard49 ? 2'd1 :
  _guard50 ? 2'd0 :
  _guard61 ? 2'd3 :
  _guard67 ? 2'd2 :
  2'd0;
assign read_controller_A0_RVALID =
  _guard68 ? RVALID :
  1'd0;
assign read_controller_A0_RLAST =
  _guard69 ? RLAST :
  1'd0;
assign read_controller_A0_RDATA =
  _guard70 ? RDATA :
  32'd0;
assign read_controller_A0_clk = clk;
assign read_controller_A0_axi_address =
  _guard71 ? address_translator_A0_axi_address :
  64'd0;
assign read_controller_A0_go = _guard72;
assign read_controller_A0_reset = reset;
assign read_controller_A0_RRESP =
  _guard73 ? RRESP :
  2'd0;
assign read_controller_A0_ARREADY =
  _guard74 ? ARREADY :
  1'd0;
assign read_controller_A0_ARESETn =
  _guard75 ? ARESETn :
  1'd0;
assign tdcc_go_in = content_en;
assign invoke0_go_in = _guard81;
assign invoke0_done_in = write_controller_A0_done;
assign invoke1_go_in = _guard87;
assign write_controller_A0_WREADY =
  _guard88 ? WREADY :
  1'd0;
assign write_controller_A0_clk = clk;
assign write_controller_A0_axi_address =
  _guard89 ? address_translator_A0_axi_address :
  64'd0;
assign write_controller_A0_AWREADY =
  _guard90 ? AWREADY :
  1'd0;
assign write_controller_A0_go = _guard91;
assign write_controller_A0_reset = reset;
assign write_controller_A0_write_data =
  _guard92 ? write_data :
  32'd0;
assign write_controller_A0_BVALID =
  _guard93 ? BVALID :
  1'd0;
assign write_controller_A0_ARESETn =
  _guard94 ? ARESETn :
  1'd0;
assign address_translator_A0_calyx_mem_addr = addr0;
assign tdcc_done_in = _guard95;
assign invoke1_done_in = read_controller_A0_done;
// COMPONENT END: axi_dyn_mem_A0
endmodule
module m_ar_channel_B0(
  input logic ARESETn,
  input logic ARREADY,
  input logic [63:0] axi_address,
  output logic ARVALID,
  output logic [63:0] ARADDR,
  output logic [2:0] ARSIZE,
  output logic [7:0] ARLEN,
  output logic [1:0] ARBURST,
  output logic [2:0] ARPROT,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: m_ar_channel_B0
logic arvalid_in;
logic arvalid_write_en;
logic arvalid_clk;
logic arvalid_reset;
logic arvalid_out;
logic arvalid_done;
logic ar_handshake_occurred_in;
logic ar_handshake_occurred_write_en;
logic ar_handshake_occurred_clk;
logic ar_handshake_occurred_reset;
logic ar_handshake_occurred_out;
logic ar_handshake_occurred_done;
logic bt_reg_in;
logic bt_reg_write_en;
logic bt_reg_clk;
logic bt_reg_reset;
logic bt_reg_out;
logic bt_reg_done;
logic fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic fsm_out;
logic fsm_done;
logic adder_left;
logic adder_right;
logic adder_out;
logic ud_out;
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
logic do_ar_transfer_go_in;
logic do_ar_transfer_go_out;
logic do_ar_transfer_done_in;
logic do_ar_transfer_done_out;
logic invoke2_go_in;
logic invoke2_go_out;
logic invoke2_done_in;
logic invoke2_done_out;
logic early_reset_static_par_go_in;
logic early_reset_static_par_go_out;
logic early_reset_static_par_done_in;
logic early_reset_static_par_done_out;
logic wrapper_early_reset_static_par_go_in;
logic wrapper_early_reset_static_par_go_out;
logic wrapper_early_reset_static_par_done_in;
logic wrapper_early_reset_static_par_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
std_reg # (
    .WIDTH(1)
) arvalid (
    .clk(arvalid_clk),
    .done(arvalid_done),
    .in(arvalid_in),
    .out(arvalid_out),
    .reset(arvalid_reset),
    .write_en(arvalid_write_en)
);
std_reg # (
    .WIDTH(1)
) ar_handshake_occurred (
    .clk(ar_handshake_occurred_clk),
    .done(ar_handshake_occurred_done),
    .in(ar_handshake_occurred_in),
    .out(ar_handshake_occurred_out),
    .reset(ar_handshake_occurred_reset),
    .write_en(ar_handshake_occurred_write_en)
);
std_reg # (
    .WIDTH(1)
) bt_reg (
    .clk(bt_reg_clk),
    .done(bt_reg_done),
    .in(bt_reg_in),
    .out(bt_reg_out),
    .reset(bt_reg_reset),
    .write_en(bt_reg_write_en)
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
std_add # (
    .WIDTH(1)
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
) do_ar_transfer_go (
    .in(do_ar_transfer_go_in),
    .out(do_ar_transfer_go_out)
);
std_wire # (
    .WIDTH(1)
) do_ar_transfer_done (
    .in(do_ar_transfer_done_in),
    .out(do_ar_transfer_done_out)
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
wire _guard1 = do_ar_transfer_done_out;
wire _guard2 = ~_guard1;
wire _guard3 = fsm0_out == 2'd1;
wire _guard4 = _guard2 & _guard3;
wire _guard5 = tdcc_go_out;
wire _guard6 = _guard4 & _guard5;
wire _guard7 = tdcc_done_out;
wire _guard8 = do_ar_transfer_go_out;
wire _guard9 = do_ar_transfer_go_out;
wire _guard10 = do_ar_transfer_go_out;
wire _guard11 = do_ar_transfer_go_out;
wire _guard12 = do_ar_transfer_go_out;
wire _guard13 = early_reset_static_par_go_out;
wire _guard14 = fsm_out == 1'd0;
wire _guard15 = ~_guard14;
wire _guard16 = early_reset_static_par_go_out;
wire _guard17 = _guard15 & _guard16;
wire _guard18 = fsm_out == 1'd0;
wire _guard19 = early_reset_static_par_go_out;
wire _guard20 = _guard18 & _guard19;
wire _guard21 = early_reset_static_par_go_out;
wire _guard22 = early_reset_static_par_go_out;
wire _guard23 = ar_handshake_occurred_out;
wire _guard24 = ~_guard23;
wire _guard25 = do_ar_transfer_go_out;
wire _guard26 = _guard24 & _guard25;
wire _guard27 = early_reset_static_par_go_out;
wire _guard28 = _guard26 | _guard27;
wire _guard29 = arvalid_out;
wire _guard30 = ARREADY;
wire _guard31 = _guard29 & _guard30;
wire _guard32 = do_ar_transfer_go_out;
wire _guard33 = _guard31 & _guard32;
wire _guard34 = early_reset_static_par_go_out;
wire _guard35 = invoke2_done_out;
wire _guard36 = ~_guard35;
wire _guard37 = fsm0_out == 2'd2;
wire _guard38 = _guard36 & _guard37;
wire _guard39 = tdcc_go_out;
wire _guard40 = _guard38 & _guard39;
wire _guard41 = wrapper_early_reset_static_par_done_out;
wire _guard42 = ~_guard41;
wire _guard43 = fsm0_out == 2'd0;
wire _guard44 = _guard42 & _guard43;
wire _guard45 = tdcc_go_out;
wire _guard46 = _guard44 & _guard45;
wire _guard47 = fsm_out == 1'd0;
wire _guard48 = signal_reg_out;
wire _guard49 = _guard47 & _guard48;
wire _guard50 = fsm0_out == 2'd3;
wire _guard51 = fsm0_out == 2'd0;
wire _guard52 = wrapper_early_reset_static_par_done_out;
wire _guard53 = _guard51 & _guard52;
wire _guard54 = tdcc_go_out;
wire _guard55 = _guard53 & _guard54;
wire _guard56 = _guard50 | _guard55;
wire _guard57 = fsm0_out == 2'd1;
wire _guard58 = do_ar_transfer_done_out;
wire _guard59 = _guard57 & _guard58;
wire _guard60 = tdcc_go_out;
wire _guard61 = _guard59 & _guard60;
wire _guard62 = _guard56 | _guard61;
wire _guard63 = fsm0_out == 2'd2;
wire _guard64 = invoke2_done_out;
wire _guard65 = _guard63 & _guard64;
wire _guard66 = tdcc_go_out;
wire _guard67 = _guard65 & _guard66;
wire _guard68 = _guard62 | _guard67;
wire _guard69 = fsm0_out == 2'd0;
wire _guard70 = wrapper_early_reset_static_par_done_out;
wire _guard71 = _guard69 & _guard70;
wire _guard72 = tdcc_go_out;
wire _guard73 = _guard71 & _guard72;
wire _guard74 = fsm0_out == 2'd3;
wire _guard75 = fsm0_out == 2'd2;
wire _guard76 = invoke2_done_out;
wire _guard77 = _guard75 & _guard76;
wire _guard78 = tdcc_go_out;
wire _guard79 = _guard77 & _guard78;
wire _guard80 = fsm0_out == 2'd1;
wire _guard81 = do_ar_transfer_done_out;
wire _guard82 = _guard80 & _guard81;
wire _guard83 = tdcc_go_out;
wire _guard84 = _guard82 & _guard83;
wire _guard85 = do_ar_transfer_go_out;
wire _guard86 = early_reset_static_par_go_out;
wire _guard87 = _guard85 | _guard86;
wire _guard88 = ARREADY;
wire _guard89 = arvalid_out;
wire _guard90 = _guard88 & _guard89;
wire _guard91 = do_ar_transfer_go_out;
wire _guard92 = _guard90 & _guard91;
wire _guard93 = ARREADY;
wire _guard94 = arvalid_out;
wire _guard95 = _guard93 & _guard94;
wire _guard96 = ~_guard95;
wire _guard97 = do_ar_transfer_go_out;
wire _guard98 = _guard96 & _guard97;
wire _guard99 = early_reset_static_par_go_out;
wire _guard100 = _guard98 | _guard99;
wire _guard101 = fsm_out == 1'd0;
wire _guard102 = signal_reg_out;
wire _guard103 = _guard101 & _guard102;
wire _guard104 = fsm_out == 1'd0;
wire _guard105 = signal_reg_out;
wire _guard106 = ~_guard105;
wire _guard107 = _guard104 & _guard106;
wire _guard108 = wrapper_early_reset_static_par_go_out;
wire _guard109 = _guard107 & _guard108;
wire _guard110 = _guard103 | _guard109;
wire _guard111 = fsm_out == 1'd0;
wire _guard112 = signal_reg_out;
wire _guard113 = ~_guard112;
wire _guard114 = _guard111 & _guard113;
wire _guard115 = wrapper_early_reset_static_par_go_out;
wire _guard116 = _guard114 & _guard115;
wire _guard117 = fsm_out == 1'd0;
wire _guard118 = signal_reg_out;
wire _guard119 = _guard117 & _guard118;
wire _guard120 = do_ar_transfer_go_out;
wire _guard121 = invoke2_go_out;
wire _guard122 = _guard120 | _guard121;
wire _guard123 = arvalid_out;
wire _guard124 = ARREADY;
wire _guard125 = _guard123 & _guard124;
wire _guard126 = ~_guard125;
wire _guard127 = ar_handshake_occurred_out;
wire _guard128 = ~_guard127;
wire _guard129 = _guard126 & _guard128;
wire _guard130 = do_ar_transfer_go_out;
wire _guard131 = _guard129 & _guard130;
wire _guard132 = arvalid_out;
wire _guard133 = ARREADY;
wire _guard134 = _guard132 & _guard133;
wire _guard135 = ar_handshake_occurred_out;
wire _guard136 = _guard134 | _guard135;
wire _guard137 = do_ar_transfer_go_out;
wire _guard138 = _guard136 & _guard137;
wire _guard139 = invoke2_go_out;
wire _guard140 = _guard138 | _guard139;
wire _guard141 = fsm0_out == 2'd3;
wire _guard142 = wrapper_early_reset_static_par_go_out;
assign do_ar_transfer_go_in = _guard6;
assign done = _guard7;
assign ARPROT =
  _guard8 ? 3'd6 :
  3'd0;
assign ARSIZE =
  _guard9 ? 3'd2 :
  3'd0;
assign ARLEN =
  _guard10 ? 8'd0 :
  8'd0;
assign ARADDR =
  _guard11 ? axi_address :
  64'd0;
assign ARBURST =
  _guard12 ? 2'd1 :
  2'd0;
assign ARVALID = arvalid_out;
assign fsm_write_en = _guard13;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard17 ? adder_out :
  _guard20 ? 1'd0 :
  1'd0;
assign adder_left =
  _guard21 ? fsm_out :
  1'd0;
assign adder_right = _guard22;
assign ar_handshake_occurred_write_en = _guard28;
assign ar_handshake_occurred_clk = clk;
assign ar_handshake_occurred_reset = reset;
assign ar_handshake_occurred_in =
  _guard33 ? 1'd1 :
  _guard34 ? 1'd0 :
  'x;
assign invoke2_go_in = _guard40;
assign wrapper_early_reset_static_par_go_in = _guard46;
assign wrapper_early_reset_static_par_done_in = _guard49;
assign tdcc_go_in = go;
assign fsm0_write_en = _guard68;
assign fsm0_clk = clk;
assign fsm0_reset = reset;
assign fsm0_in =
  _guard73 ? 2'd1 :
  _guard74 ? 2'd0 :
  _guard79 ? 2'd3 :
  _guard84 ? 2'd2 :
  2'd0;
assign early_reset_static_par_done_in = ud_out;
assign bt_reg_write_en = _guard87;
assign bt_reg_clk = clk;
assign bt_reg_reset = reset;
assign bt_reg_in =
  _guard92 ? 1'd1 :
  _guard100 ? 1'd0 :
  'x;
assign signal_reg_write_en = _guard110;
assign signal_reg_clk = clk;
assign signal_reg_reset = reset;
assign signal_reg_in =
  _guard116 ? 1'd1 :
  _guard119 ? 1'd0 :
  1'd0;
assign invoke2_done_in = arvalid_done;
assign arvalid_write_en = _guard122;
assign arvalid_clk = clk;
assign arvalid_reset = reset;
assign arvalid_in =
  _guard131 ? 1'd1 :
  _guard140 ? 1'd0 :
  'x;
assign tdcc_done_in = _guard141;
assign early_reset_static_par_go_in = _guard142;
assign do_ar_transfer_done_in = bt_reg_out;
// COMPONENT END: m_ar_channel_B0
endmodule
module m_aw_channel_B0(
  input logic ARESETn,
  input logic AWREADY,
  input logic [63:0] axi_address,
  output logic AWVALID,
  output logic [63:0] AWADDR,
  output logic [2:0] AWSIZE,
  output logic [7:0] AWLEN,
  output logic [1:0] AWBURST,
  output logic [2:0] AWPROT,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: m_aw_channel_B0
logic awvalid_in;
logic awvalid_write_en;
logic awvalid_clk;
logic awvalid_reset;
logic awvalid_out;
logic awvalid_done;
logic aw_handshake_occurred_in;
logic aw_handshake_occurred_write_en;
logic aw_handshake_occurred_clk;
logic aw_handshake_occurred_reset;
logic aw_handshake_occurred_out;
logic aw_handshake_occurred_done;
logic bt_reg_in;
logic bt_reg_write_en;
logic bt_reg_clk;
logic bt_reg_reset;
logic bt_reg_out;
logic bt_reg_done;
logic fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic fsm_out;
logic fsm_done;
logic adder_left;
logic adder_right;
logic adder_out;
logic ud_out;
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
logic do_aw_transfer_go_in;
logic do_aw_transfer_go_out;
logic do_aw_transfer_done_in;
logic do_aw_transfer_done_out;
logic invoke2_go_in;
logic invoke2_go_out;
logic invoke2_done_in;
logic invoke2_done_out;
logic early_reset_static_par_go_in;
logic early_reset_static_par_go_out;
logic early_reset_static_par_done_in;
logic early_reset_static_par_done_out;
logic wrapper_early_reset_static_par_go_in;
logic wrapper_early_reset_static_par_go_out;
logic wrapper_early_reset_static_par_done_in;
logic wrapper_early_reset_static_par_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
std_reg # (
    .WIDTH(1)
) awvalid (
    .clk(awvalid_clk),
    .done(awvalid_done),
    .in(awvalid_in),
    .out(awvalid_out),
    .reset(awvalid_reset),
    .write_en(awvalid_write_en)
);
std_reg # (
    .WIDTH(1)
) aw_handshake_occurred (
    .clk(aw_handshake_occurred_clk),
    .done(aw_handshake_occurred_done),
    .in(aw_handshake_occurred_in),
    .out(aw_handshake_occurred_out),
    .reset(aw_handshake_occurred_reset),
    .write_en(aw_handshake_occurred_write_en)
);
std_reg # (
    .WIDTH(1)
) bt_reg (
    .clk(bt_reg_clk),
    .done(bt_reg_done),
    .in(bt_reg_in),
    .out(bt_reg_out),
    .reset(bt_reg_reset),
    .write_en(bt_reg_write_en)
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
std_add # (
    .WIDTH(1)
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
) do_aw_transfer_go (
    .in(do_aw_transfer_go_in),
    .out(do_aw_transfer_go_out)
);
std_wire # (
    .WIDTH(1)
) do_aw_transfer_done (
    .in(do_aw_transfer_done_in),
    .out(do_aw_transfer_done_out)
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
wire _guard1 = tdcc_done_out;
wire _guard2 = do_aw_transfer_go_out;
wire _guard3 = do_aw_transfer_go_out;
wire _guard4 = do_aw_transfer_go_out;
wire _guard5 = do_aw_transfer_go_out;
wire _guard6 = do_aw_transfer_go_out;
wire _guard7 = early_reset_static_par_go_out;
wire _guard8 = fsm_out == 1'd0;
wire _guard9 = ~_guard8;
wire _guard10 = early_reset_static_par_go_out;
wire _guard11 = _guard9 & _guard10;
wire _guard12 = fsm_out == 1'd0;
wire _guard13 = early_reset_static_par_go_out;
wire _guard14 = _guard12 & _guard13;
wire _guard15 = early_reset_static_par_go_out;
wire _guard16 = early_reset_static_par_go_out;
wire _guard17 = invoke2_done_out;
wire _guard18 = ~_guard17;
wire _guard19 = fsm0_out == 2'd2;
wire _guard20 = _guard18 & _guard19;
wire _guard21 = tdcc_go_out;
wire _guard22 = _guard20 & _guard21;
wire _guard23 = wrapper_early_reset_static_par_done_out;
wire _guard24 = ~_guard23;
wire _guard25 = fsm0_out == 2'd0;
wire _guard26 = _guard24 & _guard25;
wire _guard27 = tdcc_go_out;
wire _guard28 = _guard26 & _guard27;
wire _guard29 = fsm_out == 1'd0;
wire _guard30 = signal_reg_out;
wire _guard31 = _guard29 & _guard30;
wire _guard32 = fsm0_out == 2'd3;
wire _guard33 = fsm0_out == 2'd0;
wire _guard34 = wrapper_early_reset_static_par_done_out;
wire _guard35 = _guard33 & _guard34;
wire _guard36 = tdcc_go_out;
wire _guard37 = _guard35 & _guard36;
wire _guard38 = _guard32 | _guard37;
wire _guard39 = fsm0_out == 2'd1;
wire _guard40 = do_aw_transfer_done_out;
wire _guard41 = _guard39 & _guard40;
wire _guard42 = tdcc_go_out;
wire _guard43 = _guard41 & _guard42;
wire _guard44 = _guard38 | _guard43;
wire _guard45 = fsm0_out == 2'd2;
wire _guard46 = invoke2_done_out;
wire _guard47 = _guard45 & _guard46;
wire _guard48 = tdcc_go_out;
wire _guard49 = _guard47 & _guard48;
wire _guard50 = _guard44 | _guard49;
wire _guard51 = fsm0_out == 2'd0;
wire _guard52 = wrapper_early_reset_static_par_done_out;
wire _guard53 = _guard51 & _guard52;
wire _guard54 = tdcc_go_out;
wire _guard55 = _guard53 & _guard54;
wire _guard56 = fsm0_out == 2'd3;
wire _guard57 = fsm0_out == 2'd2;
wire _guard58 = invoke2_done_out;
wire _guard59 = _guard57 & _guard58;
wire _guard60 = tdcc_go_out;
wire _guard61 = _guard59 & _guard60;
wire _guard62 = fsm0_out == 2'd1;
wire _guard63 = do_aw_transfer_done_out;
wire _guard64 = _guard62 & _guard63;
wire _guard65 = tdcc_go_out;
wire _guard66 = _guard64 & _guard65;
wire _guard67 = do_aw_transfer_done_out;
wire _guard68 = ~_guard67;
wire _guard69 = fsm0_out == 2'd1;
wire _guard70 = _guard68 & _guard69;
wire _guard71 = tdcc_go_out;
wire _guard72 = _guard70 & _guard71;
wire _guard73 = do_aw_transfer_go_out;
wire _guard74 = early_reset_static_par_go_out;
wire _guard75 = _guard73 | _guard74;
wire _guard76 = AWREADY;
wire _guard77 = awvalid_out;
wire _guard78 = _guard76 & _guard77;
wire _guard79 = do_aw_transfer_go_out;
wire _guard80 = _guard78 & _guard79;
wire _guard81 = AWREADY;
wire _guard82 = awvalid_out;
wire _guard83 = _guard81 & _guard82;
wire _guard84 = ~_guard83;
wire _guard85 = do_aw_transfer_go_out;
wire _guard86 = _guard84 & _guard85;
wire _guard87 = early_reset_static_par_go_out;
wire _guard88 = _guard86 | _guard87;
wire _guard89 = fsm_out == 1'd0;
wire _guard90 = signal_reg_out;
wire _guard91 = _guard89 & _guard90;
wire _guard92 = fsm_out == 1'd0;
wire _guard93 = signal_reg_out;
wire _guard94 = ~_guard93;
wire _guard95 = _guard92 & _guard94;
wire _guard96 = wrapper_early_reset_static_par_go_out;
wire _guard97 = _guard95 & _guard96;
wire _guard98 = _guard91 | _guard97;
wire _guard99 = fsm_out == 1'd0;
wire _guard100 = signal_reg_out;
wire _guard101 = ~_guard100;
wire _guard102 = _guard99 & _guard101;
wire _guard103 = wrapper_early_reset_static_par_go_out;
wire _guard104 = _guard102 & _guard103;
wire _guard105 = fsm_out == 1'd0;
wire _guard106 = signal_reg_out;
wire _guard107 = _guard105 & _guard106;
wire _guard108 = aw_handshake_occurred_out;
wire _guard109 = ~_guard108;
wire _guard110 = do_aw_transfer_go_out;
wire _guard111 = _guard109 & _guard110;
wire _guard112 = early_reset_static_par_go_out;
wire _guard113 = _guard111 | _guard112;
wire _guard114 = awvalid_out;
wire _guard115 = AWREADY;
wire _guard116 = _guard114 & _guard115;
wire _guard117 = do_aw_transfer_go_out;
wire _guard118 = _guard116 & _guard117;
wire _guard119 = early_reset_static_par_go_out;
wire _guard120 = fsm0_out == 2'd3;
wire _guard121 = do_aw_transfer_go_out;
wire _guard122 = invoke2_go_out;
wire _guard123 = _guard121 | _guard122;
wire _guard124 = awvalid_out;
wire _guard125 = AWREADY;
wire _guard126 = _guard124 & _guard125;
wire _guard127 = ~_guard126;
wire _guard128 = aw_handshake_occurred_out;
wire _guard129 = ~_guard128;
wire _guard130 = _guard127 & _guard129;
wire _guard131 = do_aw_transfer_go_out;
wire _guard132 = _guard130 & _guard131;
wire _guard133 = awvalid_out;
wire _guard134 = AWREADY;
wire _guard135 = _guard133 & _guard134;
wire _guard136 = aw_handshake_occurred_out;
wire _guard137 = _guard135 | _guard136;
wire _guard138 = do_aw_transfer_go_out;
wire _guard139 = _guard137 & _guard138;
wire _guard140 = invoke2_go_out;
wire _guard141 = _guard139 | _guard140;
wire _guard142 = wrapper_early_reset_static_par_go_out;
assign done = _guard1;
assign AWADDR =
  _guard2 ? axi_address :
  64'd0;
assign AWPROT =
  _guard3 ? 3'd6 :
  3'd0;
assign AWSIZE =
  _guard4 ? 3'd2 :
  3'd0;
assign AWVALID = awvalid_out;
assign AWBURST =
  _guard5 ? 2'd1 :
  2'd0;
assign AWLEN =
  _guard6 ? 8'd0 :
  8'd0;
assign fsm_write_en = _guard7;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard11 ? adder_out :
  _guard14 ? 1'd0 :
  1'd0;
assign adder_left =
  _guard15 ? fsm_out :
  1'd0;
assign adder_right = _guard16;
assign invoke2_go_in = _guard22;
assign wrapper_early_reset_static_par_go_in = _guard28;
assign wrapper_early_reset_static_par_done_in = _guard31;
assign tdcc_go_in = go;
assign fsm0_write_en = _guard50;
assign fsm0_clk = clk;
assign fsm0_reset = reset;
assign fsm0_in =
  _guard55 ? 2'd1 :
  _guard56 ? 2'd0 :
  _guard61 ? 2'd3 :
  _guard66 ? 2'd2 :
  2'd0;
assign do_aw_transfer_go_in = _guard72;
assign do_aw_transfer_done_in = bt_reg_out;
assign early_reset_static_par_done_in = ud_out;
assign bt_reg_write_en = _guard75;
assign bt_reg_clk = clk;
assign bt_reg_reset = reset;
assign bt_reg_in =
  _guard80 ? 1'd1 :
  _guard88 ? 1'd0 :
  'x;
assign signal_reg_write_en = _guard98;
assign signal_reg_clk = clk;
assign signal_reg_reset = reset;
assign signal_reg_in =
  _guard104 ? 1'd1 :
  _guard107 ? 1'd0 :
  1'd0;
assign invoke2_done_in = awvalid_done;
assign aw_handshake_occurred_write_en = _guard113;
assign aw_handshake_occurred_clk = clk;
assign aw_handshake_occurred_reset = reset;
assign aw_handshake_occurred_in =
  _guard118 ? 1'd1 :
  _guard119 ? 1'd0 :
  'x;
assign tdcc_done_in = _guard120;
assign awvalid_write_en = _guard123;
assign awvalid_clk = clk;
assign awvalid_reset = reset;
assign awvalid_in =
  _guard132 ? 1'd1 :
  _guard141 ? 1'd0 :
  'x;
assign early_reset_static_par_go_in = _guard142;
// COMPONENT END: m_aw_channel_B0
endmodule
module m_read_channel_B0(
  input logic ARESETn,
  input logic RVALID,
  input logic RLAST,
  input logic [31:0] RDATA,
  input logic [1:0] RRESP,
  output logic RREADY,
  output logic [31:0] read_data,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: m_read_channel_B0
logic [31:0] read_reg_in;
logic read_reg_write_en;
logic read_reg_clk;
logic read_reg_reset;
logic [31:0] read_reg_out;
logic read_reg_done;
logic rready_in;
logic rready_write_en;
logic rready_clk;
logic rready_reset;
logic rready_out;
logic rready_done;
logic n_RLAST_in;
logic n_RLAST_write_en;
logic n_RLAST_clk;
logic n_RLAST_reset;
logic n_RLAST_out;
logic n_RLAST_done;
logic [1:0] fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic [1:0] fsm_out;
logic fsm_done;
logic block_transfer_go_in;
logic block_transfer_go_out;
logic block_transfer_done_in;
logic block_transfer_done_out;
logic invoke0_go_in;
logic invoke0_go_out;
logic invoke0_done_in;
logic invoke0_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
std_reg # (
    .WIDTH(32)
) read_reg (
    .clk(read_reg_clk),
    .done(read_reg_done),
    .in(read_reg_in),
    .out(read_reg_out),
    .reset(read_reg_reset),
    .write_en(read_reg_write_en)
);
std_reg # (
    .WIDTH(1)
) rready (
    .clk(rready_clk),
    .done(rready_done),
    .in(rready_in),
    .out(rready_out),
    .reset(rready_reset),
    .write_en(rready_write_en)
);
std_reg # (
    .WIDTH(1)
) n_RLAST (
    .clk(n_RLAST_clk),
    .done(n_RLAST_done),
    .in(n_RLAST_in),
    .out(n_RLAST_out),
    .reset(n_RLAST_reset),
    .write_en(n_RLAST_write_en)
);
std_reg # (
    .WIDTH(2)
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
) block_transfer_go (
    .in(block_transfer_go_in),
    .out(block_transfer_go_out)
);
std_wire # (
    .WIDTH(1)
) block_transfer_done (
    .in(block_transfer_done_in),
    .out(block_transfer_done_out)
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
wire _guard1 = tdcc_done_out;
wire _guard2 = fsm_out == 2'd2;
wire _guard3 = fsm_out == 2'd0;
wire _guard4 = invoke0_done_out;
wire _guard5 = n_RLAST_out;
wire _guard6 = _guard4 & _guard5;
wire _guard7 = _guard3 & _guard6;
wire _guard8 = tdcc_go_out;
wire _guard9 = _guard7 & _guard8;
wire _guard10 = _guard2 | _guard9;
wire _guard11 = fsm_out == 2'd1;
wire _guard12 = block_transfer_done_out;
wire _guard13 = n_RLAST_out;
wire _guard14 = _guard12 & _guard13;
wire _guard15 = _guard11 & _guard14;
wire _guard16 = tdcc_go_out;
wire _guard17 = _guard15 & _guard16;
wire _guard18 = _guard10 | _guard17;
wire _guard19 = fsm_out == 2'd0;
wire _guard20 = invoke0_done_out;
wire _guard21 = n_RLAST_out;
wire _guard22 = ~_guard21;
wire _guard23 = _guard20 & _guard22;
wire _guard24 = _guard19 & _guard23;
wire _guard25 = tdcc_go_out;
wire _guard26 = _guard24 & _guard25;
wire _guard27 = _guard18 | _guard26;
wire _guard28 = fsm_out == 2'd1;
wire _guard29 = block_transfer_done_out;
wire _guard30 = n_RLAST_out;
wire _guard31 = ~_guard30;
wire _guard32 = _guard29 & _guard31;
wire _guard33 = _guard28 & _guard32;
wire _guard34 = tdcc_go_out;
wire _guard35 = _guard33 & _guard34;
wire _guard36 = _guard27 | _guard35;
wire _guard37 = fsm_out == 2'd0;
wire _guard38 = invoke0_done_out;
wire _guard39 = n_RLAST_out;
wire _guard40 = _guard38 & _guard39;
wire _guard41 = _guard37 & _guard40;
wire _guard42 = tdcc_go_out;
wire _guard43 = _guard41 & _guard42;
wire _guard44 = fsm_out == 2'd1;
wire _guard45 = block_transfer_done_out;
wire _guard46 = n_RLAST_out;
wire _guard47 = _guard45 & _guard46;
wire _guard48 = _guard44 & _guard47;
wire _guard49 = tdcc_go_out;
wire _guard50 = _guard48 & _guard49;
wire _guard51 = _guard43 | _guard50;
wire _guard52 = fsm_out == 2'd2;
wire _guard53 = fsm_out == 2'd0;
wire _guard54 = invoke0_done_out;
wire _guard55 = n_RLAST_out;
wire _guard56 = ~_guard55;
wire _guard57 = _guard54 & _guard56;
wire _guard58 = _guard53 & _guard57;
wire _guard59 = tdcc_go_out;
wire _guard60 = _guard58 & _guard59;
wire _guard61 = fsm_out == 2'd1;
wire _guard62 = block_transfer_done_out;
wire _guard63 = n_RLAST_out;
wire _guard64 = ~_guard63;
wire _guard65 = _guard62 & _guard64;
wire _guard66 = _guard61 & _guard65;
wire _guard67 = tdcc_go_out;
wire _guard68 = _guard66 & _guard67;
wire _guard69 = _guard60 | _guard68;
wire _guard70 = rready_out;
wire _guard71 = RVALID;
wire _guard72 = _guard70 & _guard71;
wire _guard73 = block_transfer_go_out;
wire _guard74 = _guard72 & _guard73;
wire _guard75 = rready_out;
wire _guard76 = RVALID;
wire _guard77 = _guard75 & _guard76;
wire _guard78 = ~_guard77;
wire _guard79 = block_transfer_go_out;
wire _guard80 = _guard78 & _guard79;
wire _guard81 = block_transfer_go_out;
wire _guard82 = invoke0_done_out;
wire _guard83 = ~_guard82;
wire _guard84 = fsm_out == 2'd0;
wire _guard85 = _guard83 & _guard84;
wire _guard86 = tdcc_go_out;
wire _guard87 = _guard85 & _guard86;
wire _guard88 = block_transfer_go_out;
wire _guard89 = invoke0_go_out;
wire _guard90 = _guard88 | _guard89;
wire _guard91 = RLAST;
wire _guard92 = ~_guard91;
wire _guard93 = block_transfer_go_out;
wire _guard94 = _guard92 & _guard93;
wire _guard95 = invoke0_go_out;
wire _guard96 = _guard94 | _guard95;
wire _guard97 = RLAST;
wire _guard98 = block_transfer_go_out;
wire _guard99 = _guard97 & _guard98;
wire _guard100 = fsm_out == 2'd2;
wire _guard101 = block_transfer_done_out;
wire _guard102 = ~_guard101;
wire _guard103 = fsm_out == 2'd1;
wire _guard104 = _guard102 & _guard103;
wire _guard105 = tdcc_go_out;
wire _guard106 = _guard104 & _guard105;
wire _guard107 = block_transfer_go_out;
wire _guard108 = rready_out;
wire _guard109 = RVALID;
wire _guard110 = _guard108 & _guard109;
wire _guard111 = ~_guard110;
wire _guard112 = block_transfer_go_out;
wire _guard113 = _guard111 & _guard112;
wire _guard114 = rready_out;
wire _guard115 = RVALID;
wire _guard116 = _guard114 & _guard115;
wire _guard117 = block_transfer_go_out;
wire _guard118 = _guard116 & _guard117;
assign done = _guard1;
assign RREADY = rready_out;
assign read_data = read_reg_out;
assign fsm_write_en = _guard36;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard51 ? 2'd1 :
  _guard52 ? 2'd0 :
  _guard69 ? 2'd2 :
  2'd0;
assign block_transfer_done_in = read_reg_done;
assign read_reg_write_en =
  _guard74 ? 1'd1 :
  _guard80 ? 1'd0 :
  1'd0;
assign read_reg_clk = clk;
assign read_reg_reset = reset;
assign read_reg_in = RDATA;
assign tdcc_go_in = go;
assign invoke0_go_in = _guard87;
assign n_RLAST_write_en = _guard90;
assign n_RLAST_clk = clk;
assign n_RLAST_reset = reset;
assign n_RLAST_in =
  _guard96 ? 1'd1 :
  _guard99 ? 1'd0 :
  'x;
assign invoke0_done_in = n_RLAST_done;
assign tdcc_done_in = _guard100;
assign block_transfer_go_in = _guard106;
assign rready_write_en = _guard107;
assign rready_clk = clk;
assign rready_reset = reset;
assign rready_in =
  _guard113 ? 1'd1 :
  _guard118 ? 1'd0 :
  'x;
// COMPONENT END: m_read_channel_B0
endmodule
module m_write_channel_B0(
  input logic ARESETn,
  input logic WREADY,
  input logic [31:0] write_data,
  output logic WVALID,
  output logic WLAST,
  output logic [31:0] WDATA,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: m_write_channel_B0
logic wvalid_in;
logic wvalid_write_en;
logic wvalid_clk;
logic wvalid_reset;
logic wvalid_out;
logic wvalid_done;
logic w_handshake_occurred_in;
logic w_handshake_occurred_write_en;
logic w_handshake_occurred_clk;
logic w_handshake_occurred_reset;
logic w_handshake_occurred_out;
logic w_handshake_occurred_done;
logic bt_reg_in;
logic bt_reg_write_en;
logic bt_reg_clk;
logic bt_reg_reset;
logic bt_reg_out;
logic bt_reg_done;
logic fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic fsm_out;
logic fsm_done;
logic adder_left;
logic adder_right;
logic adder_out;
logic ud_out;
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
logic service_write_transfer_go_in;
logic service_write_transfer_go_out;
logic service_write_transfer_done_in;
logic service_write_transfer_done_out;
logic early_reset_static_par_go_in;
logic early_reset_static_par_go_out;
logic early_reset_static_par_done_in;
logic early_reset_static_par_done_out;
logic wrapper_early_reset_static_par_go_in;
logic wrapper_early_reset_static_par_go_out;
logic wrapper_early_reset_static_par_done_in;
logic wrapper_early_reset_static_par_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
std_reg # (
    .WIDTH(1)
) wvalid (
    .clk(wvalid_clk),
    .done(wvalid_done),
    .in(wvalid_in),
    .out(wvalid_out),
    .reset(wvalid_reset),
    .write_en(wvalid_write_en)
);
std_reg # (
    .WIDTH(1)
) w_handshake_occurred (
    .clk(w_handshake_occurred_clk),
    .done(w_handshake_occurred_done),
    .in(w_handshake_occurred_in),
    .out(w_handshake_occurred_out),
    .reset(w_handshake_occurred_reset),
    .write_en(w_handshake_occurred_write_en)
);
std_reg # (
    .WIDTH(1)
) bt_reg (
    .clk(bt_reg_clk),
    .done(bt_reg_done),
    .in(bt_reg_in),
    .out(bt_reg_out),
    .reset(bt_reg_reset),
    .write_en(bt_reg_write_en)
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
std_add # (
    .WIDTH(1)
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
) service_write_transfer_go (
    .in(service_write_transfer_go_in),
    .out(service_write_transfer_go_out)
);
std_wire # (
    .WIDTH(1)
) service_write_transfer_done (
    .in(service_write_transfer_done_in),
    .out(service_write_transfer_done_out)
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
wire _guard1 = tdcc_done_out;
wire _guard2 = service_write_transfer_go_out;
wire _guard3 = service_write_transfer_go_out;
wire _guard4 = early_reset_static_par_go_out;
wire _guard5 = fsm_out == 1'd0;
wire _guard6 = ~_guard5;
wire _guard7 = early_reset_static_par_go_out;
wire _guard8 = _guard6 & _guard7;
wire _guard9 = fsm_out == 1'd0;
wire _guard10 = early_reset_static_par_go_out;
wire _guard11 = _guard9 & _guard10;
wire _guard12 = early_reset_static_par_go_out;
wire _guard13 = early_reset_static_par_go_out;
wire _guard14 = service_write_transfer_go_out;
wire _guard15 = wvalid_out;
wire _guard16 = WREADY;
wire _guard17 = _guard15 & _guard16;
wire _guard18 = ~_guard17;
wire _guard19 = w_handshake_occurred_out;
wire _guard20 = ~_guard19;
wire _guard21 = _guard18 & _guard20;
wire _guard22 = service_write_transfer_go_out;
wire _guard23 = _guard21 & _guard22;
wire _guard24 = wvalid_out;
wire _guard25 = WREADY;
wire _guard26 = _guard24 & _guard25;
wire _guard27 = w_handshake_occurred_out;
wire _guard28 = _guard26 | _guard27;
wire _guard29 = service_write_transfer_go_out;
wire _guard30 = _guard28 & _guard29;
wire _guard31 = wrapper_early_reset_static_par_done_out;
wire _guard32 = ~_guard31;
wire _guard33 = fsm0_out == 2'd0;
wire _guard34 = _guard32 & _guard33;
wire _guard35 = tdcc_go_out;
wire _guard36 = _guard34 & _guard35;
wire _guard37 = fsm_out == 1'd0;
wire _guard38 = signal_reg_out;
wire _guard39 = _guard37 & _guard38;
wire _guard40 = fsm0_out == 2'd2;
wire _guard41 = fsm0_out == 2'd0;
wire _guard42 = wrapper_early_reset_static_par_done_out;
wire _guard43 = _guard41 & _guard42;
wire _guard44 = tdcc_go_out;
wire _guard45 = _guard43 & _guard44;
wire _guard46 = _guard40 | _guard45;
wire _guard47 = fsm0_out == 2'd1;
wire _guard48 = service_write_transfer_done_out;
wire _guard49 = _guard47 & _guard48;
wire _guard50 = tdcc_go_out;
wire _guard51 = _guard49 & _guard50;
wire _guard52 = _guard46 | _guard51;
wire _guard53 = fsm0_out == 2'd0;
wire _guard54 = wrapper_early_reset_static_par_done_out;
wire _guard55 = _guard53 & _guard54;
wire _guard56 = tdcc_go_out;
wire _guard57 = _guard55 & _guard56;
wire _guard58 = fsm0_out == 2'd2;
wire _guard59 = fsm0_out == 2'd1;
wire _guard60 = service_write_transfer_done_out;
wire _guard61 = _guard59 & _guard60;
wire _guard62 = tdcc_go_out;
wire _guard63 = _guard61 & _guard62;
wire _guard64 = service_write_transfer_done_out;
wire _guard65 = ~_guard64;
wire _guard66 = fsm0_out == 2'd1;
wire _guard67 = _guard65 & _guard66;
wire _guard68 = tdcc_go_out;
wire _guard69 = _guard67 & _guard68;
wire _guard70 = service_write_transfer_go_out;
wire _guard71 = early_reset_static_par_go_out;
wire _guard72 = _guard70 | _guard71;
wire _guard73 = wvalid_out;
wire _guard74 = WREADY;
wire _guard75 = _guard73 & _guard74;
wire _guard76 = service_write_transfer_go_out;
wire _guard77 = _guard75 & _guard76;
wire _guard78 = wvalid_out;
wire _guard79 = WREADY;
wire _guard80 = _guard78 & _guard79;
wire _guard81 = ~_guard80;
wire _guard82 = service_write_transfer_go_out;
wire _guard83 = _guard81 & _guard82;
wire _guard84 = early_reset_static_par_go_out;
wire _guard85 = _guard83 | _guard84;
wire _guard86 = fsm_out == 1'd0;
wire _guard87 = signal_reg_out;
wire _guard88 = _guard86 & _guard87;
wire _guard89 = fsm_out == 1'd0;
wire _guard90 = signal_reg_out;
wire _guard91 = ~_guard90;
wire _guard92 = _guard89 & _guard91;
wire _guard93 = wrapper_early_reset_static_par_go_out;
wire _guard94 = _guard92 & _guard93;
wire _guard95 = _guard88 | _guard94;
wire _guard96 = fsm_out == 1'd0;
wire _guard97 = signal_reg_out;
wire _guard98 = ~_guard97;
wire _guard99 = _guard96 & _guard98;
wire _guard100 = wrapper_early_reset_static_par_go_out;
wire _guard101 = _guard99 & _guard100;
wire _guard102 = fsm_out == 1'd0;
wire _guard103 = signal_reg_out;
wire _guard104 = _guard102 & _guard103;
wire _guard105 = w_handshake_occurred_out;
wire _guard106 = ~_guard105;
wire _guard107 = service_write_transfer_go_out;
wire _guard108 = _guard106 & _guard107;
wire _guard109 = early_reset_static_par_go_out;
wire _guard110 = _guard108 | _guard109;
wire _guard111 = wvalid_out;
wire _guard112 = WREADY;
wire _guard113 = _guard111 & _guard112;
wire _guard114 = service_write_transfer_go_out;
wire _guard115 = _guard113 & _guard114;
wire _guard116 = wvalid_out;
wire _guard117 = WREADY;
wire _guard118 = _guard116 & _guard117;
wire _guard119 = ~_guard118;
wire _guard120 = service_write_transfer_go_out;
wire _guard121 = _guard119 & _guard120;
wire _guard122 = early_reset_static_par_go_out;
wire _guard123 = _guard121 | _guard122;
wire _guard124 = fsm0_out == 2'd2;
wire _guard125 = wrapper_early_reset_static_par_go_out;
assign done = _guard1;
assign WVALID = wvalid_out;
assign WDATA =
  _guard2 ? write_data :
  32'd0;
assign WLAST = _guard3;
assign fsm_write_en = _guard4;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard8 ? adder_out :
  _guard11 ? 1'd0 :
  1'd0;
assign adder_left =
  _guard12 ? fsm_out :
  1'd0;
assign adder_right = _guard13;
assign wvalid_write_en = _guard14;
assign wvalid_clk = clk;
assign wvalid_reset = reset;
assign wvalid_in =
  _guard23 ? 1'd1 :
  _guard30 ? 1'd0 :
  'x;
assign wrapper_early_reset_static_par_go_in = _guard36;
assign wrapper_early_reset_static_par_done_in = _guard39;
assign tdcc_go_in = go;
assign service_write_transfer_done_in = bt_reg_out;
assign fsm0_write_en = _guard52;
assign fsm0_clk = clk;
assign fsm0_reset = reset;
assign fsm0_in =
  _guard57 ? 2'd1 :
  _guard58 ? 2'd0 :
  _guard63 ? 2'd2 :
  2'd0;
assign service_write_transfer_go_in = _guard69;
assign early_reset_static_par_done_in = ud_out;
assign bt_reg_write_en = _guard72;
assign bt_reg_clk = clk;
assign bt_reg_reset = reset;
assign bt_reg_in =
  _guard77 ? 1'd1 :
  _guard85 ? 1'd0 :
  'x;
assign signal_reg_write_en = _guard95;
assign signal_reg_clk = clk;
assign signal_reg_reset = reset;
assign signal_reg_in =
  _guard101 ? 1'd1 :
  _guard104 ? 1'd0 :
  1'd0;
assign w_handshake_occurred_write_en = _guard110;
assign w_handshake_occurred_clk = clk;
assign w_handshake_occurred_reset = reset;
assign w_handshake_occurred_in =
  _guard115 ? 1'd1 :
  _guard123 ? 1'd0 :
  'x;
assign tdcc_done_in = _guard124;
assign early_reset_static_par_go_in = _guard125;
// COMPONENT END: m_write_channel_B0
endmodule
module m_bresp_channel_B0(
  input logic ARESETn,
  input logic BVALID,
  output logic BREADY,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: m_bresp_channel_B0
logic bready_in;
logic bready_write_en;
logic bready_clk;
logic bready_reset;
logic bready_out;
logic bready_done;
logic bt_reg_in;
logic bt_reg_write_en;
logic bt_reg_clk;
logic bt_reg_reset;
logic bt_reg_out;
logic bt_reg_done;
logic [1:0] fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic [1:0] fsm_out;
logic fsm_done;
logic block_transfer_go_in;
logic block_transfer_go_out;
logic block_transfer_done_in;
logic block_transfer_done_out;
logic invoke0_go_in;
logic invoke0_go_out;
logic invoke0_done_in;
logic invoke0_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
std_reg # (
    .WIDTH(1)
) bready (
    .clk(bready_clk),
    .done(bready_done),
    .in(bready_in),
    .out(bready_out),
    .reset(bready_reset),
    .write_en(bready_write_en)
);
std_reg # (
    .WIDTH(1)
) bt_reg (
    .clk(bt_reg_clk),
    .done(bt_reg_done),
    .in(bt_reg_in),
    .out(bt_reg_out),
    .reset(bt_reg_reset),
    .write_en(bt_reg_write_en)
);
std_reg # (
    .WIDTH(2)
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
) block_transfer_go (
    .in(block_transfer_go_in),
    .out(block_transfer_go_out)
);
std_wire # (
    .WIDTH(1)
) block_transfer_done (
    .in(block_transfer_done_in),
    .out(block_transfer_done_out)
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
wire _guard1 = tdcc_done_out;
wire _guard2 = fsm_out == 2'd2;
wire _guard3 = fsm_out == 2'd0;
wire _guard4 = invoke0_done_out;
wire _guard5 = _guard3 & _guard4;
wire _guard6 = tdcc_go_out;
wire _guard7 = _guard5 & _guard6;
wire _guard8 = _guard2 | _guard7;
wire _guard9 = fsm_out == 2'd1;
wire _guard10 = block_transfer_done_out;
wire _guard11 = _guard9 & _guard10;
wire _guard12 = tdcc_go_out;
wire _guard13 = _guard11 & _guard12;
wire _guard14 = _guard8 | _guard13;
wire _guard15 = fsm_out == 2'd0;
wire _guard16 = invoke0_done_out;
wire _guard17 = _guard15 & _guard16;
wire _guard18 = tdcc_go_out;
wire _guard19 = _guard17 & _guard18;
wire _guard20 = fsm_out == 2'd2;
wire _guard21 = fsm_out == 2'd1;
wire _guard22 = block_transfer_done_out;
wire _guard23 = _guard21 & _guard22;
wire _guard24 = tdcc_go_out;
wire _guard25 = _guard23 & _guard24;
wire _guard26 = block_transfer_go_out;
wire _guard27 = bready_out;
wire _guard28 = BVALID;
wire _guard29 = _guard27 & _guard28;
wire _guard30 = ~_guard29;
wire _guard31 = block_transfer_go_out;
wire _guard32 = _guard30 & _guard31;
wire _guard33 = bready_out;
wire _guard34 = BVALID;
wire _guard35 = _guard33 & _guard34;
wire _guard36 = block_transfer_go_out;
wire _guard37 = _guard35 & _guard36;
wire _guard38 = invoke0_done_out;
wire _guard39 = ~_guard38;
wire _guard40 = fsm_out == 2'd0;
wire _guard41 = _guard39 & _guard40;
wire _guard42 = tdcc_go_out;
wire _guard43 = _guard41 & _guard42;
wire _guard44 = block_transfer_go_out;
wire _guard45 = invoke0_go_out;
wire _guard46 = _guard44 | _guard45;
wire _guard47 = bready_out;
wire _guard48 = BVALID;
wire _guard49 = _guard47 & _guard48;
wire _guard50 = block_transfer_go_out;
wire _guard51 = _guard49 & _guard50;
wire _guard52 = bready_out;
wire _guard53 = BVALID;
wire _guard54 = _guard52 & _guard53;
wire _guard55 = ~_guard54;
wire _guard56 = block_transfer_go_out;
wire _guard57 = _guard55 & _guard56;
wire _guard58 = invoke0_go_out;
wire _guard59 = _guard57 | _guard58;
wire _guard60 = fsm_out == 2'd2;
wire _guard61 = block_transfer_done_out;
wire _guard62 = ~_guard61;
wire _guard63 = fsm_out == 2'd1;
wire _guard64 = _guard62 & _guard63;
wire _guard65 = tdcc_go_out;
wire _guard66 = _guard64 & _guard65;
assign done = _guard1;
assign BREADY = bready_out;
assign fsm_write_en = _guard14;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard19 ? 2'd1 :
  _guard20 ? 2'd0 :
  _guard25 ? 2'd2 :
  2'd0;
assign block_transfer_done_in = bt_reg_out;
assign bready_write_en = _guard26;
assign bready_clk = clk;
assign bready_reset = reset;
assign bready_in =
  _guard32 ? 1'd1 :
  _guard37 ? 1'd0 :
  'x;
assign tdcc_go_in = go;
assign invoke0_go_in = _guard43;
assign invoke0_done_in = bt_reg_done;
assign bt_reg_write_en = _guard46;
assign bt_reg_clk = clk;
assign bt_reg_reset = reset;
assign bt_reg_in =
  _guard51 ? 1'd1 :
  _guard59 ? 1'd0 :
  'x;
assign tdcc_done_in = _guard60;
assign block_transfer_go_in = _guard66;
// COMPONENT END: m_bresp_channel_B0
endmodule
module address_translator_B0(
  input logic [2:0] calyx_mem_addr,
  output logic [63:0] axi_address
);
// COMPONENT START: address_translator_B0
logic [63:0] mul_B0_in;
logic [63:0] mul_B0_out;
logic [2:0] pad_input_addr_in;
logic [63:0] pad_input_addr_out;
std_const_mult # (
    .VALUE(4),
    .WIDTH(64)
) mul_B0 (
    .in(mul_B0_in),
    .out(mul_B0_out)
);
std_pad # (
    .IN_WIDTH(3),
    .OUT_WIDTH(64)
) pad_input_addr (
    .in(pad_input_addr_in),
    .out(pad_input_addr_out)
);
wire _guard0 = 1;
assign axi_address = mul_B0_out;
assign mul_B0_in = pad_input_addr_out;
assign pad_input_addr_in = calyx_mem_addr;
// COMPONENT END: address_translator_B0
endmodule
module read_controller_B0(
  input logic [63:0] axi_address,
  input logic ARESETn,
  input logic ARREADY,
  input logic RVALID,
  input logic RLAST,
  input logic [31:0] RDATA,
  input logic [1:0] RRESP,
  output logic ARVALID,
  output logic [63:0] ARADDR,
  output logic [2:0] ARSIZE,
  output logic [7:0] ARLEN,
  output logic [1:0] ARBURST,
  output logic [2:0] ARPROT,
  output logic RREADY,
  output logic [31:0] read_data,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: read_controller_B0
logic ar_channel_B0_ARESETn;
logic ar_channel_B0_ARREADY;
logic [63:0] ar_channel_B0_axi_address;
logic ar_channel_B0_ARVALID;
logic [63:0] ar_channel_B0_ARADDR;
logic [2:0] ar_channel_B0_ARSIZE;
logic [7:0] ar_channel_B0_ARLEN;
logic [1:0] ar_channel_B0_ARBURST;
logic [2:0] ar_channel_B0_ARPROT;
logic ar_channel_B0_go;
logic ar_channel_B0_clk;
logic ar_channel_B0_reset;
logic ar_channel_B0_done;
logic read_channel_B0_ARESETn;
logic read_channel_B0_RVALID;
logic read_channel_B0_RLAST;
logic [31:0] read_channel_B0_RDATA;
logic [1:0] read_channel_B0_RRESP;
logic read_channel_B0_RREADY;
logic [31:0] read_channel_B0_read_data;
logic read_channel_B0_go;
logic read_channel_B0_clk;
logic read_channel_B0_reset;
logic read_channel_B0_done;
logic [1:0] fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic [1:0] fsm_out;
logic fsm_done;
logic invoke0_go_in;
logic invoke0_go_out;
logic invoke0_done_in;
logic invoke0_done_out;
logic invoke1_go_in;
logic invoke1_go_out;
logic invoke1_done_in;
logic invoke1_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
m_ar_channel_B0 ar_channel_B0 (
    .ARADDR(ar_channel_B0_ARADDR),
    .ARBURST(ar_channel_B0_ARBURST),
    .ARESETn(ar_channel_B0_ARESETn),
    .ARLEN(ar_channel_B0_ARLEN),
    .ARPROT(ar_channel_B0_ARPROT),
    .ARREADY(ar_channel_B0_ARREADY),
    .ARSIZE(ar_channel_B0_ARSIZE),
    .ARVALID(ar_channel_B0_ARVALID),
    .axi_address(ar_channel_B0_axi_address),
    .clk(ar_channel_B0_clk),
    .done(ar_channel_B0_done),
    .go(ar_channel_B0_go),
    .reset(ar_channel_B0_reset)
);
m_read_channel_B0 read_channel_B0 (
    .ARESETn(read_channel_B0_ARESETn),
    .RDATA(read_channel_B0_RDATA),
    .RLAST(read_channel_B0_RLAST),
    .RREADY(read_channel_B0_RREADY),
    .RRESP(read_channel_B0_RRESP),
    .RVALID(read_channel_B0_RVALID),
    .clk(read_channel_B0_clk),
    .done(read_channel_B0_done),
    .go(read_channel_B0_go),
    .read_data(read_channel_B0_read_data),
    .reset(read_channel_B0_reset)
);
std_reg # (
    .WIDTH(2)
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
wire _guard1 = tdcc_done_out;
wire _guard2 = invoke0_go_out;
wire _guard3 = invoke0_go_out;
wire _guard4 = invoke1_go_out;
wire _guard5 = invoke0_go_out;
wire _guard6 = invoke0_go_out;
wire _guard7 = invoke0_go_out;
wire _guard8 = invoke0_go_out;
wire _guard9 = fsm_out == 2'd2;
wire _guard10 = fsm_out == 2'd0;
wire _guard11 = invoke0_done_out;
wire _guard12 = _guard10 & _guard11;
wire _guard13 = tdcc_go_out;
wire _guard14 = _guard12 & _guard13;
wire _guard15 = _guard9 | _guard14;
wire _guard16 = fsm_out == 2'd1;
wire _guard17 = invoke1_done_out;
wire _guard18 = _guard16 & _guard17;
wire _guard19 = tdcc_go_out;
wire _guard20 = _guard18 & _guard19;
wire _guard21 = _guard15 | _guard20;
wire _guard22 = fsm_out == 2'd0;
wire _guard23 = invoke0_done_out;
wire _guard24 = _guard22 & _guard23;
wire _guard25 = tdcc_go_out;
wire _guard26 = _guard24 & _guard25;
wire _guard27 = fsm_out == 2'd2;
wire _guard28 = fsm_out == 2'd1;
wire _guard29 = invoke1_done_out;
wire _guard30 = _guard28 & _guard29;
wire _guard31 = tdcc_go_out;
wire _guard32 = _guard30 & _guard31;
wire _guard33 = invoke1_go_out;
wire _guard34 = invoke1_go_out;
wire _guard35 = invoke1_go_out;
wire _guard36 = invoke1_go_out;
wire _guard37 = invoke1_go_out;
wire _guard38 = invoke1_go_out;
wire _guard39 = invoke0_done_out;
wire _guard40 = ~_guard39;
wire _guard41 = fsm_out == 2'd0;
wire _guard42 = _guard40 & _guard41;
wire _guard43 = tdcc_go_out;
wire _guard44 = _guard42 & _guard43;
wire _guard45 = invoke1_done_out;
wire _guard46 = ~_guard45;
wire _guard47 = fsm_out == 2'd1;
wire _guard48 = _guard46 & _guard47;
wire _guard49 = tdcc_go_out;
wire _guard50 = _guard48 & _guard49;
wire _guard51 = invoke0_go_out;
wire _guard52 = invoke0_go_out;
wire _guard53 = invoke0_go_out;
wire _guard54 = invoke0_go_out;
wire _guard55 = fsm_out == 2'd2;
assign done = _guard1;
assign ARPROT =
  _guard2 ? ar_channel_B0_ARPROT :
  3'd0;
assign ARSIZE =
  _guard3 ? ar_channel_B0_ARSIZE :
  3'd0;
assign RREADY =
  _guard4 ? read_channel_B0_RREADY :
  1'd0;
assign read_data = read_channel_B0_read_data;
assign ARLEN =
  _guard5 ? ar_channel_B0_ARLEN :
  8'd0;
assign ARADDR =
  _guard6 ? ar_channel_B0_ARADDR :
  64'd0;
assign ARBURST =
  _guard7 ? ar_channel_B0_ARBURST :
  2'd0;
assign ARVALID =
  _guard8 ? ar_channel_B0_ARVALID :
  1'd0;
assign fsm_write_en = _guard21;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard26 ? 2'd1 :
  _guard27 ? 2'd0 :
  _guard32 ? 2'd2 :
  2'd0;
assign read_channel_B0_RVALID =
  _guard33 ? RVALID :
  1'd0;
assign read_channel_B0_RLAST =
  _guard34 ? RLAST :
  1'd0;
assign read_channel_B0_RDATA =
  _guard35 ? RDATA :
  32'd0;
assign read_channel_B0_clk = clk;
assign read_channel_B0_go = _guard36;
assign read_channel_B0_reset = reset;
assign read_channel_B0_RRESP =
  _guard37 ? RRESP :
  2'd0;
assign read_channel_B0_ARESETn =
  _guard38 ? ARESETn :
  1'd0;
assign tdcc_go_in = go;
assign invoke0_go_in = _guard44;
assign invoke0_done_in = ar_channel_B0_done;
assign invoke1_go_in = _guard50;
assign ar_channel_B0_clk = clk;
assign ar_channel_B0_axi_address =
  _guard51 ? axi_address :
  64'd0;
assign ar_channel_B0_go = _guard52;
assign ar_channel_B0_reset = reset;
assign ar_channel_B0_ARREADY =
  _guard53 ? ARREADY :
  1'd0;
assign ar_channel_B0_ARESETn =
  _guard54 ? ARESETn :
  1'd0;
assign tdcc_done_in = _guard55;
assign invoke1_done_in = read_channel_B0_done;
// COMPONENT END: read_controller_B0
endmodule
module write_controller_B0(
  input logic [63:0] axi_address,
  input logic [31:0] write_data,
  input logic ARESETn,
  input logic AWREADY,
  input logic WREADY,
  input logic BVALID,
  output logic AWVALID,
  output logic [63:0] AWADDR,
  output logic [2:0] AWSIZE,
  output logic [7:0] AWLEN,
  output logic [1:0] AWBURST,
  output logic [2:0] AWPROT,
  output logic WVALID,
  output logic WLAST,
  output logic [31:0] WDATA,
  output logic BREADY,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: write_controller_B0
logic aw_channel_B0_ARESETn;
logic aw_channel_B0_AWREADY;
logic [63:0] aw_channel_B0_axi_address;
logic aw_channel_B0_AWVALID;
logic [63:0] aw_channel_B0_AWADDR;
logic [2:0] aw_channel_B0_AWSIZE;
logic [7:0] aw_channel_B0_AWLEN;
logic [1:0] aw_channel_B0_AWBURST;
logic [2:0] aw_channel_B0_AWPROT;
logic aw_channel_B0_go;
logic aw_channel_B0_clk;
logic aw_channel_B0_reset;
logic aw_channel_B0_done;
logic write_channel_B0_ARESETn;
logic write_channel_B0_WREADY;
logic [31:0] write_channel_B0_write_data;
logic write_channel_B0_WVALID;
logic write_channel_B0_WLAST;
logic [31:0] write_channel_B0_WDATA;
logic write_channel_B0_go;
logic write_channel_B0_clk;
logic write_channel_B0_reset;
logic write_channel_B0_done;
logic bresp_channel_B0_ARESETn;
logic bresp_channel_B0_BVALID;
logic bresp_channel_B0_BREADY;
logic bresp_channel_B0_go;
logic bresp_channel_B0_clk;
logic bresp_channel_B0_reset;
logic bresp_channel_B0_done;
logic [1:0] fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic [1:0] fsm_out;
logic fsm_done;
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
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
m_aw_channel_B0 aw_channel_B0 (
    .ARESETn(aw_channel_B0_ARESETn),
    .AWADDR(aw_channel_B0_AWADDR),
    .AWBURST(aw_channel_B0_AWBURST),
    .AWLEN(aw_channel_B0_AWLEN),
    .AWPROT(aw_channel_B0_AWPROT),
    .AWREADY(aw_channel_B0_AWREADY),
    .AWSIZE(aw_channel_B0_AWSIZE),
    .AWVALID(aw_channel_B0_AWVALID),
    .axi_address(aw_channel_B0_axi_address),
    .clk(aw_channel_B0_clk),
    .done(aw_channel_B0_done),
    .go(aw_channel_B0_go),
    .reset(aw_channel_B0_reset)
);
m_write_channel_B0 write_channel_B0 (
    .ARESETn(write_channel_B0_ARESETn),
    .WDATA(write_channel_B0_WDATA),
    .WLAST(write_channel_B0_WLAST),
    .WREADY(write_channel_B0_WREADY),
    .WVALID(write_channel_B0_WVALID),
    .clk(write_channel_B0_clk),
    .done(write_channel_B0_done),
    .go(write_channel_B0_go),
    .reset(write_channel_B0_reset),
    .write_data(write_channel_B0_write_data)
);
m_bresp_channel_B0 bresp_channel_B0 (
    .ARESETn(bresp_channel_B0_ARESETn),
    .BREADY(bresp_channel_B0_BREADY),
    .BVALID(bresp_channel_B0_BVALID),
    .clk(bresp_channel_B0_clk),
    .done(bresp_channel_B0_done),
    .go(bresp_channel_B0_go),
    .reset(bresp_channel_B0_reset)
);
std_reg # (
    .WIDTH(2)
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
wire _guard1 = tdcc_done_out;
wire _guard2 = invoke0_go_out;
wire _guard3 = invoke0_go_out;
wire _guard4 = invoke1_go_out;
wire _guard5 = invoke1_go_out;
wire _guard6 = invoke0_go_out;
wire _guard7 = invoke0_go_out;
wire _guard8 = invoke0_go_out;
wire _guard9 = invoke1_go_out;
wire _guard10 = invoke2_go_out;
wire _guard11 = invoke0_go_out;
wire _guard12 = fsm_out == 2'd3;
wire _guard13 = fsm_out == 2'd0;
wire _guard14 = invoke0_done_out;
wire _guard15 = _guard13 & _guard14;
wire _guard16 = tdcc_go_out;
wire _guard17 = _guard15 & _guard16;
wire _guard18 = _guard12 | _guard17;
wire _guard19 = fsm_out == 2'd1;
wire _guard20 = invoke1_done_out;
wire _guard21 = _guard19 & _guard20;
wire _guard22 = tdcc_go_out;
wire _guard23 = _guard21 & _guard22;
wire _guard24 = _guard18 | _guard23;
wire _guard25 = fsm_out == 2'd2;
wire _guard26 = invoke2_done_out;
wire _guard27 = _guard25 & _guard26;
wire _guard28 = tdcc_go_out;
wire _guard29 = _guard27 & _guard28;
wire _guard30 = _guard24 | _guard29;
wire _guard31 = fsm_out == 2'd0;
wire _guard32 = invoke0_done_out;
wire _guard33 = _guard31 & _guard32;
wire _guard34 = tdcc_go_out;
wire _guard35 = _guard33 & _guard34;
wire _guard36 = fsm_out == 2'd3;
wire _guard37 = fsm_out == 2'd2;
wire _guard38 = invoke2_done_out;
wire _guard39 = _guard37 & _guard38;
wire _guard40 = tdcc_go_out;
wire _guard41 = _guard39 & _guard40;
wire _guard42 = fsm_out == 2'd1;
wire _guard43 = invoke1_done_out;
wire _guard44 = _guard42 & _guard43;
wire _guard45 = tdcc_go_out;
wire _guard46 = _guard44 & _guard45;
wire _guard47 = invoke2_done_out;
wire _guard48 = ~_guard47;
wire _guard49 = fsm_out == 2'd2;
wire _guard50 = _guard48 & _guard49;
wire _guard51 = tdcc_go_out;
wire _guard52 = _guard50 & _guard51;
wire _guard53 = invoke1_go_out;
wire _guard54 = invoke1_go_out;
wire _guard55 = invoke1_go_out;
wire _guard56 = invoke1_go_out;
wire _guard57 = invoke0_done_out;
wire _guard58 = ~_guard57;
wire _guard59 = fsm_out == 2'd0;
wire _guard60 = _guard58 & _guard59;
wire _guard61 = tdcc_go_out;
wire _guard62 = _guard60 & _guard61;
wire _guard63 = invoke0_go_out;
wire _guard64 = invoke0_go_out;
wire _guard65 = invoke0_go_out;
wire _guard66 = invoke0_go_out;
wire _guard67 = invoke1_done_out;
wire _guard68 = ~_guard67;
wire _guard69 = fsm_out == 2'd1;
wire _guard70 = _guard68 & _guard69;
wire _guard71 = tdcc_go_out;
wire _guard72 = _guard70 & _guard71;
wire _guard73 = invoke2_go_out;
wire _guard74 = invoke2_go_out;
wire _guard75 = fsm_out == 2'd3;
assign done = _guard1;
assign AWADDR =
  _guard2 ? aw_channel_B0_AWADDR :
  64'd0;
assign AWPROT =
  _guard3 ? aw_channel_B0_AWPROT :
  3'd0;
assign WVALID =
  _guard4 ? write_channel_B0_WVALID :
  1'd0;
assign WDATA =
  _guard5 ? write_channel_B0_WDATA :
  32'd0;
assign AWSIZE =
  _guard6 ? aw_channel_B0_AWSIZE :
  3'd0;
assign AWVALID =
  _guard7 ? aw_channel_B0_AWVALID :
  1'd0;
assign AWBURST =
  _guard8 ? aw_channel_B0_AWBURST :
  2'd0;
assign WLAST =
  _guard9 ? write_channel_B0_WLAST :
  1'd0;
assign BREADY =
  _guard10 ? bresp_channel_B0_BREADY :
  1'd0;
assign AWLEN =
  _guard11 ? aw_channel_B0_AWLEN :
  8'd0;
assign fsm_write_en = _guard30;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard35 ? 2'd1 :
  _guard36 ? 2'd0 :
  _guard41 ? 2'd3 :
  _guard46 ? 2'd2 :
  2'd0;
assign invoke2_go_in = _guard52;
assign write_channel_B0_WREADY =
  _guard53 ? WREADY :
  1'd0;
assign write_channel_B0_clk = clk;
assign write_channel_B0_go = _guard54;
assign write_channel_B0_reset = reset;
assign write_channel_B0_write_data =
  _guard55 ? write_data :
  32'd0;
assign write_channel_B0_ARESETn =
  _guard56 ? ARESETn :
  1'd0;
assign tdcc_go_in = go;
assign invoke0_go_in = _guard62;
assign aw_channel_B0_clk = clk;
assign aw_channel_B0_axi_address =
  _guard63 ? axi_address :
  64'd0;
assign aw_channel_B0_AWREADY =
  _guard64 ? AWREADY :
  1'd0;
assign aw_channel_B0_go = _guard65;
assign aw_channel_B0_reset = reset;
assign aw_channel_B0_ARESETn =
  _guard66 ? ARESETn :
  1'd0;
assign invoke0_done_in = aw_channel_B0_done;
assign invoke1_go_in = _guard72;
assign invoke2_done_in = bresp_channel_B0_done;
assign bresp_channel_B0_clk = clk;
assign bresp_channel_B0_go = _guard73;
assign bresp_channel_B0_reset = reset;
assign bresp_channel_B0_BVALID =
  _guard74 ? BVALID :
  1'd0;
assign tdcc_done_in = _guard75;
assign invoke1_done_in = write_channel_B0_done;
// COMPONENT END: write_controller_B0
endmodule
module axi_dyn_mem_B0(
  input logic [2:0] addr0,
  input logic content_en,
  input logic write_en,
  input logic [31:0] write_data,
  input logic ARESETn,
  input logic ARREADY,
  input logic RVALID,
  input logic RLAST,
  input logic [31:0] RDATA,
  input logic [1:0] RRESP,
  input logic AWREADY,
  input logic WREADY,
  input logic BVALID,
  input logic [1:0] BRESP,
  output logic [31:0] read_data,
  output logic ARVALID,
  output logic [63:0] ARADDR,
  output logic [2:0] ARSIZE,
  output logic [7:0] ARLEN,
  output logic [1:0] ARBURST,
  output logic [2:0] ARPROT,
  output logic RREADY,
  output logic AWVALID,
  output logic [63:0] AWADDR,
  output logic [2:0] AWSIZE,
  output logic [7:0] AWLEN,
  output logic [1:0] AWBURST,
  output logic [2:0] AWPROT,
  output logic WVALID,
  output logic WLAST,
  output logic [31:0] WDATA,
  output logic BREADY,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: axi_dyn_mem_B0
logic [2:0] address_translator_B0_calyx_mem_addr;
logic [63:0] address_translator_B0_axi_address;
logic [63:0] read_controller_B0_axi_address;
logic read_controller_B0_ARESETn;
logic read_controller_B0_ARREADY;
logic read_controller_B0_RVALID;
logic read_controller_B0_RLAST;
logic [31:0] read_controller_B0_RDATA;
logic [1:0] read_controller_B0_RRESP;
logic read_controller_B0_ARVALID;
logic [63:0] read_controller_B0_ARADDR;
logic [2:0] read_controller_B0_ARSIZE;
logic [7:0] read_controller_B0_ARLEN;
logic [1:0] read_controller_B0_ARBURST;
logic [2:0] read_controller_B0_ARPROT;
logic read_controller_B0_RREADY;
logic [31:0] read_controller_B0_read_data;
logic read_controller_B0_go;
logic read_controller_B0_clk;
logic read_controller_B0_reset;
logic read_controller_B0_done;
logic [63:0] write_controller_B0_axi_address;
logic [31:0] write_controller_B0_write_data;
logic write_controller_B0_ARESETn;
logic write_controller_B0_AWREADY;
logic write_controller_B0_WREADY;
logic write_controller_B0_BVALID;
logic write_controller_B0_AWVALID;
logic [63:0] write_controller_B0_AWADDR;
logic [2:0] write_controller_B0_AWSIZE;
logic [7:0] write_controller_B0_AWLEN;
logic [1:0] write_controller_B0_AWBURST;
logic [2:0] write_controller_B0_AWPROT;
logic write_controller_B0_WVALID;
logic write_controller_B0_WLAST;
logic [31:0] write_controller_B0_WDATA;
logic write_controller_B0_BREADY;
logic write_controller_B0_go;
logic write_controller_B0_clk;
logic write_controller_B0_reset;
logic write_controller_B0_done;
logic [1:0] fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic [1:0] fsm_out;
logic fsm_done;
logic invoke0_go_in;
logic invoke0_go_out;
logic invoke0_done_in;
logic invoke0_done_out;
logic invoke1_go_in;
logic invoke1_go_out;
logic invoke1_done_in;
logic invoke1_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
address_translator_B0 address_translator_B0 (
    .axi_address(address_translator_B0_axi_address),
    .calyx_mem_addr(address_translator_B0_calyx_mem_addr)
);
read_controller_B0 read_controller_B0 (
    .ARADDR(read_controller_B0_ARADDR),
    .ARBURST(read_controller_B0_ARBURST),
    .ARESETn(read_controller_B0_ARESETn),
    .ARLEN(read_controller_B0_ARLEN),
    .ARPROT(read_controller_B0_ARPROT),
    .ARREADY(read_controller_B0_ARREADY),
    .ARSIZE(read_controller_B0_ARSIZE),
    .ARVALID(read_controller_B0_ARVALID),
    .RDATA(read_controller_B0_RDATA),
    .RLAST(read_controller_B0_RLAST),
    .RREADY(read_controller_B0_RREADY),
    .RRESP(read_controller_B0_RRESP),
    .RVALID(read_controller_B0_RVALID),
    .axi_address(read_controller_B0_axi_address),
    .clk(read_controller_B0_clk),
    .done(read_controller_B0_done),
    .go(read_controller_B0_go),
    .read_data(read_controller_B0_read_data),
    .reset(read_controller_B0_reset)
);
write_controller_B0 write_controller_B0 (
    .ARESETn(write_controller_B0_ARESETn),
    .AWADDR(write_controller_B0_AWADDR),
    .AWBURST(write_controller_B0_AWBURST),
    .AWLEN(write_controller_B0_AWLEN),
    .AWPROT(write_controller_B0_AWPROT),
    .AWREADY(write_controller_B0_AWREADY),
    .AWSIZE(write_controller_B0_AWSIZE),
    .AWVALID(write_controller_B0_AWVALID),
    .BREADY(write_controller_B0_BREADY),
    .BVALID(write_controller_B0_BVALID),
    .WDATA(write_controller_B0_WDATA),
    .WLAST(write_controller_B0_WLAST),
    .WREADY(write_controller_B0_WREADY),
    .WVALID(write_controller_B0_WVALID),
    .axi_address(write_controller_B0_axi_address),
    .clk(write_controller_B0_clk),
    .done(write_controller_B0_done),
    .go(write_controller_B0_go),
    .reset(write_controller_B0_reset),
    .write_data(write_controller_B0_write_data)
);
std_reg # (
    .WIDTH(2)
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
wire _guard1 = invoke0_go_out;
wire _guard2 = invoke0_go_out;
wire _guard3 = invoke0_go_out;
wire _guard4 = invoke0_go_out;
wire _guard5 = invoke0_go_out;
wire _guard6 = invoke0_go_out;
wire _guard7 = invoke0_go_out;
wire _guard8 = tdcc_done_out;
wire _guard9 = invoke1_go_out;
wire _guard10 = invoke0_go_out;
wire _guard11 = invoke0_go_out;
wire _guard12 = invoke0_go_out;
wire _guard13 = invoke0_go_out;
wire _guard14 = invoke1_go_out;
wire _guard15 = invoke1_go_out;
wire _guard16 = invoke1_go_out;
wire _guard17 = invoke0_go_out;
wire _guard18 = invoke0_go_out;
wire _guard19 = invoke1_go_out;
wire _guard20 = invoke0_go_out;
wire _guard21 = invoke0_go_out;
wire _guard22 = invoke0_go_out;
wire _guard23 = invoke0_go_out;
wire _guard24 = invoke1_go_out;
wire _guard25 = invoke1_go_out;
wire _guard26 = fsm_out == 2'd3;
wire _guard27 = fsm_out == 2'd0;
wire _guard28 = write_en;
wire _guard29 = _guard27 & _guard28;
wire _guard30 = tdcc_go_out;
wire _guard31 = _guard29 & _guard30;
wire _guard32 = _guard26 | _guard31;
wire _guard33 = fsm_out == 2'd0;
wire _guard34 = write_en;
wire _guard35 = ~_guard34;
wire _guard36 = _guard33 & _guard35;
wire _guard37 = tdcc_go_out;
wire _guard38 = _guard36 & _guard37;
wire _guard39 = _guard32 | _guard38;
wire _guard40 = fsm_out == 2'd1;
wire _guard41 = invoke0_done_out;
wire _guard42 = _guard40 & _guard41;
wire _guard43 = tdcc_go_out;
wire _guard44 = _guard42 & _guard43;
wire _guard45 = _guard39 | _guard44;
wire _guard46 = fsm_out == 2'd2;
wire _guard47 = invoke1_done_out;
wire _guard48 = _guard46 & _guard47;
wire _guard49 = tdcc_go_out;
wire _guard50 = _guard48 & _guard49;
wire _guard51 = _guard45 | _guard50;
wire _guard52 = fsm_out == 2'd0;
wire _guard53 = write_en;
wire _guard54 = _guard52 & _guard53;
wire _guard55 = tdcc_go_out;
wire _guard56 = _guard54 & _guard55;
wire _guard57 = fsm_out == 2'd3;
wire _guard58 = fsm_out == 2'd1;
wire _guard59 = invoke0_done_out;
wire _guard60 = _guard58 & _guard59;
wire _guard61 = tdcc_go_out;
wire _guard62 = _guard60 & _guard61;
wire _guard63 = fsm_out == 2'd2;
wire _guard64 = invoke1_done_out;
wire _guard65 = _guard63 & _guard64;
wire _guard66 = tdcc_go_out;
wire _guard67 = _guard65 & _guard66;
wire _guard68 = _guard62 | _guard67;
wire _guard69 = fsm_out == 2'd0;
wire _guard70 = write_en;
wire _guard71 = ~_guard70;
wire _guard72 = _guard69 & _guard71;
wire _guard73 = tdcc_go_out;
wire _guard74 = _guard72 & _guard73;
wire _guard75 = invoke0_done_out;
wire _guard76 = ~_guard75;
wire _guard77 = fsm_out == 2'd1;
wire _guard78 = _guard76 & _guard77;
wire _guard79 = tdcc_go_out;
wire _guard80 = _guard78 & _guard79;
wire _guard81 = invoke1_go_out;
wire _guard82 = invoke1_go_out;
wire _guard83 = invoke1_go_out;
wire _guard84 = invoke1_go_out;
wire _guard85 = invoke1_go_out;
wire _guard86 = invoke1_go_out;
wire _guard87 = invoke1_go_out;
wire _guard88 = invoke1_go_out;
wire _guard89 = invoke1_done_out;
wire _guard90 = ~_guard89;
wire _guard91 = fsm_out == 2'd2;
wire _guard92 = _guard90 & _guard91;
wire _guard93 = tdcc_go_out;
wire _guard94 = _guard92 & _guard93;
wire _guard95 = fsm_out == 2'd3;
assign write_controller_B0_WREADY =
  _guard1 ? WREADY :
  1'd0;
assign write_controller_B0_clk = clk;
assign write_controller_B0_axi_address =
  _guard2 ? address_translator_B0_axi_address :
  64'd0;
assign write_controller_B0_AWREADY =
  _guard3 ? AWREADY :
  1'd0;
assign write_controller_B0_go = _guard4;
assign write_controller_B0_reset = reset;
assign write_controller_B0_write_data =
  _guard5 ? write_data :
  32'd0;
assign write_controller_B0_BVALID =
  _guard6 ? BVALID :
  1'd0;
assign write_controller_B0_ARESETn =
  _guard7 ? ARESETn :
  1'd0;
assign done = _guard8;
assign ARPROT =
  _guard9 ? read_controller_B0_ARPROT :
  3'd0;
assign AWADDR =
  _guard10 ? write_controller_B0_AWADDR :
  64'd0;
assign AWPROT =
  _guard11 ? write_controller_B0_AWPROT :
  3'd0;
assign WVALID =
  _guard12 ? write_controller_B0_WVALID :
  1'd0;
assign WDATA =
  _guard13 ? write_controller_B0_WDATA :
  32'd0;
assign ARSIZE =
  _guard14 ? read_controller_B0_ARSIZE :
  3'd0;
assign RREADY =
  _guard15 ? read_controller_B0_RREADY :
  1'd0;
assign read_data = read_controller_B0_read_data;
assign ARLEN =
  _guard16 ? read_controller_B0_ARLEN :
  8'd0;
assign AWSIZE =
  _guard17 ? write_controller_B0_AWSIZE :
  3'd0;
assign AWVALID =
  _guard18 ? write_controller_B0_AWVALID :
  1'd0;
assign ARADDR =
  _guard19 ? read_controller_B0_ARADDR :
  64'd0;
assign AWBURST =
  _guard20 ? write_controller_B0_AWBURST :
  2'd0;
assign WLAST =
  _guard21 ? write_controller_B0_WLAST :
  1'd0;
assign BREADY =
  _guard22 ? write_controller_B0_BREADY :
  1'd0;
assign AWLEN =
  _guard23 ? write_controller_B0_AWLEN :
  8'd0;
assign ARBURST =
  _guard24 ? read_controller_B0_ARBURST :
  2'd0;
assign ARVALID =
  _guard25 ? read_controller_B0_ARVALID :
  1'd0;
assign fsm_write_en = _guard51;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard56 ? 2'd1 :
  _guard57 ? 2'd0 :
  _guard68 ? 2'd3 :
  _guard74 ? 2'd2 :
  2'd0;
assign address_translator_B0_calyx_mem_addr = addr0;
assign tdcc_go_in = content_en;
assign invoke0_go_in = _guard80;
assign read_controller_B0_RVALID =
  _guard81 ? RVALID :
  1'd0;
assign read_controller_B0_RLAST =
  _guard82 ? RLAST :
  1'd0;
assign read_controller_B0_RDATA =
  _guard83 ? RDATA :
  32'd0;
assign read_controller_B0_clk = clk;
assign read_controller_B0_axi_address =
  _guard84 ? address_translator_B0_axi_address :
  64'd0;
assign read_controller_B0_go = _guard85;
assign read_controller_B0_reset = reset;
assign read_controller_B0_RRESP =
  _guard86 ? RRESP :
  2'd0;
assign read_controller_B0_ARREADY =
  _guard87 ? ARREADY :
  1'd0;
assign read_controller_B0_ARESETn =
  _guard88 ? ARESETn :
  1'd0;
assign invoke0_done_in = write_controller_B0_done;
assign invoke1_go_in = _guard94;
assign tdcc_done_in = _guard95;
assign invoke1_done_in = read_controller_B0_done;
// COMPONENT END: axi_dyn_mem_B0
endmodule
module m_ar_channel_Sum0(
  input logic ARESETn,
  input logic ARREADY,
  input logic [63:0] axi_address,
  output logic ARVALID,
  output logic [63:0] ARADDR,
  output logic [2:0] ARSIZE,
  output logic [7:0] ARLEN,
  output logic [1:0] ARBURST,
  output logic [2:0] ARPROT,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: m_ar_channel_Sum0
logic arvalid_in;
logic arvalid_write_en;
logic arvalid_clk;
logic arvalid_reset;
logic arvalid_out;
logic arvalid_done;
logic ar_handshake_occurred_in;
logic ar_handshake_occurred_write_en;
logic ar_handshake_occurred_clk;
logic ar_handshake_occurred_reset;
logic ar_handshake_occurred_out;
logic ar_handshake_occurred_done;
logic bt_reg_in;
logic bt_reg_write_en;
logic bt_reg_clk;
logic bt_reg_reset;
logic bt_reg_out;
logic bt_reg_done;
logic fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic fsm_out;
logic fsm_done;
logic adder_left;
logic adder_right;
logic adder_out;
logic ud_out;
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
logic do_ar_transfer_go_in;
logic do_ar_transfer_go_out;
logic do_ar_transfer_done_in;
logic do_ar_transfer_done_out;
logic invoke2_go_in;
logic invoke2_go_out;
logic invoke2_done_in;
logic invoke2_done_out;
logic early_reset_static_par_go_in;
logic early_reset_static_par_go_out;
logic early_reset_static_par_done_in;
logic early_reset_static_par_done_out;
logic wrapper_early_reset_static_par_go_in;
logic wrapper_early_reset_static_par_go_out;
logic wrapper_early_reset_static_par_done_in;
logic wrapper_early_reset_static_par_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
std_reg # (
    .WIDTH(1)
) arvalid (
    .clk(arvalid_clk),
    .done(arvalid_done),
    .in(arvalid_in),
    .out(arvalid_out),
    .reset(arvalid_reset),
    .write_en(arvalid_write_en)
);
std_reg # (
    .WIDTH(1)
) ar_handshake_occurred (
    .clk(ar_handshake_occurred_clk),
    .done(ar_handshake_occurred_done),
    .in(ar_handshake_occurred_in),
    .out(ar_handshake_occurred_out),
    .reset(ar_handshake_occurred_reset),
    .write_en(ar_handshake_occurred_write_en)
);
std_reg # (
    .WIDTH(1)
) bt_reg (
    .clk(bt_reg_clk),
    .done(bt_reg_done),
    .in(bt_reg_in),
    .out(bt_reg_out),
    .reset(bt_reg_reset),
    .write_en(bt_reg_write_en)
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
std_add # (
    .WIDTH(1)
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
) do_ar_transfer_go (
    .in(do_ar_transfer_go_in),
    .out(do_ar_transfer_go_out)
);
std_wire # (
    .WIDTH(1)
) do_ar_transfer_done (
    .in(do_ar_transfer_done_in),
    .out(do_ar_transfer_done_out)
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
wire _guard1 = do_ar_transfer_done_out;
wire _guard2 = ~_guard1;
wire _guard3 = fsm0_out == 2'd1;
wire _guard4 = _guard2 & _guard3;
wire _guard5 = tdcc_go_out;
wire _guard6 = _guard4 & _guard5;
wire _guard7 = tdcc_done_out;
wire _guard8 = do_ar_transfer_go_out;
wire _guard9 = do_ar_transfer_go_out;
wire _guard10 = do_ar_transfer_go_out;
wire _guard11 = do_ar_transfer_go_out;
wire _guard12 = do_ar_transfer_go_out;
wire _guard13 = early_reset_static_par_go_out;
wire _guard14 = fsm_out == 1'd0;
wire _guard15 = ~_guard14;
wire _guard16 = early_reset_static_par_go_out;
wire _guard17 = _guard15 & _guard16;
wire _guard18 = fsm_out == 1'd0;
wire _guard19 = early_reset_static_par_go_out;
wire _guard20 = _guard18 & _guard19;
wire _guard21 = early_reset_static_par_go_out;
wire _guard22 = early_reset_static_par_go_out;
wire _guard23 = ar_handshake_occurred_out;
wire _guard24 = ~_guard23;
wire _guard25 = do_ar_transfer_go_out;
wire _guard26 = _guard24 & _guard25;
wire _guard27 = early_reset_static_par_go_out;
wire _guard28 = _guard26 | _guard27;
wire _guard29 = arvalid_out;
wire _guard30 = ARREADY;
wire _guard31 = _guard29 & _guard30;
wire _guard32 = do_ar_transfer_go_out;
wire _guard33 = _guard31 & _guard32;
wire _guard34 = early_reset_static_par_go_out;
wire _guard35 = invoke2_done_out;
wire _guard36 = ~_guard35;
wire _guard37 = fsm0_out == 2'd2;
wire _guard38 = _guard36 & _guard37;
wire _guard39 = tdcc_go_out;
wire _guard40 = _guard38 & _guard39;
wire _guard41 = wrapper_early_reset_static_par_done_out;
wire _guard42 = ~_guard41;
wire _guard43 = fsm0_out == 2'd0;
wire _guard44 = _guard42 & _guard43;
wire _guard45 = tdcc_go_out;
wire _guard46 = _guard44 & _guard45;
wire _guard47 = fsm_out == 1'd0;
wire _guard48 = signal_reg_out;
wire _guard49 = _guard47 & _guard48;
wire _guard50 = fsm0_out == 2'd3;
wire _guard51 = fsm0_out == 2'd0;
wire _guard52 = wrapper_early_reset_static_par_done_out;
wire _guard53 = _guard51 & _guard52;
wire _guard54 = tdcc_go_out;
wire _guard55 = _guard53 & _guard54;
wire _guard56 = _guard50 | _guard55;
wire _guard57 = fsm0_out == 2'd1;
wire _guard58 = do_ar_transfer_done_out;
wire _guard59 = _guard57 & _guard58;
wire _guard60 = tdcc_go_out;
wire _guard61 = _guard59 & _guard60;
wire _guard62 = _guard56 | _guard61;
wire _guard63 = fsm0_out == 2'd2;
wire _guard64 = invoke2_done_out;
wire _guard65 = _guard63 & _guard64;
wire _guard66 = tdcc_go_out;
wire _guard67 = _guard65 & _guard66;
wire _guard68 = _guard62 | _guard67;
wire _guard69 = fsm0_out == 2'd0;
wire _guard70 = wrapper_early_reset_static_par_done_out;
wire _guard71 = _guard69 & _guard70;
wire _guard72 = tdcc_go_out;
wire _guard73 = _guard71 & _guard72;
wire _guard74 = fsm0_out == 2'd3;
wire _guard75 = fsm0_out == 2'd2;
wire _guard76 = invoke2_done_out;
wire _guard77 = _guard75 & _guard76;
wire _guard78 = tdcc_go_out;
wire _guard79 = _guard77 & _guard78;
wire _guard80 = fsm0_out == 2'd1;
wire _guard81 = do_ar_transfer_done_out;
wire _guard82 = _guard80 & _guard81;
wire _guard83 = tdcc_go_out;
wire _guard84 = _guard82 & _guard83;
wire _guard85 = do_ar_transfer_go_out;
wire _guard86 = early_reset_static_par_go_out;
wire _guard87 = _guard85 | _guard86;
wire _guard88 = ARREADY;
wire _guard89 = arvalid_out;
wire _guard90 = _guard88 & _guard89;
wire _guard91 = do_ar_transfer_go_out;
wire _guard92 = _guard90 & _guard91;
wire _guard93 = ARREADY;
wire _guard94 = arvalid_out;
wire _guard95 = _guard93 & _guard94;
wire _guard96 = ~_guard95;
wire _guard97 = do_ar_transfer_go_out;
wire _guard98 = _guard96 & _guard97;
wire _guard99 = early_reset_static_par_go_out;
wire _guard100 = _guard98 | _guard99;
wire _guard101 = fsm_out == 1'd0;
wire _guard102 = signal_reg_out;
wire _guard103 = _guard101 & _guard102;
wire _guard104 = fsm_out == 1'd0;
wire _guard105 = signal_reg_out;
wire _guard106 = ~_guard105;
wire _guard107 = _guard104 & _guard106;
wire _guard108 = wrapper_early_reset_static_par_go_out;
wire _guard109 = _guard107 & _guard108;
wire _guard110 = _guard103 | _guard109;
wire _guard111 = fsm_out == 1'd0;
wire _guard112 = signal_reg_out;
wire _guard113 = ~_guard112;
wire _guard114 = _guard111 & _guard113;
wire _guard115 = wrapper_early_reset_static_par_go_out;
wire _guard116 = _guard114 & _guard115;
wire _guard117 = fsm_out == 1'd0;
wire _guard118 = signal_reg_out;
wire _guard119 = _guard117 & _guard118;
wire _guard120 = do_ar_transfer_go_out;
wire _guard121 = invoke2_go_out;
wire _guard122 = _guard120 | _guard121;
wire _guard123 = arvalid_out;
wire _guard124 = ARREADY;
wire _guard125 = _guard123 & _guard124;
wire _guard126 = ~_guard125;
wire _guard127 = ar_handshake_occurred_out;
wire _guard128 = ~_guard127;
wire _guard129 = _guard126 & _guard128;
wire _guard130 = do_ar_transfer_go_out;
wire _guard131 = _guard129 & _guard130;
wire _guard132 = arvalid_out;
wire _guard133 = ARREADY;
wire _guard134 = _guard132 & _guard133;
wire _guard135 = ar_handshake_occurred_out;
wire _guard136 = _guard134 | _guard135;
wire _guard137 = do_ar_transfer_go_out;
wire _guard138 = _guard136 & _guard137;
wire _guard139 = invoke2_go_out;
wire _guard140 = _guard138 | _guard139;
wire _guard141 = fsm0_out == 2'd3;
wire _guard142 = wrapper_early_reset_static_par_go_out;
assign do_ar_transfer_go_in = _guard6;
assign done = _guard7;
assign ARPROT =
  _guard8 ? 3'd6 :
  3'd0;
assign ARSIZE =
  _guard9 ? 3'd2 :
  3'd0;
assign ARLEN =
  _guard10 ? 8'd0 :
  8'd0;
assign ARADDR =
  _guard11 ? axi_address :
  64'd0;
assign ARBURST =
  _guard12 ? 2'd1 :
  2'd0;
assign ARVALID = arvalid_out;
assign fsm_write_en = _guard13;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard17 ? adder_out :
  _guard20 ? 1'd0 :
  1'd0;
assign adder_left =
  _guard21 ? fsm_out :
  1'd0;
assign adder_right = _guard22;
assign ar_handshake_occurred_write_en = _guard28;
assign ar_handshake_occurred_clk = clk;
assign ar_handshake_occurred_reset = reset;
assign ar_handshake_occurred_in =
  _guard33 ? 1'd1 :
  _guard34 ? 1'd0 :
  'x;
assign invoke2_go_in = _guard40;
assign wrapper_early_reset_static_par_go_in = _guard46;
assign wrapper_early_reset_static_par_done_in = _guard49;
assign tdcc_go_in = go;
assign fsm0_write_en = _guard68;
assign fsm0_clk = clk;
assign fsm0_reset = reset;
assign fsm0_in =
  _guard73 ? 2'd1 :
  _guard74 ? 2'd0 :
  _guard79 ? 2'd3 :
  _guard84 ? 2'd2 :
  2'd0;
assign early_reset_static_par_done_in = ud_out;
assign bt_reg_write_en = _guard87;
assign bt_reg_clk = clk;
assign bt_reg_reset = reset;
assign bt_reg_in =
  _guard92 ? 1'd1 :
  _guard100 ? 1'd0 :
  'x;
assign signal_reg_write_en = _guard110;
assign signal_reg_clk = clk;
assign signal_reg_reset = reset;
assign signal_reg_in =
  _guard116 ? 1'd1 :
  _guard119 ? 1'd0 :
  1'd0;
assign invoke2_done_in = arvalid_done;
assign arvalid_write_en = _guard122;
assign arvalid_clk = clk;
assign arvalid_reset = reset;
assign arvalid_in =
  _guard131 ? 1'd1 :
  _guard140 ? 1'd0 :
  'x;
assign tdcc_done_in = _guard141;
assign early_reset_static_par_go_in = _guard142;
assign do_ar_transfer_done_in = bt_reg_out;
// COMPONENT END: m_ar_channel_Sum0
endmodule
module m_aw_channel_Sum0(
  input logic ARESETn,
  input logic AWREADY,
  input logic [63:0] axi_address,
  output logic AWVALID,
  output logic [63:0] AWADDR,
  output logic [2:0] AWSIZE,
  output logic [7:0] AWLEN,
  output logic [1:0] AWBURST,
  output logic [2:0] AWPROT,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: m_aw_channel_Sum0
logic awvalid_in;
logic awvalid_write_en;
logic awvalid_clk;
logic awvalid_reset;
logic awvalid_out;
logic awvalid_done;
logic aw_handshake_occurred_in;
logic aw_handshake_occurred_write_en;
logic aw_handshake_occurred_clk;
logic aw_handshake_occurred_reset;
logic aw_handshake_occurred_out;
logic aw_handshake_occurred_done;
logic bt_reg_in;
logic bt_reg_write_en;
logic bt_reg_clk;
logic bt_reg_reset;
logic bt_reg_out;
logic bt_reg_done;
logic fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic fsm_out;
logic fsm_done;
logic adder_left;
logic adder_right;
logic adder_out;
logic ud_out;
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
logic do_aw_transfer_go_in;
logic do_aw_transfer_go_out;
logic do_aw_transfer_done_in;
logic do_aw_transfer_done_out;
logic invoke2_go_in;
logic invoke2_go_out;
logic invoke2_done_in;
logic invoke2_done_out;
logic early_reset_static_par_go_in;
logic early_reset_static_par_go_out;
logic early_reset_static_par_done_in;
logic early_reset_static_par_done_out;
logic wrapper_early_reset_static_par_go_in;
logic wrapper_early_reset_static_par_go_out;
logic wrapper_early_reset_static_par_done_in;
logic wrapper_early_reset_static_par_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
std_reg # (
    .WIDTH(1)
) awvalid (
    .clk(awvalid_clk),
    .done(awvalid_done),
    .in(awvalid_in),
    .out(awvalid_out),
    .reset(awvalid_reset),
    .write_en(awvalid_write_en)
);
std_reg # (
    .WIDTH(1)
) aw_handshake_occurred (
    .clk(aw_handshake_occurred_clk),
    .done(aw_handshake_occurred_done),
    .in(aw_handshake_occurred_in),
    .out(aw_handshake_occurred_out),
    .reset(aw_handshake_occurred_reset),
    .write_en(aw_handshake_occurred_write_en)
);
std_reg # (
    .WIDTH(1)
) bt_reg (
    .clk(bt_reg_clk),
    .done(bt_reg_done),
    .in(bt_reg_in),
    .out(bt_reg_out),
    .reset(bt_reg_reset),
    .write_en(bt_reg_write_en)
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
std_add # (
    .WIDTH(1)
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
) do_aw_transfer_go (
    .in(do_aw_transfer_go_in),
    .out(do_aw_transfer_go_out)
);
std_wire # (
    .WIDTH(1)
) do_aw_transfer_done (
    .in(do_aw_transfer_done_in),
    .out(do_aw_transfer_done_out)
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
wire _guard1 = tdcc_done_out;
wire _guard2 = do_aw_transfer_go_out;
wire _guard3 = do_aw_transfer_go_out;
wire _guard4 = do_aw_transfer_go_out;
wire _guard5 = do_aw_transfer_go_out;
wire _guard6 = do_aw_transfer_go_out;
wire _guard7 = early_reset_static_par_go_out;
wire _guard8 = fsm_out == 1'd0;
wire _guard9 = ~_guard8;
wire _guard10 = early_reset_static_par_go_out;
wire _guard11 = _guard9 & _guard10;
wire _guard12 = fsm_out == 1'd0;
wire _guard13 = early_reset_static_par_go_out;
wire _guard14 = _guard12 & _guard13;
wire _guard15 = early_reset_static_par_go_out;
wire _guard16 = early_reset_static_par_go_out;
wire _guard17 = invoke2_done_out;
wire _guard18 = ~_guard17;
wire _guard19 = fsm0_out == 2'd2;
wire _guard20 = _guard18 & _guard19;
wire _guard21 = tdcc_go_out;
wire _guard22 = _guard20 & _guard21;
wire _guard23 = wrapper_early_reset_static_par_done_out;
wire _guard24 = ~_guard23;
wire _guard25 = fsm0_out == 2'd0;
wire _guard26 = _guard24 & _guard25;
wire _guard27 = tdcc_go_out;
wire _guard28 = _guard26 & _guard27;
wire _guard29 = fsm_out == 1'd0;
wire _guard30 = signal_reg_out;
wire _guard31 = _guard29 & _guard30;
wire _guard32 = fsm0_out == 2'd3;
wire _guard33 = fsm0_out == 2'd0;
wire _guard34 = wrapper_early_reset_static_par_done_out;
wire _guard35 = _guard33 & _guard34;
wire _guard36 = tdcc_go_out;
wire _guard37 = _guard35 & _guard36;
wire _guard38 = _guard32 | _guard37;
wire _guard39 = fsm0_out == 2'd1;
wire _guard40 = do_aw_transfer_done_out;
wire _guard41 = _guard39 & _guard40;
wire _guard42 = tdcc_go_out;
wire _guard43 = _guard41 & _guard42;
wire _guard44 = _guard38 | _guard43;
wire _guard45 = fsm0_out == 2'd2;
wire _guard46 = invoke2_done_out;
wire _guard47 = _guard45 & _guard46;
wire _guard48 = tdcc_go_out;
wire _guard49 = _guard47 & _guard48;
wire _guard50 = _guard44 | _guard49;
wire _guard51 = fsm0_out == 2'd0;
wire _guard52 = wrapper_early_reset_static_par_done_out;
wire _guard53 = _guard51 & _guard52;
wire _guard54 = tdcc_go_out;
wire _guard55 = _guard53 & _guard54;
wire _guard56 = fsm0_out == 2'd3;
wire _guard57 = fsm0_out == 2'd2;
wire _guard58 = invoke2_done_out;
wire _guard59 = _guard57 & _guard58;
wire _guard60 = tdcc_go_out;
wire _guard61 = _guard59 & _guard60;
wire _guard62 = fsm0_out == 2'd1;
wire _guard63 = do_aw_transfer_done_out;
wire _guard64 = _guard62 & _guard63;
wire _guard65 = tdcc_go_out;
wire _guard66 = _guard64 & _guard65;
wire _guard67 = do_aw_transfer_done_out;
wire _guard68 = ~_guard67;
wire _guard69 = fsm0_out == 2'd1;
wire _guard70 = _guard68 & _guard69;
wire _guard71 = tdcc_go_out;
wire _guard72 = _guard70 & _guard71;
wire _guard73 = do_aw_transfer_go_out;
wire _guard74 = early_reset_static_par_go_out;
wire _guard75 = _guard73 | _guard74;
wire _guard76 = AWREADY;
wire _guard77 = awvalid_out;
wire _guard78 = _guard76 & _guard77;
wire _guard79 = do_aw_transfer_go_out;
wire _guard80 = _guard78 & _guard79;
wire _guard81 = AWREADY;
wire _guard82 = awvalid_out;
wire _guard83 = _guard81 & _guard82;
wire _guard84 = ~_guard83;
wire _guard85 = do_aw_transfer_go_out;
wire _guard86 = _guard84 & _guard85;
wire _guard87 = early_reset_static_par_go_out;
wire _guard88 = _guard86 | _guard87;
wire _guard89 = fsm_out == 1'd0;
wire _guard90 = signal_reg_out;
wire _guard91 = _guard89 & _guard90;
wire _guard92 = fsm_out == 1'd0;
wire _guard93 = signal_reg_out;
wire _guard94 = ~_guard93;
wire _guard95 = _guard92 & _guard94;
wire _guard96 = wrapper_early_reset_static_par_go_out;
wire _guard97 = _guard95 & _guard96;
wire _guard98 = _guard91 | _guard97;
wire _guard99 = fsm_out == 1'd0;
wire _guard100 = signal_reg_out;
wire _guard101 = ~_guard100;
wire _guard102 = _guard99 & _guard101;
wire _guard103 = wrapper_early_reset_static_par_go_out;
wire _guard104 = _guard102 & _guard103;
wire _guard105 = fsm_out == 1'd0;
wire _guard106 = signal_reg_out;
wire _guard107 = _guard105 & _guard106;
wire _guard108 = aw_handshake_occurred_out;
wire _guard109 = ~_guard108;
wire _guard110 = do_aw_transfer_go_out;
wire _guard111 = _guard109 & _guard110;
wire _guard112 = early_reset_static_par_go_out;
wire _guard113 = _guard111 | _guard112;
wire _guard114 = awvalid_out;
wire _guard115 = AWREADY;
wire _guard116 = _guard114 & _guard115;
wire _guard117 = do_aw_transfer_go_out;
wire _guard118 = _guard116 & _guard117;
wire _guard119 = early_reset_static_par_go_out;
wire _guard120 = fsm0_out == 2'd3;
wire _guard121 = do_aw_transfer_go_out;
wire _guard122 = invoke2_go_out;
wire _guard123 = _guard121 | _guard122;
wire _guard124 = awvalid_out;
wire _guard125 = AWREADY;
wire _guard126 = _guard124 & _guard125;
wire _guard127 = ~_guard126;
wire _guard128 = aw_handshake_occurred_out;
wire _guard129 = ~_guard128;
wire _guard130 = _guard127 & _guard129;
wire _guard131 = do_aw_transfer_go_out;
wire _guard132 = _guard130 & _guard131;
wire _guard133 = awvalid_out;
wire _guard134 = AWREADY;
wire _guard135 = _guard133 & _guard134;
wire _guard136 = aw_handshake_occurred_out;
wire _guard137 = _guard135 | _guard136;
wire _guard138 = do_aw_transfer_go_out;
wire _guard139 = _guard137 & _guard138;
wire _guard140 = invoke2_go_out;
wire _guard141 = _guard139 | _guard140;
wire _guard142 = wrapper_early_reset_static_par_go_out;
assign done = _guard1;
assign AWADDR =
  _guard2 ? axi_address :
  64'd0;
assign AWPROT =
  _guard3 ? 3'd6 :
  3'd0;
assign AWSIZE =
  _guard4 ? 3'd2 :
  3'd0;
assign AWVALID = awvalid_out;
assign AWBURST =
  _guard5 ? 2'd1 :
  2'd0;
assign AWLEN =
  _guard6 ? 8'd0 :
  8'd0;
assign fsm_write_en = _guard7;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard11 ? adder_out :
  _guard14 ? 1'd0 :
  1'd0;
assign adder_left =
  _guard15 ? fsm_out :
  1'd0;
assign adder_right = _guard16;
assign invoke2_go_in = _guard22;
assign wrapper_early_reset_static_par_go_in = _guard28;
assign wrapper_early_reset_static_par_done_in = _guard31;
assign tdcc_go_in = go;
assign fsm0_write_en = _guard50;
assign fsm0_clk = clk;
assign fsm0_reset = reset;
assign fsm0_in =
  _guard55 ? 2'd1 :
  _guard56 ? 2'd0 :
  _guard61 ? 2'd3 :
  _guard66 ? 2'd2 :
  2'd0;
assign do_aw_transfer_go_in = _guard72;
assign do_aw_transfer_done_in = bt_reg_out;
assign early_reset_static_par_done_in = ud_out;
assign bt_reg_write_en = _guard75;
assign bt_reg_clk = clk;
assign bt_reg_reset = reset;
assign bt_reg_in =
  _guard80 ? 1'd1 :
  _guard88 ? 1'd0 :
  'x;
assign signal_reg_write_en = _guard98;
assign signal_reg_clk = clk;
assign signal_reg_reset = reset;
assign signal_reg_in =
  _guard104 ? 1'd1 :
  _guard107 ? 1'd0 :
  1'd0;
assign invoke2_done_in = awvalid_done;
assign aw_handshake_occurred_write_en = _guard113;
assign aw_handshake_occurred_clk = clk;
assign aw_handshake_occurred_reset = reset;
assign aw_handshake_occurred_in =
  _guard118 ? 1'd1 :
  _guard119 ? 1'd0 :
  'x;
assign tdcc_done_in = _guard120;
assign awvalid_write_en = _guard123;
assign awvalid_clk = clk;
assign awvalid_reset = reset;
assign awvalid_in =
  _guard132 ? 1'd1 :
  _guard141 ? 1'd0 :
  'x;
assign early_reset_static_par_go_in = _guard142;
// COMPONENT END: m_aw_channel_Sum0
endmodule
module m_read_channel_Sum0(
  input logic ARESETn,
  input logic RVALID,
  input logic RLAST,
  input logic [31:0] RDATA,
  input logic [1:0] RRESP,
  output logic RREADY,
  output logic [31:0] read_data,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: m_read_channel_Sum0
logic [31:0] read_reg_in;
logic read_reg_write_en;
logic read_reg_clk;
logic read_reg_reset;
logic [31:0] read_reg_out;
logic read_reg_done;
logic rready_in;
logic rready_write_en;
logic rready_clk;
logic rready_reset;
logic rready_out;
logic rready_done;
logic n_RLAST_in;
logic n_RLAST_write_en;
logic n_RLAST_clk;
logic n_RLAST_reset;
logic n_RLAST_out;
logic n_RLAST_done;
logic [1:0] fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic [1:0] fsm_out;
logic fsm_done;
logic block_transfer_go_in;
logic block_transfer_go_out;
logic block_transfer_done_in;
logic block_transfer_done_out;
logic invoke0_go_in;
logic invoke0_go_out;
logic invoke0_done_in;
logic invoke0_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
std_reg # (
    .WIDTH(32)
) read_reg (
    .clk(read_reg_clk),
    .done(read_reg_done),
    .in(read_reg_in),
    .out(read_reg_out),
    .reset(read_reg_reset),
    .write_en(read_reg_write_en)
);
std_reg # (
    .WIDTH(1)
) rready (
    .clk(rready_clk),
    .done(rready_done),
    .in(rready_in),
    .out(rready_out),
    .reset(rready_reset),
    .write_en(rready_write_en)
);
std_reg # (
    .WIDTH(1)
) n_RLAST (
    .clk(n_RLAST_clk),
    .done(n_RLAST_done),
    .in(n_RLAST_in),
    .out(n_RLAST_out),
    .reset(n_RLAST_reset),
    .write_en(n_RLAST_write_en)
);
std_reg # (
    .WIDTH(2)
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
) block_transfer_go (
    .in(block_transfer_go_in),
    .out(block_transfer_go_out)
);
std_wire # (
    .WIDTH(1)
) block_transfer_done (
    .in(block_transfer_done_in),
    .out(block_transfer_done_out)
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
wire _guard1 = tdcc_done_out;
wire _guard2 = fsm_out == 2'd2;
wire _guard3 = fsm_out == 2'd0;
wire _guard4 = invoke0_done_out;
wire _guard5 = n_RLAST_out;
wire _guard6 = _guard4 & _guard5;
wire _guard7 = _guard3 & _guard6;
wire _guard8 = tdcc_go_out;
wire _guard9 = _guard7 & _guard8;
wire _guard10 = _guard2 | _guard9;
wire _guard11 = fsm_out == 2'd1;
wire _guard12 = block_transfer_done_out;
wire _guard13 = n_RLAST_out;
wire _guard14 = _guard12 & _guard13;
wire _guard15 = _guard11 & _guard14;
wire _guard16 = tdcc_go_out;
wire _guard17 = _guard15 & _guard16;
wire _guard18 = _guard10 | _guard17;
wire _guard19 = fsm_out == 2'd0;
wire _guard20 = invoke0_done_out;
wire _guard21 = n_RLAST_out;
wire _guard22 = ~_guard21;
wire _guard23 = _guard20 & _guard22;
wire _guard24 = _guard19 & _guard23;
wire _guard25 = tdcc_go_out;
wire _guard26 = _guard24 & _guard25;
wire _guard27 = _guard18 | _guard26;
wire _guard28 = fsm_out == 2'd1;
wire _guard29 = block_transfer_done_out;
wire _guard30 = n_RLAST_out;
wire _guard31 = ~_guard30;
wire _guard32 = _guard29 & _guard31;
wire _guard33 = _guard28 & _guard32;
wire _guard34 = tdcc_go_out;
wire _guard35 = _guard33 & _guard34;
wire _guard36 = _guard27 | _guard35;
wire _guard37 = fsm_out == 2'd0;
wire _guard38 = invoke0_done_out;
wire _guard39 = n_RLAST_out;
wire _guard40 = _guard38 & _guard39;
wire _guard41 = _guard37 & _guard40;
wire _guard42 = tdcc_go_out;
wire _guard43 = _guard41 & _guard42;
wire _guard44 = fsm_out == 2'd1;
wire _guard45 = block_transfer_done_out;
wire _guard46 = n_RLAST_out;
wire _guard47 = _guard45 & _guard46;
wire _guard48 = _guard44 & _guard47;
wire _guard49 = tdcc_go_out;
wire _guard50 = _guard48 & _guard49;
wire _guard51 = _guard43 | _guard50;
wire _guard52 = fsm_out == 2'd2;
wire _guard53 = fsm_out == 2'd0;
wire _guard54 = invoke0_done_out;
wire _guard55 = n_RLAST_out;
wire _guard56 = ~_guard55;
wire _guard57 = _guard54 & _guard56;
wire _guard58 = _guard53 & _guard57;
wire _guard59 = tdcc_go_out;
wire _guard60 = _guard58 & _guard59;
wire _guard61 = fsm_out == 2'd1;
wire _guard62 = block_transfer_done_out;
wire _guard63 = n_RLAST_out;
wire _guard64 = ~_guard63;
wire _guard65 = _guard62 & _guard64;
wire _guard66 = _guard61 & _guard65;
wire _guard67 = tdcc_go_out;
wire _guard68 = _guard66 & _guard67;
wire _guard69 = _guard60 | _guard68;
wire _guard70 = rready_out;
wire _guard71 = RVALID;
wire _guard72 = _guard70 & _guard71;
wire _guard73 = block_transfer_go_out;
wire _guard74 = _guard72 & _guard73;
wire _guard75 = rready_out;
wire _guard76 = RVALID;
wire _guard77 = _guard75 & _guard76;
wire _guard78 = ~_guard77;
wire _guard79 = block_transfer_go_out;
wire _guard80 = _guard78 & _guard79;
wire _guard81 = block_transfer_go_out;
wire _guard82 = invoke0_done_out;
wire _guard83 = ~_guard82;
wire _guard84 = fsm_out == 2'd0;
wire _guard85 = _guard83 & _guard84;
wire _guard86 = tdcc_go_out;
wire _guard87 = _guard85 & _guard86;
wire _guard88 = block_transfer_go_out;
wire _guard89 = invoke0_go_out;
wire _guard90 = _guard88 | _guard89;
wire _guard91 = RLAST;
wire _guard92 = ~_guard91;
wire _guard93 = block_transfer_go_out;
wire _guard94 = _guard92 & _guard93;
wire _guard95 = invoke0_go_out;
wire _guard96 = _guard94 | _guard95;
wire _guard97 = RLAST;
wire _guard98 = block_transfer_go_out;
wire _guard99 = _guard97 & _guard98;
wire _guard100 = fsm_out == 2'd2;
wire _guard101 = block_transfer_done_out;
wire _guard102 = ~_guard101;
wire _guard103 = fsm_out == 2'd1;
wire _guard104 = _guard102 & _guard103;
wire _guard105 = tdcc_go_out;
wire _guard106 = _guard104 & _guard105;
wire _guard107 = block_transfer_go_out;
wire _guard108 = rready_out;
wire _guard109 = RVALID;
wire _guard110 = _guard108 & _guard109;
wire _guard111 = ~_guard110;
wire _guard112 = block_transfer_go_out;
wire _guard113 = _guard111 & _guard112;
wire _guard114 = rready_out;
wire _guard115 = RVALID;
wire _guard116 = _guard114 & _guard115;
wire _guard117 = block_transfer_go_out;
wire _guard118 = _guard116 & _guard117;
assign done = _guard1;
assign RREADY = rready_out;
assign read_data = read_reg_out;
assign fsm_write_en = _guard36;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard51 ? 2'd1 :
  _guard52 ? 2'd0 :
  _guard69 ? 2'd2 :
  2'd0;
assign block_transfer_done_in = read_reg_done;
assign read_reg_write_en =
  _guard74 ? 1'd1 :
  _guard80 ? 1'd0 :
  1'd0;
assign read_reg_clk = clk;
assign read_reg_reset = reset;
assign read_reg_in = RDATA;
assign tdcc_go_in = go;
assign invoke0_go_in = _guard87;
assign n_RLAST_write_en = _guard90;
assign n_RLAST_clk = clk;
assign n_RLAST_reset = reset;
assign n_RLAST_in =
  _guard96 ? 1'd1 :
  _guard99 ? 1'd0 :
  'x;
assign invoke0_done_in = n_RLAST_done;
assign tdcc_done_in = _guard100;
assign block_transfer_go_in = _guard106;
assign rready_write_en = _guard107;
assign rready_clk = clk;
assign rready_reset = reset;
assign rready_in =
  _guard113 ? 1'd1 :
  _guard118 ? 1'd0 :
  'x;
// COMPONENT END: m_read_channel_Sum0
endmodule
module m_write_channel_Sum0(
  input logic ARESETn,
  input logic WREADY,
  input logic [31:0] write_data,
  output logic WVALID,
  output logic WLAST,
  output logic [31:0] WDATA,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: m_write_channel_Sum0
logic wvalid_in;
logic wvalid_write_en;
logic wvalid_clk;
logic wvalid_reset;
logic wvalid_out;
logic wvalid_done;
logic w_handshake_occurred_in;
logic w_handshake_occurred_write_en;
logic w_handshake_occurred_clk;
logic w_handshake_occurred_reset;
logic w_handshake_occurred_out;
logic w_handshake_occurred_done;
logic bt_reg_in;
logic bt_reg_write_en;
logic bt_reg_clk;
logic bt_reg_reset;
logic bt_reg_out;
logic bt_reg_done;
logic fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic fsm_out;
logic fsm_done;
logic adder_left;
logic adder_right;
logic adder_out;
logic ud_out;
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
logic service_write_transfer_go_in;
logic service_write_transfer_go_out;
logic service_write_transfer_done_in;
logic service_write_transfer_done_out;
logic early_reset_static_par_go_in;
logic early_reset_static_par_go_out;
logic early_reset_static_par_done_in;
logic early_reset_static_par_done_out;
logic wrapper_early_reset_static_par_go_in;
logic wrapper_early_reset_static_par_go_out;
logic wrapper_early_reset_static_par_done_in;
logic wrapper_early_reset_static_par_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
std_reg # (
    .WIDTH(1)
) wvalid (
    .clk(wvalid_clk),
    .done(wvalid_done),
    .in(wvalid_in),
    .out(wvalid_out),
    .reset(wvalid_reset),
    .write_en(wvalid_write_en)
);
std_reg # (
    .WIDTH(1)
) w_handshake_occurred (
    .clk(w_handshake_occurred_clk),
    .done(w_handshake_occurred_done),
    .in(w_handshake_occurred_in),
    .out(w_handshake_occurred_out),
    .reset(w_handshake_occurred_reset),
    .write_en(w_handshake_occurred_write_en)
);
std_reg # (
    .WIDTH(1)
) bt_reg (
    .clk(bt_reg_clk),
    .done(bt_reg_done),
    .in(bt_reg_in),
    .out(bt_reg_out),
    .reset(bt_reg_reset),
    .write_en(bt_reg_write_en)
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
std_add # (
    .WIDTH(1)
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
) service_write_transfer_go (
    .in(service_write_transfer_go_in),
    .out(service_write_transfer_go_out)
);
std_wire # (
    .WIDTH(1)
) service_write_transfer_done (
    .in(service_write_transfer_done_in),
    .out(service_write_transfer_done_out)
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
wire _guard1 = tdcc_done_out;
wire _guard2 = service_write_transfer_go_out;
wire _guard3 = service_write_transfer_go_out;
wire _guard4 = early_reset_static_par_go_out;
wire _guard5 = fsm_out == 1'd0;
wire _guard6 = ~_guard5;
wire _guard7 = early_reset_static_par_go_out;
wire _guard8 = _guard6 & _guard7;
wire _guard9 = fsm_out == 1'd0;
wire _guard10 = early_reset_static_par_go_out;
wire _guard11 = _guard9 & _guard10;
wire _guard12 = early_reset_static_par_go_out;
wire _guard13 = early_reset_static_par_go_out;
wire _guard14 = service_write_transfer_go_out;
wire _guard15 = wvalid_out;
wire _guard16 = WREADY;
wire _guard17 = _guard15 & _guard16;
wire _guard18 = ~_guard17;
wire _guard19 = w_handshake_occurred_out;
wire _guard20 = ~_guard19;
wire _guard21 = _guard18 & _guard20;
wire _guard22 = service_write_transfer_go_out;
wire _guard23 = _guard21 & _guard22;
wire _guard24 = wvalid_out;
wire _guard25 = WREADY;
wire _guard26 = _guard24 & _guard25;
wire _guard27 = w_handshake_occurred_out;
wire _guard28 = _guard26 | _guard27;
wire _guard29 = service_write_transfer_go_out;
wire _guard30 = _guard28 & _guard29;
wire _guard31 = wrapper_early_reset_static_par_done_out;
wire _guard32 = ~_guard31;
wire _guard33 = fsm0_out == 2'd0;
wire _guard34 = _guard32 & _guard33;
wire _guard35 = tdcc_go_out;
wire _guard36 = _guard34 & _guard35;
wire _guard37 = fsm_out == 1'd0;
wire _guard38 = signal_reg_out;
wire _guard39 = _guard37 & _guard38;
wire _guard40 = fsm0_out == 2'd2;
wire _guard41 = fsm0_out == 2'd0;
wire _guard42 = wrapper_early_reset_static_par_done_out;
wire _guard43 = _guard41 & _guard42;
wire _guard44 = tdcc_go_out;
wire _guard45 = _guard43 & _guard44;
wire _guard46 = _guard40 | _guard45;
wire _guard47 = fsm0_out == 2'd1;
wire _guard48 = service_write_transfer_done_out;
wire _guard49 = _guard47 & _guard48;
wire _guard50 = tdcc_go_out;
wire _guard51 = _guard49 & _guard50;
wire _guard52 = _guard46 | _guard51;
wire _guard53 = fsm0_out == 2'd0;
wire _guard54 = wrapper_early_reset_static_par_done_out;
wire _guard55 = _guard53 & _guard54;
wire _guard56 = tdcc_go_out;
wire _guard57 = _guard55 & _guard56;
wire _guard58 = fsm0_out == 2'd2;
wire _guard59 = fsm0_out == 2'd1;
wire _guard60 = service_write_transfer_done_out;
wire _guard61 = _guard59 & _guard60;
wire _guard62 = tdcc_go_out;
wire _guard63 = _guard61 & _guard62;
wire _guard64 = service_write_transfer_done_out;
wire _guard65 = ~_guard64;
wire _guard66 = fsm0_out == 2'd1;
wire _guard67 = _guard65 & _guard66;
wire _guard68 = tdcc_go_out;
wire _guard69 = _guard67 & _guard68;
wire _guard70 = service_write_transfer_go_out;
wire _guard71 = early_reset_static_par_go_out;
wire _guard72 = _guard70 | _guard71;
wire _guard73 = wvalid_out;
wire _guard74 = WREADY;
wire _guard75 = _guard73 & _guard74;
wire _guard76 = service_write_transfer_go_out;
wire _guard77 = _guard75 & _guard76;
wire _guard78 = wvalid_out;
wire _guard79 = WREADY;
wire _guard80 = _guard78 & _guard79;
wire _guard81 = ~_guard80;
wire _guard82 = service_write_transfer_go_out;
wire _guard83 = _guard81 & _guard82;
wire _guard84 = early_reset_static_par_go_out;
wire _guard85 = _guard83 | _guard84;
wire _guard86 = fsm_out == 1'd0;
wire _guard87 = signal_reg_out;
wire _guard88 = _guard86 & _guard87;
wire _guard89 = fsm_out == 1'd0;
wire _guard90 = signal_reg_out;
wire _guard91 = ~_guard90;
wire _guard92 = _guard89 & _guard91;
wire _guard93 = wrapper_early_reset_static_par_go_out;
wire _guard94 = _guard92 & _guard93;
wire _guard95 = _guard88 | _guard94;
wire _guard96 = fsm_out == 1'd0;
wire _guard97 = signal_reg_out;
wire _guard98 = ~_guard97;
wire _guard99 = _guard96 & _guard98;
wire _guard100 = wrapper_early_reset_static_par_go_out;
wire _guard101 = _guard99 & _guard100;
wire _guard102 = fsm_out == 1'd0;
wire _guard103 = signal_reg_out;
wire _guard104 = _guard102 & _guard103;
wire _guard105 = w_handshake_occurred_out;
wire _guard106 = ~_guard105;
wire _guard107 = service_write_transfer_go_out;
wire _guard108 = _guard106 & _guard107;
wire _guard109 = early_reset_static_par_go_out;
wire _guard110 = _guard108 | _guard109;
wire _guard111 = wvalid_out;
wire _guard112 = WREADY;
wire _guard113 = _guard111 & _guard112;
wire _guard114 = service_write_transfer_go_out;
wire _guard115 = _guard113 & _guard114;
wire _guard116 = wvalid_out;
wire _guard117 = WREADY;
wire _guard118 = _guard116 & _guard117;
wire _guard119 = ~_guard118;
wire _guard120 = service_write_transfer_go_out;
wire _guard121 = _guard119 & _guard120;
wire _guard122 = early_reset_static_par_go_out;
wire _guard123 = _guard121 | _guard122;
wire _guard124 = fsm0_out == 2'd2;
wire _guard125 = wrapper_early_reset_static_par_go_out;
assign done = _guard1;
assign WVALID = wvalid_out;
assign WDATA =
  _guard2 ? write_data :
  32'd0;
assign WLAST = _guard3;
assign fsm_write_en = _guard4;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard8 ? adder_out :
  _guard11 ? 1'd0 :
  1'd0;
assign adder_left =
  _guard12 ? fsm_out :
  1'd0;
assign adder_right = _guard13;
assign wvalid_write_en = _guard14;
assign wvalid_clk = clk;
assign wvalid_reset = reset;
assign wvalid_in =
  _guard23 ? 1'd1 :
  _guard30 ? 1'd0 :
  'x;
assign wrapper_early_reset_static_par_go_in = _guard36;
assign wrapper_early_reset_static_par_done_in = _guard39;
assign tdcc_go_in = go;
assign service_write_transfer_done_in = bt_reg_out;
assign fsm0_write_en = _guard52;
assign fsm0_clk = clk;
assign fsm0_reset = reset;
assign fsm0_in =
  _guard57 ? 2'd1 :
  _guard58 ? 2'd0 :
  _guard63 ? 2'd2 :
  2'd0;
assign service_write_transfer_go_in = _guard69;
assign early_reset_static_par_done_in = ud_out;
assign bt_reg_write_en = _guard72;
assign bt_reg_clk = clk;
assign bt_reg_reset = reset;
assign bt_reg_in =
  _guard77 ? 1'd1 :
  _guard85 ? 1'd0 :
  'x;
assign signal_reg_write_en = _guard95;
assign signal_reg_clk = clk;
assign signal_reg_reset = reset;
assign signal_reg_in =
  _guard101 ? 1'd1 :
  _guard104 ? 1'd0 :
  1'd0;
assign w_handshake_occurred_write_en = _guard110;
assign w_handshake_occurred_clk = clk;
assign w_handshake_occurred_reset = reset;
assign w_handshake_occurred_in =
  _guard115 ? 1'd1 :
  _guard123 ? 1'd0 :
  'x;
assign tdcc_done_in = _guard124;
assign early_reset_static_par_go_in = _guard125;
// COMPONENT END: m_write_channel_Sum0
endmodule
module m_bresp_channel_Sum0(
  input logic ARESETn,
  input logic BVALID,
  output logic BREADY,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: m_bresp_channel_Sum0
logic bready_in;
logic bready_write_en;
logic bready_clk;
logic bready_reset;
logic bready_out;
logic bready_done;
logic bt_reg_in;
logic bt_reg_write_en;
logic bt_reg_clk;
logic bt_reg_reset;
logic bt_reg_out;
logic bt_reg_done;
logic [1:0] fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic [1:0] fsm_out;
logic fsm_done;
logic block_transfer_go_in;
logic block_transfer_go_out;
logic block_transfer_done_in;
logic block_transfer_done_out;
logic invoke0_go_in;
logic invoke0_go_out;
logic invoke0_done_in;
logic invoke0_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
std_reg # (
    .WIDTH(1)
) bready (
    .clk(bready_clk),
    .done(bready_done),
    .in(bready_in),
    .out(bready_out),
    .reset(bready_reset),
    .write_en(bready_write_en)
);
std_reg # (
    .WIDTH(1)
) bt_reg (
    .clk(bt_reg_clk),
    .done(bt_reg_done),
    .in(bt_reg_in),
    .out(bt_reg_out),
    .reset(bt_reg_reset),
    .write_en(bt_reg_write_en)
);
std_reg # (
    .WIDTH(2)
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
) block_transfer_go (
    .in(block_transfer_go_in),
    .out(block_transfer_go_out)
);
std_wire # (
    .WIDTH(1)
) block_transfer_done (
    .in(block_transfer_done_in),
    .out(block_transfer_done_out)
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
wire _guard1 = tdcc_done_out;
wire _guard2 = fsm_out == 2'd2;
wire _guard3 = fsm_out == 2'd0;
wire _guard4 = invoke0_done_out;
wire _guard5 = _guard3 & _guard4;
wire _guard6 = tdcc_go_out;
wire _guard7 = _guard5 & _guard6;
wire _guard8 = _guard2 | _guard7;
wire _guard9 = fsm_out == 2'd1;
wire _guard10 = block_transfer_done_out;
wire _guard11 = _guard9 & _guard10;
wire _guard12 = tdcc_go_out;
wire _guard13 = _guard11 & _guard12;
wire _guard14 = _guard8 | _guard13;
wire _guard15 = fsm_out == 2'd0;
wire _guard16 = invoke0_done_out;
wire _guard17 = _guard15 & _guard16;
wire _guard18 = tdcc_go_out;
wire _guard19 = _guard17 & _guard18;
wire _guard20 = fsm_out == 2'd2;
wire _guard21 = fsm_out == 2'd1;
wire _guard22 = block_transfer_done_out;
wire _guard23 = _guard21 & _guard22;
wire _guard24 = tdcc_go_out;
wire _guard25 = _guard23 & _guard24;
wire _guard26 = block_transfer_go_out;
wire _guard27 = bready_out;
wire _guard28 = BVALID;
wire _guard29 = _guard27 & _guard28;
wire _guard30 = ~_guard29;
wire _guard31 = block_transfer_go_out;
wire _guard32 = _guard30 & _guard31;
wire _guard33 = bready_out;
wire _guard34 = BVALID;
wire _guard35 = _guard33 & _guard34;
wire _guard36 = block_transfer_go_out;
wire _guard37 = _guard35 & _guard36;
wire _guard38 = invoke0_done_out;
wire _guard39 = ~_guard38;
wire _guard40 = fsm_out == 2'd0;
wire _guard41 = _guard39 & _guard40;
wire _guard42 = tdcc_go_out;
wire _guard43 = _guard41 & _guard42;
wire _guard44 = block_transfer_go_out;
wire _guard45 = invoke0_go_out;
wire _guard46 = _guard44 | _guard45;
wire _guard47 = bready_out;
wire _guard48 = BVALID;
wire _guard49 = _guard47 & _guard48;
wire _guard50 = block_transfer_go_out;
wire _guard51 = _guard49 & _guard50;
wire _guard52 = bready_out;
wire _guard53 = BVALID;
wire _guard54 = _guard52 & _guard53;
wire _guard55 = ~_guard54;
wire _guard56 = block_transfer_go_out;
wire _guard57 = _guard55 & _guard56;
wire _guard58 = invoke0_go_out;
wire _guard59 = _guard57 | _guard58;
wire _guard60 = fsm_out == 2'd2;
wire _guard61 = block_transfer_done_out;
wire _guard62 = ~_guard61;
wire _guard63 = fsm_out == 2'd1;
wire _guard64 = _guard62 & _guard63;
wire _guard65 = tdcc_go_out;
wire _guard66 = _guard64 & _guard65;
assign done = _guard1;
assign BREADY = bready_out;
assign fsm_write_en = _guard14;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard19 ? 2'd1 :
  _guard20 ? 2'd0 :
  _guard25 ? 2'd2 :
  2'd0;
assign block_transfer_done_in = bt_reg_out;
assign bready_write_en = _guard26;
assign bready_clk = clk;
assign bready_reset = reset;
assign bready_in =
  _guard32 ? 1'd1 :
  _guard37 ? 1'd0 :
  'x;
assign tdcc_go_in = go;
assign invoke0_go_in = _guard43;
assign invoke0_done_in = bt_reg_done;
assign bt_reg_write_en = _guard46;
assign bt_reg_clk = clk;
assign bt_reg_reset = reset;
assign bt_reg_in =
  _guard51 ? 1'd1 :
  _guard59 ? 1'd0 :
  'x;
assign tdcc_done_in = _guard60;
assign block_transfer_go_in = _guard66;
// COMPONENT END: m_bresp_channel_Sum0
endmodule
module address_translator_Sum0(
  input logic [2:0] calyx_mem_addr,
  output logic [63:0] axi_address
);
// COMPONENT START: address_translator_Sum0
logic [63:0] mul_Sum0_in;
logic [63:0] mul_Sum0_out;
logic [2:0] pad_input_addr_in;
logic [63:0] pad_input_addr_out;
std_const_mult # (
    .VALUE(4),
    .WIDTH(64)
) mul_Sum0 (
    .in(mul_Sum0_in),
    .out(mul_Sum0_out)
);
std_pad # (
    .IN_WIDTH(3),
    .OUT_WIDTH(64)
) pad_input_addr (
    .in(pad_input_addr_in),
    .out(pad_input_addr_out)
);
wire _guard0 = 1;
assign axi_address = mul_Sum0_out;
assign mul_Sum0_in = pad_input_addr_out;
assign pad_input_addr_in = calyx_mem_addr;
// COMPONENT END: address_translator_Sum0
endmodule
module read_controller_Sum0(
  input logic [63:0] axi_address,
  input logic ARESETn,
  input logic ARREADY,
  input logic RVALID,
  input logic RLAST,
  input logic [31:0] RDATA,
  input logic [1:0] RRESP,
  output logic ARVALID,
  output logic [63:0] ARADDR,
  output logic [2:0] ARSIZE,
  output logic [7:0] ARLEN,
  output logic [1:0] ARBURST,
  output logic [2:0] ARPROT,
  output logic RREADY,
  output logic [31:0] read_data,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: read_controller_Sum0
logic ar_channel_Sum0_ARESETn;
logic ar_channel_Sum0_ARREADY;
logic [63:0] ar_channel_Sum0_axi_address;
logic ar_channel_Sum0_ARVALID;
logic [63:0] ar_channel_Sum0_ARADDR;
logic [2:0] ar_channel_Sum0_ARSIZE;
logic [7:0] ar_channel_Sum0_ARLEN;
logic [1:0] ar_channel_Sum0_ARBURST;
logic [2:0] ar_channel_Sum0_ARPROT;
logic ar_channel_Sum0_go;
logic ar_channel_Sum0_clk;
logic ar_channel_Sum0_reset;
logic ar_channel_Sum0_done;
logic read_channel_Sum0_ARESETn;
logic read_channel_Sum0_RVALID;
logic read_channel_Sum0_RLAST;
logic [31:0] read_channel_Sum0_RDATA;
logic [1:0] read_channel_Sum0_RRESP;
logic read_channel_Sum0_RREADY;
logic [31:0] read_channel_Sum0_read_data;
logic read_channel_Sum0_go;
logic read_channel_Sum0_clk;
logic read_channel_Sum0_reset;
logic read_channel_Sum0_done;
logic [1:0] fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic [1:0] fsm_out;
logic fsm_done;
logic invoke0_go_in;
logic invoke0_go_out;
logic invoke0_done_in;
logic invoke0_done_out;
logic invoke1_go_in;
logic invoke1_go_out;
logic invoke1_done_in;
logic invoke1_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
m_ar_channel_Sum0 ar_channel_Sum0 (
    .ARADDR(ar_channel_Sum0_ARADDR),
    .ARBURST(ar_channel_Sum0_ARBURST),
    .ARESETn(ar_channel_Sum0_ARESETn),
    .ARLEN(ar_channel_Sum0_ARLEN),
    .ARPROT(ar_channel_Sum0_ARPROT),
    .ARREADY(ar_channel_Sum0_ARREADY),
    .ARSIZE(ar_channel_Sum0_ARSIZE),
    .ARVALID(ar_channel_Sum0_ARVALID),
    .axi_address(ar_channel_Sum0_axi_address),
    .clk(ar_channel_Sum0_clk),
    .done(ar_channel_Sum0_done),
    .go(ar_channel_Sum0_go),
    .reset(ar_channel_Sum0_reset)
);
m_read_channel_Sum0 read_channel_Sum0 (
    .ARESETn(read_channel_Sum0_ARESETn),
    .RDATA(read_channel_Sum0_RDATA),
    .RLAST(read_channel_Sum0_RLAST),
    .RREADY(read_channel_Sum0_RREADY),
    .RRESP(read_channel_Sum0_RRESP),
    .RVALID(read_channel_Sum0_RVALID),
    .clk(read_channel_Sum0_clk),
    .done(read_channel_Sum0_done),
    .go(read_channel_Sum0_go),
    .read_data(read_channel_Sum0_read_data),
    .reset(read_channel_Sum0_reset)
);
std_reg # (
    .WIDTH(2)
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
wire _guard1 = invoke1_go_out;
wire _guard2 = invoke1_go_out;
wire _guard3 = invoke1_go_out;
wire _guard4 = invoke1_go_out;
wire _guard5 = invoke1_go_out;
wire _guard6 = invoke1_go_out;
wire _guard7 = tdcc_done_out;
wire _guard8 = invoke0_go_out;
wire _guard9 = invoke0_go_out;
wire _guard10 = invoke1_go_out;
wire _guard11 = invoke0_go_out;
wire _guard12 = invoke0_go_out;
wire _guard13 = invoke0_go_out;
wire _guard14 = invoke0_go_out;
wire _guard15 = fsm_out == 2'd2;
wire _guard16 = fsm_out == 2'd0;
wire _guard17 = invoke0_done_out;
wire _guard18 = _guard16 & _guard17;
wire _guard19 = tdcc_go_out;
wire _guard20 = _guard18 & _guard19;
wire _guard21 = _guard15 | _guard20;
wire _guard22 = fsm_out == 2'd1;
wire _guard23 = invoke1_done_out;
wire _guard24 = _guard22 & _guard23;
wire _guard25 = tdcc_go_out;
wire _guard26 = _guard24 & _guard25;
wire _guard27 = _guard21 | _guard26;
wire _guard28 = fsm_out == 2'd0;
wire _guard29 = invoke0_done_out;
wire _guard30 = _guard28 & _guard29;
wire _guard31 = tdcc_go_out;
wire _guard32 = _guard30 & _guard31;
wire _guard33 = fsm_out == 2'd2;
wire _guard34 = fsm_out == 2'd1;
wire _guard35 = invoke1_done_out;
wire _guard36 = _guard34 & _guard35;
wire _guard37 = tdcc_go_out;
wire _guard38 = _guard36 & _guard37;
wire _guard39 = invoke0_go_out;
wire _guard40 = invoke0_go_out;
wire _guard41 = invoke0_go_out;
wire _guard42 = invoke0_go_out;
wire _guard43 = invoke0_done_out;
wire _guard44 = ~_guard43;
wire _guard45 = fsm_out == 2'd0;
wire _guard46 = _guard44 & _guard45;
wire _guard47 = tdcc_go_out;
wire _guard48 = _guard46 & _guard47;
wire _guard49 = invoke1_done_out;
wire _guard50 = ~_guard49;
wire _guard51 = fsm_out == 2'd1;
wire _guard52 = _guard50 & _guard51;
wire _guard53 = tdcc_go_out;
wire _guard54 = _guard52 & _guard53;
wire _guard55 = fsm_out == 2'd2;
assign read_channel_Sum0_RVALID =
  _guard1 ? RVALID :
  1'd0;
assign read_channel_Sum0_RLAST =
  _guard2 ? RLAST :
  1'd0;
assign read_channel_Sum0_RDATA =
  _guard3 ? RDATA :
  32'd0;
assign read_channel_Sum0_clk = clk;
assign read_channel_Sum0_go = _guard4;
assign read_channel_Sum0_reset = reset;
assign read_channel_Sum0_RRESP =
  _guard5 ? RRESP :
  2'd0;
assign read_channel_Sum0_ARESETn =
  _guard6 ? ARESETn :
  1'd0;
assign done = _guard7;
assign ARPROT =
  _guard8 ? ar_channel_Sum0_ARPROT :
  3'd0;
assign ARSIZE =
  _guard9 ? ar_channel_Sum0_ARSIZE :
  3'd0;
assign RREADY =
  _guard10 ? read_channel_Sum0_RREADY :
  1'd0;
assign read_data = read_channel_Sum0_read_data;
assign ARLEN =
  _guard11 ? ar_channel_Sum0_ARLEN :
  8'd0;
assign ARADDR =
  _guard12 ? ar_channel_Sum0_ARADDR :
  64'd0;
assign ARBURST =
  _guard13 ? ar_channel_Sum0_ARBURST :
  2'd0;
assign ARVALID =
  _guard14 ? ar_channel_Sum0_ARVALID :
  1'd0;
assign fsm_write_en = _guard27;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard32 ? 2'd1 :
  _guard33 ? 2'd0 :
  _guard38 ? 2'd2 :
  2'd0;
assign ar_channel_Sum0_clk = clk;
assign ar_channel_Sum0_axi_address =
  _guard39 ? axi_address :
  64'd0;
assign ar_channel_Sum0_go = _guard40;
assign ar_channel_Sum0_reset = reset;
assign ar_channel_Sum0_ARREADY =
  _guard41 ? ARREADY :
  1'd0;
assign ar_channel_Sum0_ARESETn =
  _guard42 ? ARESETn :
  1'd0;
assign tdcc_go_in = go;
assign invoke0_go_in = _guard48;
assign invoke0_done_in = ar_channel_Sum0_done;
assign invoke1_go_in = _guard54;
assign tdcc_done_in = _guard55;
assign invoke1_done_in = read_channel_Sum0_done;
// COMPONENT END: read_controller_Sum0
endmodule
module write_controller_Sum0(
  input logic [63:0] axi_address,
  input logic [31:0] write_data,
  input logic ARESETn,
  input logic AWREADY,
  input logic WREADY,
  input logic BVALID,
  output logic AWVALID,
  output logic [63:0] AWADDR,
  output logic [2:0] AWSIZE,
  output logic [7:0] AWLEN,
  output logic [1:0] AWBURST,
  output logic [2:0] AWPROT,
  output logic WVALID,
  output logic WLAST,
  output logic [31:0] WDATA,
  output logic BREADY,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: write_controller_Sum0
logic aw_channel_Sum0_ARESETn;
logic aw_channel_Sum0_AWREADY;
logic [63:0] aw_channel_Sum0_axi_address;
logic aw_channel_Sum0_AWVALID;
logic [63:0] aw_channel_Sum0_AWADDR;
logic [2:0] aw_channel_Sum0_AWSIZE;
logic [7:0] aw_channel_Sum0_AWLEN;
logic [1:0] aw_channel_Sum0_AWBURST;
logic [2:0] aw_channel_Sum0_AWPROT;
logic aw_channel_Sum0_go;
logic aw_channel_Sum0_clk;
logic aw_channel_Sum0_reset;
logic aw_channel_Sum0_done;
logic write_channel_Sum0_ARESETn;
logic write_channel_Sum0_WREADY;
logic [31:0] write_channel_Sum0_write_data;
logic write_channel_Sum0_WVALID;
logic write_channel_Sum0_WLAST;
logic [31:0] write_channel_Sum0_WDATA;
logic write_channel_Sum0_go;
logic write_channel_Sum0_clk;
logic write_channel_Sum0_reset;
logic write_channel_Sum0_done;
logic bresp_channel_Sum0_ARESETn;
logic bresp_channel_Sum0_BVALID;
logic bresp_channel_Sum0_BREADY;
logic bresp_channel_Sum0_go;
logic bresp_channel_Sum0_clk;
logic bresp_channel_Sum0_reset;
logic bresp_channel_Sum0_done;
logic [1:0] fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic [1:0] fsm_out;
logic fsm_done;
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
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
m_aw_channel_Sum0 aw_channel_Sum0 (
    .ARESETn(aw_channel_Sum0_ARESETn),
    .AWADDR(aw_channel_Sum0_AWADDR),
    .AWBURST(aw_channel_Sum0_AWBURST),
    .AWLEN(aw_channel_Sum0_AWLEN),
    .AWPROT(aw_channel_Sum0_AWPROT),
    .AWREADY(aw_channel_Sum0_AWREADY),
    .AWSIZE(aw_channel_Sum0_AWSIZE),
    .AWVALID(aw_channel_Sum0_AWVALID),
    .axi_address(aw_channel_Sum0_axi_address),
    .clk(aw_channel_Sum0_clk),
    .done(aw_channel_Sum0_done),
    .go(aw_channel_Sum0_go),
    .reset(aw_channel_Sum0_reset)
);
m_write_channel_Sum0 write_channel_Sum0 (
    .ARESETn(write_channel_Sum0_ARESETn),
    .WDATA(write_channel_Sum0_WDATA),
    .WLAST(write_channel_Sum0_WLAST),
    .WREADY(write_channel_Sum0_WREADY),
    .WVALID(write_channel_Sum0_WVALID),
    .clk(write_channel_Sum0_clk),
    .done(write_channel_Sum0_done),
    .go(write_channel_Sum0_go),
    .reset(write_channel_Sum0_reset),
    .write_data(write_channel_Sum0_write_data)
);
m_bresp_channel_Sum0 bresp_channel_Sum0 (
    .ARESETn(bresp_channel_Sum0_ARESETn),
    .BREADY(bresp_channel_Sum0_BREADY),
    .BVALID(bresp_channel_Sum0_BVALID),
    .clk(bresp_channel_Sum0_clk),
    .done(bresp_channel_Sum0_done),
    .go(bresp_channel_Sum0_go),
    .reset(bresp_channel_Sum0_reset)
);
std_reg # (
    .WIDTH(2)
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
wire _guard1 = tdcc_done_out;
wire _guard2 = invoke0_go_out;
wire _guard3 = invoke0_go_out;
wire _guard4 = invoke1_go_out;
wire _guard5 = invoke1_go_out;
wire _guard6 = invoke0_go_out;
wire _guard7 = invoke0_go_out;
wire _guard8 = invoke0_go_out;
wire _guard9 = invoke1_go_out;
wire _guard10 = invoke2_go_out;
wire _guard11 = invoke0_go_out;
wire _guard12 = fsm_out == 2'd3;
wire _guard13 = fsm_out == 2'd0;
wire _guard14 = invoke0_done_out;
wire _guard15 = _guard13 & _guard14;
wire _guard16 = tdcc_go_out;
wire _guard17 = _guard15 & _guard16;
wire _guard18 = _guard12 | _guard17;
wire _guard19 = fsm_out == 2'd1;
wire _guard20 = invoke1_done_out;
wire _guard21 = _guard19 & _guard20;
wire _guard22 = tdcc_go_out;
wire _guard23 = _guard21 & _guard22;
wire _guard24 = _guard18 | _guard23;
wire _guard25 = fsm_out == 2'd2;
wire _guard26 = invoke2_done_out;
wire _guard27 = _guard25 & _guard26;
wire _guard28 = tdcc_go_out;
wire _guard29 = _guard27 & _guard28;
wire _guard30 = _guard24 | _guard29;
wire _guard31 = fsm_out == 2'd0;
wire _guard32 = invoke0_done_out;
wire _guard33 = _guard31 & _guard32;
wire _guard34 = tdcc_go_out;
wire _guard35 = _guard33 & _guard34;
wire _guard36 = fsm_out == 2'd3;
wire _guard37 = fsm_out == 2'd2;
wire _guard38 = invoke2_done_out;
wire _guard39 = _guard37 & _guard38;
wire _guard40 = tdcc_go_out;
wire _guard41 = _guard39 & _guard40;
wire _guard42 = fsm_out == 2'd1;
wire _guard43 = invoke1_done_out;
wire _guard44 = _guard42 & _guard43;
wire _guard45 = tdcc_go_out;
wire _guard46 = _guard44 & _guard45;
wire _guard47 = invoke2_go_out;
wire _guard48 = invoke2_go_out;
wire _guard49 = invoke2_done_out;
wire _guard50 = ~_guard49;
wire _guard51 = fsm_out == 2'd2;
wire _guard52 = _guard50 & _guard51;
wire _guard53 = tdcc_go_out;
wire _guard54 = _guard52 & _guard53;
wire _guard55 = invoke0_done_out;
wire _guard56 = ~_guard55;
wire _guard57 = fsm_out == 2'd0;
wire _guard58 = _guard56 & _guard57;
wire _guard59 = tdcc_go_out;
wire _guard60 = _guard58 & _guard59;
wire _guard61 = invoke1_done_out;
wire _guard62 = ~_guard61;
wire _guard63 = fsm_out == 2'd1;
wire _guard64 = _guard62 & _guard63;
wire _guard65 = tdcc_go_out;
wire _guard66 = _guard64 & _guard65;
wire _guard67 = fsm_out == 2'd3;
wire _guard68 = invoke0_go_out;
wire _guard69 = invoke0_go_out;
wire _guard70 = invoke0_go_out;
wire _guard71 = invoke0_go_out;
wire _guard72 = invoke1_go_out;
wire _guard73 = invoke1_go_out;
wire _guard74 = invoke1_go_out;
wire _guard75 = invoke1_go_out;
assign done = _guard1;
assign AWADDR =
  _guard2 ? aw_channel_Sum0_AWADDR :
  64'd0;
assign AWPROT =
  _guard3 ? aw_channel_Sum0_AWPROT :
  3'd0;
assign WVALID =
  _guard4 ? write_channel_Sum0_WVALID :
  1'd0;
assign WDATA =
  _guard5 ? write_channel_Sum0_WDATA :
  32'd0;
assign AWSIZE =
  _guard6 ? aw_channel_Sum0_AWSIZE :
  3'd0;
assign AWVALID =
  _guard7 ? aw_channel_Sum0_AWVALID :
  1'd0;
assign AWBURST =
  _guard8 ? aw_channel_Sum0_AWBURST :
  2'd0;
assign WLAST =
  _guard9 ? write_channel_Sum0_WLAST :
  1'd0;
assign BREADY =
  _guard10 ? bresp_channel_Sum0_BREADY :
  1'd0;
assign AWLEN =
  _guard11 ? aw_channel_Sum0_AWLEN :
  8'd0;
assign fsm_write_en = _guard30;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard35 ? 2'd1 :
  _guard36 ? 2'd0 :
  _guard41 ? 2'd3 :
  _guard46 ? 2'd2 :
  2'd0;
assign bresp_channel_Sum0_clk = clk;
assign bresp_channel_Sum0_go = _guard47;
assign bresp_channel_Sum0_reset = reset;
assign bresp_channel_Sum0_BVALID =
  _guard48 ? BVALID :
  1'd0;
assign invoke2_go_in = _guard54;
assign tdcc_go_in = go;
assign invoke0_go_in = _guard60;
assign invoke0_done_in = aw_channel_Sum0_done;
assign invoke1_go_in = _guard66;
assign invoke2_done_in = bresp_channel_Sum0_done;
assign tdcc_done_in = _guard67;
assign aw_channel_Sum0_clk = clk;
assign aw_channel_Sum0_axi_address =
  _guard68 ? axi_address :
  64'd0;
assign aw_channel_Sum0_AWREADY =
  _guard69 ? AWREADY :
  1'd0;
assign aw_channel_Sum0_go = _guard70;
assign aw_channel_Sum0_reset = reset;
assign aw_channel_Sum0_ARESETn =
  _guard71 ? ARESETn :
  1'd0;
assign write_channel_Sum0_WREADY =
  _guard72 ? WREADY :
  1'd0;
assign write_channel_Sum0_clk = clk;
assign write_channel_Sum0_go = _guard73;
assign write_channel_Sum0_reset = reset;
assign write_channel_Sum0_write_data =
  _guard74 ? write_data :
  32'd0;
assign write_channel_Sum0_ARESETn =
  _guard75 ? ARESETn :
  1'd0;
assign invoke1_done_in = write_channel_Sum0_done;
// COMPONENT END: write_controller_Sum0
endmodule
module axi_dyn_mem_Sum0(
  input logic [2:0] addr0,
  input logic content_en,
  input logic write_en,
  input logic [31:0] write_data,
  input logic ARESETn,
  input logic ARREADY,
  input logic RVALID,
  input logic RLAST,
  input logic [31:0] RDATA,
  input logic [1:0] RRESP,
  input logic AWREADY,
  input logic WREADY,
  input logic BVALID,
  input logic [1:0] BRESP,
  output logic [31:0] read_data,
  output logic ARVALID,
  output logic [63:0] ARADDR,
  output logic [2:0] ARSIZE,
  output logic [7:0] ARLEN,
  output logic [1:0] ARBURST,
  output logic [2:0] ARPROT,
  output logic RREADY,
  output logic AWVALID,
  output logic [63:0] AWADDR,
  output logic [2:0] AWSIZE,
  output logic [7:0] AWLEN,
  output logic [1:0] AWBURST,
  output logic [2:0] AWPROT,
  output logic WVALID,
  output logic WLAST,
  output logic [31:0] WDATA,
  output logic BREADY,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: axi_dyn_mem_Sum0
logic [2:0] address_translator_Sum0_calyx_mem_addr;
logic [63:0] address_translator_Sum0_axi_address;
logic [63:0] read_controller_Sum0_axi_address;
logic read_controller_Sum0_ARESETn;
logic read_controller_Sum0_ARREADY;
logic read_controller_Sum0_RVALID;
logic read_controller_Sum0_RLAST;
logic [31:0] read_controller_Sum0_RDATA;
logic [1:0] read_controller_Sum0_RRESP;
logic read_controller_Sum0_ARVALID;
logic [63:0] read_controller_Sum0_ARADDR;
logic [2:0] read_controller_Sum0_ARSIZE;
logic [7:0] read_controller_Sum0_ARLEN;
logic [1:0] read_controller_Sum0_ARBURST;
logic [2:0] read_controller_Sum0_ARPROT;
logic read_controller_Sum0_RREADY;
logic [31:0] read_controller_Sum0_read_data;
logic read_controller_Sum0_go;
logic read_controller_Sum0_clk;
logic read_controller_Sum0_reset;
logic read_controller_Sum0_done;
logic [63:0] write_controller_Sum0_axi_address;
logic [31:0] write_controller_Sum0_write_data;
logic write_controller_Sum0_ARESETn;
logic write_controller_Sum0_AWREADY;
logic write_controller_Sum0_WREADY;
logic write_controller_Sum0_BVALID;
logic write_controller_Sum0_AWVALID;
logic [63:0] write_controller_Sum0_AWADDR;
logic [2:0] write_controller_Sum0_AWSIZE;
logic [7:0] write_controller_Sum0_AWLEN;
logic [1:0] write_controller_Sum0_AWBURST;
logic [2:0] write_controller_Sum0_AWPROT;
logic write_controller_Sum0_WVALID;
logic write_controller_Sum0_WLAST;
logic [31:0] write_controller_Sum0_WDATA;
logic write_controller_Sum0_BREADY;
logic write_controller_Sum0_go;
logic write_controller_Sum0_clk;
logic write_controller_Sum0_reset;
logic write_controller_Sum0_done;
logic [1:0] fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic [1:0] fsm_out;
logic fsm_done;
logic invoke0_go_in;
logic invoke0_go_out;
logic invoke0_done_in;
logic invoke0_done_out;
logic invoke1_go_in;
logic invoke1_go_out;
logic invoke1_done_in;
logic invoke1_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
address_translator_Sum0 address_translator_Sum0 (
    .axi_address(address_translator_Sum0_axi_address),
    .calyx_mem_addr(address_translator_Sum0_calyx_mem_addr)
);
read_controller_Sum0 read_controller_Sum0 (
    .ARADDR(read_controller_Sum0_ARADDR),
    .ARBURST(read_controller_Sum0_ARBURST),
    .ARESETn(read_controller_Sum0_ARESETn),
    .ARLEN(read_controller_Sum0_ARLEN),
    .ARPROT(read_controller_Sum0_ARPROT),
    .ARREADY(read_controller_Sum0_ARREADY),
    .ARSIZE(read_controller_Sum0_ARSIZE),
    .ARVALID(read_controller_Sum0_ARVALID),
    .RDATA(read_controller_Sum0_RDATA),
    .RLAST(read_controller_Sum0_RLAST),
    .RREADY(read_controller_Sum0_RREADY),
    .RRESP(read_controller_Sum0_RRESP),
    .RVALID(read_controller_Sum0_RVALID),
    .axi_address(read_controller_Sum0_axi_address),
    .clk(read_controller_Sum0_clk),
    .done(read_controller_Sum0_done),
    .go(read_controller_Sum0_go),
    .read_data(read_controller_Sum0_read_data),
    .reset(read_controller_Sum0_reset)
);
write_controller_Sum0 write_controller_Sum0 (
    .ARESETn(write_controller_Sum0_ARESETn),
    .AWADDR(write_controller_Sum0_AWADDR),
    .AWBURST(write_controller_Sum0_AWBURST),
    .AWLEN(write_controller_Sum0_AWLEN),
    .AWPROT(write_controller_Sum0_AWPROT),
    .AWREADY(write_controller_Sum0_AWREADY),
    .AWSIZE(write_controller_Sum0_AWSIZE),
    .AWVALID(write_controller_Sum0_AWVALID),
    .BREADY(write_controller_Sum0_BREADY),
    .BVALID(write_controller_Sum0_BVALID),
    .WDATA(write_controller_Sum0_WDATA),
    .WLAST(write_controller_Sum0_WLAST),
    .WREADY(write_controller_Sum0_WREADY),
    .WVALID(write_controller_Sum0_WVALID),
    .axi_address(write_controller_Sum0_axi_address),
    .clk(write_controller_Sum0_clk),
    .done(write_controller_Sum0_done),
    .go(write_controller_Sum0_go),
    .reset(write_controller_Sum0_reset),
    .write_data(write_controller_Sum0_write_data)
);
std_reg # (
    .WIDTH(2)
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
wire _guard1 = tdcc_done_out;
wire _guard2 = invoke1_go_out;
wire _guard3 = invoke0_go_out;
wire _guard4 = invoke0_go_out;
wire _guard5 = invoke0_go_out;
wire _guard6 = invoke0_go_out;
wire _guard7 = invoke1_go_out;
wire _guard8 = invoke1_go_out;
wire _guard9 = invoke1_go_out;
wire _guard10 = invoke0_go_out;
wire _guard11 = invoke0_go_out;
wire _guard12 = invoke1_go_out;
wire _guard13 = invoke0_go_out;
wire _guard14 = invoke0_go_out;
wire _guard15 = invoke0_go_out;
wire _guard16 = invoke0_go_out;
wire _guard17 = invoke1_go_out;
wire _guard18 = invoke1_go_out;
wire _guard19 = fsm_out == 2'd3;
wire _guard20 = fsm_out == 2'd0;
wire _guard21 = write_en;
wire _guard22 = _guard20 & _guard21;
wire _guard23 = tdcc_go_out;
wire _guard24 = _guard22 & _guard23;
wire _guard25 = _guard19 | _guard24;
wire _guard26 = fsm_out == 2'd0;
wire _guard27 = write_en;
wire _guard28 = ~_guard27;
wire _guard29 = _guard26 & _guard28;
wire _guard30 = tdcc_go_out;
wire _guard31 = _guard29 & _guard30;
wire _guard32 = _guard25 | _guard31;
wire _guard33 = fsm_out == 2'd1;
wire _guard34 = invoke0_done_out;
wire _guard35 = _guard33 & _guard34;
wire _guard36 = tdcc_go_out;
wire _guard37 = _guard35 & _guard36;
wire _guard38 = _guard32 | _guard37;
wire _guard39 = fsm_out == 2'd2;
wire _guard40 = invoke1_done_out;
wire _guard41 = _guard39 & _guard40;
wire _guard42 = tdcc_go_out;
wire _guard43 = _guard41 & _guard42;
wire _guard44 = _guard38 | _guard43;
wire _guard45 = fsm_out == 2'd0;
wire _guard46 = write_en;
wire _guard47 = _guard45 & _guard46;
wire _guard48 = tdcc_go_out;
wire _guard49 = _guard47 & _guard48;
wire _guard50 = fsm_out == 2'd3;
wire _guard51 = fsm_out == 2'd1;
wire _guard52 = invoke0_done_out;
wire _guard53 = _guard51 & _guard52;
wire _guard54 = tdcc_go_out;
wire _guard55 = _guard53 & _guard54;
wire _guard56 = fsm_out == 2'd2;
wire _guard57 = invoke1_done_out;
wire _guard58 = _guard56 & _guard57;
wire _guard59 = tdcc_go_out;
wire _guard60 = _guard58 & _guard59;
wire _guard61 = _guard55 | _guard60;
wire _guard62 = fsm_out == 2'd0;
wire _guard63 = write_en;
wire _guard64 = ~_guard63;
wire _guard65 = _guard62 & _guard64;
wire _guard66 = tdcc_go_out;
wire _guard67 = _guard65 & _guard66;
wire _guard68 = invoke0_done_out;
wire _guard69 = ~_guard68;
wire _guard70 = fsm_out == 2'd1;
wire _guard71 = _guard69 & _guard70;
wire _guard72 = tdcc_go_out;
wire _guard73 = _guard71 & _guard72;
wire _guard74 = invoke0_go_out;
wire _guard75 = invoke0_go_out;
wire _guard76 = invoke0_go_out;
wire _guard77 = invoke0_go_out;
wire _guard78 = invoke0_go_out;
wire _guard79 = invoke0_go_out;
wire _guard80 = invoke0_go_out;
wire _guard81 = invoke1_go_out;
wire _guard82 = invoke1_go_out;
wire _guard83 = invoke1_go_out;
wire _guard84 = invoke1_go_out;
wire _guard85 = invoke1_go_out;
wire _guard86 = invoke1_go_out;
wire _guard87 = invoke1_go_out;
wire _guard88 = invoke1_go_out;
wire _guard89 = invoke1_done_out;
wire _guard90 = ~_guard89;
wire _guard91 = fsm_out == 2'd2;
wire _guard92 = _guard90 & _guard91;
wire _guard93 = tdcc_go_out;
wire _guard94 = _guard92 & _guard93;
wire _guard95 = fsm_out == 2'd3;
assign done = _guard1;
assign ARPROT =
  _guard2 ? read_controller_Sum0_ARPROT :
  3'd0;
assign AWADDR =
  _guard3 ? write_controller_Sum0_AWADDR :
  64'd0;
assign AWPROT =
  _guard4 ? write_controller_Sum0_AWPROT :
  3'd0;
assign WVALID =
  _guard5 ? write_controller_Sum0_WVALID :
  1'd0;
assign WDATA =
  _guard6 ? write_controller_Sum0_WDATA :
  32'd0;
assign ARSIZE =
  _guard7 ? read_controller_Sum0_ARSIZE :
  3'd0;
assign RREADY =
  _guard8 ? read_controller_Sum0_RREADY :
  1'd0;
assign read_data = read_controller_Sum0_read_data;
assign ARLEN =
  _guard9 ? read_controller_Sum0_ARLEN :
  8'd0;
assign AWSIZE =
  _guard10 ? write_controller_Sum0_AWSIZE :
  3'd0;
assign AWVALID =
  _guard11 ? write_controller_Sum0_AWVALID :
  1'd0;
assign ARADDR =
  _guard12 ? read_controller_Sum0_ARADDR :
  64'd0;
assign AWBURST =
  _guard13 ? write_controller_Sum0_AWBURST :
  2'd0;
assign WLAST =
  _guard14 ? write_controller_Sum0_WLAST :
  1'd0;
assign BREADY =
  _guard15 ? write_controller_Sum0_BREADY :
  1'd0;
assign AWLEN =
  _guard16 ? write_controller_Sum0_AWLEN :
  8'd0;
assign ARBURST =
  _guard17 ? read_controller_Sum0_ARBURST :
  2'd0;
assign ARVALID =
  _guard18 ? read_controller_Sum0_ARVALID :
  1'd0;
assign fsm_write_en = _guard44;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard49 ? 2'd1 :
  _guard50 ? 2'd0 :
  _guard61 ? 2'd3 :
  _guard67 ? 2'd2 :
  2'd0;
assign tdcc_go_in = content_en;
assign invoke0_go_in = _guard73;
assign write_controller_Sum0_WREADY =
  _guard74 ? WREADY :
  1'd0;
assign write_controller_Sum0_clk = clk;
assign write_controller_Sum0_axi_address =
  _guard75 ? address_translator_Sum0_axi_address :
  64'd0;
assign write_controller_Sum0_AWREADY =
  _guard76 ? AWREADY :
  1'd0;
assign write_controller_Sum0_go = _guard77;
assign write_controller_Sum0_reset = reset;
assign write_controller_Sum0_write_data =
  _guard78 ? write_data :
  32'd0;
assign write_controller_Sum0_BVALID =
  _guard79 ? BVALID :
  1'd0;
assign write_controller_Sum0_ARESETn =
  _guard80 ? ARESETn :
  1'd0;
assign read_controller_Sum0_RVALID =
  _guard81 ? RVALID :
  1'd0;
assign read_controller_Sum0_RLAST =
  _guard82 ? RLAST :
  1'd0;
assign read_controller_Sum0_RDATA =
  _guard83 ? RDATA :
  32'd0;
assign read_controller_Sum0_clk = clk;
assign read_controller_Sum0_axi_address =
  _guard84 ? address_translator_Sum0_axi_address :
  64'd0;
assign read_controller_Sum0_go = _guard85;
assign read_controller_Sum0_reset = reset;
assign read_controller_Sum0_RRESP =
  _guard86 ? RRESP :
  2'd0;
assign read_controller_Sum0_ARREADY =
  _guard87 ? ARREADY :
  1'd0;
assign read_controller_Sum0_ARESETn =
  _guard88 ? ARESETn :
  1'd0;
assign invoke0_done_in = write_controller_Sum0_done;
assign invoke1_go_in = _guard94;
assign address_translator_Sum0_calyx_mem_addr = addr0;
assign tdcc_done_in = _guard95;
assign invoke1_done_in = read_controller_Sum0_done;
// COMPONENT END: axi_dyn_mem_Sum0
endmodule
module wrapper(
  input logic A0_ARESETn,
  input logic A0_ARREADY,
  input logic A0_RVALID,
  input logic A0_RLAST,
  input logic [31:0] A0_RDATA,
  input logic [1:0] A0_RRESP,
  input logic A0_AWREADY,
  input logic A0_WREADY,
  input logic A0_BVALID,
  input logic [1:0] A0_BRESP,
  input logic A0_RID,
  input logic B0_ARESETn,
  input logic B0_ARREADY,
  input logic B0_RVALID,
  input logic B0_RLAST,
  input logic [31:0] B0_RDATA,
  input logic [1:0] B0_RRESP,
  input logic B0_AWREADY,
  input logic B0_WREADY,
  input logic B0_BVALID,
  input logic [1:0] B0_BRESP,
  input logic B0_RID,
  input logic Sum0_ARESETn,
  input logic Sum0_ARREADY,
  input logic Sum0_RVALID,
  input logic Sum0_RLAST,
  input logic [31:0] Sum0_RDATA,
  input logic [1:0] Sum0_RRESP,
  input logic Sum0_AWREADY,
  input logic Sum0_WREADY,
  input logic Sum0_BVALID,
  input logic [1:0] Sum0_BRESP,
  input logic Sum0_RID,
  output logic A0_ARVALID,
  output logic [63:0] A0_ARADDR,
  output logic [2:0] A0_ARSIZE,
  output logic [7:0] A0_ARLEN,
  output logic [1:0] A0_ARBURST,
  output logic A0_RREADY,
  output logic A0_AWVALID,
  output logic [63:0] A0_AWADDR,
  output logic [2:0] A0_AWSIZE,
  output logic [7:0] A0_AWLEN,
  output logic [1:0] A0_AWBURST,
  output logic [2:0] A0_AWPROT,
  output logic A0_WVALID,
  output logic A0_WLAST,
  output logic [31:0] A0_WDATA,
  output logic A0_BREADY,
  output logic A0_ARID,
  output logic A0_AWID,
  output logic A0_WID,
  output logic A0_BID,
  output logic B0_ARVALID,
  output logic [63:0] B0_ARADDR,
  output logic [2:0] B0_ARSIZE,
  output logic [7:0] B0_ARLEN,
  output logic [1:0] B0_ARBURST,
  output logic B0_RREADY,
  output logic B0_AWVALID,
  output logic [63:0] B0_AWADDR,
  output logic [2:0] B0_AWSIZE,
  output logic [7:0] B0_AWLEN,
  output logic [1:0] B0_AWBURST,
  output logic [2:0] B0_AWPROT,
  output logic B0_WVALID,
  output logic B0_WLAST,
  output logic [31:0] B0_WDATA,
  output logic B0_BREADY,
  output logic B0_ARID,
  output logic B0_AWID,
  output logic B0_WID,
  output logic B0_BID,
  output logic Sum0_ARVALID,
  output logic [63:0] Sum0_ARADDR,
  output logic [2:0] Sum0_ARSIZE,
  output logic [7:0] Sum0_ARLEN,
  output logic [1:0] Sum0_ARBURST,
  output logic Sum0_RREADY,
  output logic Sum0_AWVALID,
  output logic [63:0] Sum0_AWADDR,
  output logic [2:0] Sum0_AWSIZE,
  output logic [7:0] Sum0_AWLEN,
  output logic [1:0] Sum0_AWBURST,
  output logic [2:0] Sum0_AWPROT,
  output logic Sum0_WVALID,
  output logic Sum0_WLAST,
  output logic [31:0] Sum0_WDATA,
  output logic Sum0_BREADY,
  output logic Sum0_ARID,
  output logic Sum0_AWID,
  output logic Sum0_WID,
  output logic Sum0_BID,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: wrapper
logic main_compute_go;
logic main_compute_clk;
logic main_compute_reset;
logic main_compute_done;
logic [31:0] main_compute_A0_read_data;
logic [31:0] main_compute_B0_read_data;
logic main_compute_B0_write_en;
logic main_compute_Sum0_done;
logic [31:0] main_compute_A0_write_data;
logic [2:0] main_compute_Sum0_addr0;
logic main_compute_A0_write_en;
logic [2:0] main_compute_B0_addr0;
logic main_compute_B0_content_en;
logic main_compute_B0_done;
logic [2:0] main_compute_A0_addr0;
logic main_compute_A0_done;
logic main_compute_Sum0_write_en;
logic [31:0] main_compute_B0_write_data;
logic main_compute_Sum0_content_en;
logic [31:0] main_compute_Sum0_write_data;
logic [31:0] main_compute_Sum0_read_data;
logic main_compute_A0_content_en;
logic [2:0] axi_dyn_mem_A0_addr0;
logic axi_dyn_mem_A0_content_en;
logic axi_dyn_mem_A0_write_en;
logic [31:0] axi_dyn_mem_A0_write_data;
logic axi_dyn_mem_A0_ARESETn;
logic axi_dyn_mem_A0_ARREADY;
logic axi_dyn_mem_A0_RVALID;
logic axi_dyn_mem_A0_RLAST;
logic [31:0] axi_dyn_mem_A0_RDATA;
logic [1:0] axi_dyn_mem_A0_RRESP;
logic axi_dyn_mem_A0_AWREADY;
logic axi_dyn_mem_A0_WREADY;
logic axi_dyn_mem_A0_BVALID;
logic [1:0] axi_dyn_mem_A0_BRESP;
logic [31:0] axi_dyn_mem_A0_read_data;
logic axi_dyn_mem_A0_ARVALID;
logic [63:0] axi_dyn_mem_A0_ARADDR;
logic [2:0] axi_dyn_mem_A0_ARSIZE;
logic [7:0] axi_dyn_mem_A0_ARLEN;
logic [1:0] axi_dyn_mem_A0_ARBURST;
logic [2:0] axi_dyn_mem_A0_ARPROT;
logic axi_dyn_mem_A0_RREADY;
logic axi_dyn_mem_A0_AWVALID;
logic [63:0] axi_dyn_mem_A0_AWADDR;
logic [2:0] axi_dyn_mem_A0_AWSIZE;
logic [7:0] axi_dyn_mem_A0_AWLEN;
logic [1:0] axi_dyn_mem_A0_AWBURST;
logic [2:0] axi_dyn_mem_A0_AWPROT;
logic axi_dyn_mem_A0_WVALID;
logic axi_dyn_mem_A0_WLAST;
logic [31:0] axi_dyn_mem_A0_WDATA;
logic axi_dyn_mem_A0_BREADY;
logic axi_dyn_mem_A0_clk;
logic axi_dyn_mem_A0_reset;
logic axi_dyn_mem_A0_done;
logic [2:0] axi_dyn_mem_B0_addr0;
logic axi_dyn_mem_B0_content_en;
logic axi_dyn_mem_B0_write_en;
logic [31:0] axi_dyn_mem_B0_write_data;
logic axi_dyn_mem_B0_ARESETn;
logic axi_dyn_mem_B0_ARREADY;
logic axi_dyn_mem_B0_RVALID;
logic axi_dyn_mem_B0_RLAST;
logic [31:0] axi_dyn_mem_B0_RDATA;
logic [1:0] axi_dyn_mem_B0_RRESP;
logic axi_dyn_mem_B0_AWREADY;
logic axi_dyn_mem_B0_WREADY;
logic axi_dyn_mem_B0_BVALID;
logic [1:0] axi_dyn_mem_B0_BRESP;
logic [31:0] axi_dyn_mem_B0_read_data;
logic axi_dyn_mem_B0_ARVALID;
logic [63:0] axi_dyn_mem_B0_ARADDR;
logic [2:0] axi_dyn_mem_B0_ARSIZE;
logic [7:0] axi_dyn_mem_B0_ARLEN;
logic [1:0] axi_dyn_mem_B0_ARBURST;
logic [2:0] axi_dyn_mem_B0_ARPROT;
logic axi_dyn_mem_B0_RREADY;
logic axi_dyn_mem_B0_AWVALID;
logic [63:0] axi_dyn_mem_B0_AWADDR;
logic [2:0] axi_dyn_mem_B0_AWSIZE;
logic [7:0] axi_dyn_mem_B0_AWLEN;
logic [1:0] axi_dyn_mem_B0_AWBURST;
logic [2:0] axi_dyn_mem_B0_AWPROT;
logic axi_dyn_mem_B0_WVALID;
logic axi_dyn_mem_B0_WLAST;
logic [31:0] axi_dyn_mem_B0_WDATA;
logic axi_dyn_mem_B0_BREADY;
logic axi_dyn_mem_B0_clk;
logic axi_dyn_mem_B0_reset;
logic axi_dyn_mem_B0_done;
logic [2:0] axi_dyn_mem_Sum0_addr0;
logic axi_dyn_mem_Sum0_content_en;
logic axi_dyn_mem_Sum0_write_en;
logic [31:0] axi_dyn_mem_Sum0_write_data;
logic axi_dyn_mem_Sum0_ARESETn;
logic axi_dyn_mem_Sum0_ARREADY;
logic axi_dyn_mem_Sum0_RVALID;
logic axi_dyn_mem_Sum0_RLAST;
logic [31:0] axi_dyn_mem_Sum0_RDATA;
logic [1:0] axi_dyn_mem_Sum0_RRESP;
logic axi_dyn_mem_Sum0_AWREADY;
logic axi_dyn_mem_Sum0_WREADY;
logic axi_dyn_mem_Sum0_BVALID;
logic [1:0] axi_dyn_mem_Sum0_BRESP;
logic [31:0] axi_dyn_mem_Sum0_read_data;
logic axi_dyn_mem_Sum0_ARVALID;
logic [63:0] axi_dyn_mem_Sum0_ARADDR;
logic [2:0] axi_dyn_mem_Sum0_ARSIZE;
logic [7:0] axi_dyn_mem_Sum0_ARLEN;
logic [1:0] axi_dyn_mem_Sum0_ARBURST;
logic [2:0] axi_dyn_mem_Sum0_ARPROT;
logic axi_dyn_mem_Sum0_RREADY;
logic axi_dyn_mem_Sum0_AWVALID;
logic [63:0] axi_dyn_mem_Sum0_AWADDR;
logic [2:0] axi_dyn_mem_Sum0_AWSIZE;
logic [7:0] axi_dyn_mem_Sum0_AWLEN;
logic [1:0] axi_dyn_mem_Sum0_AWBURST;
logic [2:0] axi_dyn_mem_Sum0_AWPROT;
logic axi_dyn_mem_Sum0_WVALID;
logic axi_dyn_mem_Sum0_WLAST;
logic [31:0] axi_dyn_mem_Sum0_WDATA;
logic axi_dyn_mem_Sum0_BREADY;
logic axi_dyn_mem_Sum0_clk;
logic axi_dyn_mem_Sum0_reset;
logic axi_dyn_mem_Sum0_done;
logic invoke0_go_in;
logic invoke0_go_out;
logic invoke0_done_in;
logic invoke0_done_out;
main main_compute (
    .A0_addr0(main_compute_A0_addr0),
    .A0_content_en(main_compute_A0_content_en),
    .A0_done(main_compute_A0_done),
    .A0_read_data(main_compute_A0_read_data),
    .A0_write_data(main_compute_A0_write_data),
    .A0_write_en(main_compute_A0_write_en),
    .B0_addr0(main_compute_B0_addr0),
    .B0_content_en(main_compute_B0_content_en),
    .B0_done(main_compute_B0_done),
    .B0_read_data(main_compute_B0_read_data),
    .B0_write_data(main_compute_B0_write_data),
    .B0_write_en(main_compute_B0_write_en),
    .Sum0_addr0(main_compute_Sum0_addr0),
    .Sum0_content_en(main_compute_Sum0_content_en),
    .Sum0_done(main_compute_Sum0_done),
    .Sum0_read_data(main_compute_Sum0_read_data),
    .Sum0_write_data(main_compute_Sum0_write_data),
    .Sum0_write_en(main_compute_Sum0_write_en),
    .clk(main_compute_clk),
    .done(main_compute_done),
    .go(main_compute_go),
    .reset(main_compute_reset)
);
axi_dyn_mem_A0 axi_dyn_mem_A0 (
    .ARADDR(axi_dyn_mem_A0_ARADDR),
    .ARBURST(axi_dyn_mem_A0_ARBURST),
    .ARESETn(axi_dyn_mem_A0_ARESETn),
    .ARLEN(axi_dyn_mem_A0_ARLEN),
    .ARPROT(axi_dyn_mem_A0_ARPROT),
    .ARREADY(axi_dyn_mem_A0_ARREADY),
    .ARSIZE(axi_dyn_mem_A0_ARSIZE),
    .ARVALID(axi_dyn_mem_A0_ARVALID),
    .AWADDR(axi_dyn_mem_A0_AWADDR),
    .AWBURST(axi_dyn_mem_A0_AWBURST),
    .AWLEN(axi_dyn_mem_A0_AWLEN),
    .AWPROT(axi_dyn_mem_A0_AWPROT),
    .AWREADY(axi_dyn_mem_A0_AWREADY),
    .AWSIZE(axi_dyn_mem_A0_AWSIZE),
    .AWVALID(axi_dyn_mem_A0_AWVALID),
    .BREADY(axi_dyn_mem_A0_BREADY),
    .BRESP(axi_dyn_mem_A0_BRESP),
    .BVALID(axi_dyn_mem_A0_BVALID),
    .RDATA(axi_dyn_mem_A0_RDATA),
    .RLAST(axi_dyn_mem_A0_RLAST),
    .RREADY(axi_dyn_mem_A0_RREADY),
    .RRESP(axi_dyn_mem_A0_RRESP),
    .RVALID(axi_dyn_mem_A0_RVALID),
    .WDATA(axi_dyn_mem_A0_WDATA),
    .WLAST(axi_dyn_mem_A0_WLAST),
    .WREADY(axi_dyn_mem_A0_WREADY),
    .WVALID(axi_dyn_mem_A0_WVALID),
    .addr0(axi_dyn_mem_A0_addr0),
    .clk(axi_dyn_mem_A0_clk),
    .content_en(axi_dyn_mem_A0_content_en),
    .done(axi_dyn_mem_A0_done),
    .read_data(axi_dyn_mem_A0_read_data),
    .reset(axi_dyn_mem_A0_reset),
    .write_data(axi_dyn_mem_A0_write_data),
    .write_en(axi_dyn_mem_A0_write_en)
);
axi_dyn_mem_B0 axi_dyn_mem_B0 (
    .ARADDR(axi_dyn_mem_B0_ARADDR),
    .ARBURST(axi_dyn_mem_B0_ARBURST),
    .ARESETn(axi_dyn_mem_B0_ARESETn),
    .ARLEN(axi_dyn_mem_B0_ARLEN),
    .ARPROT(axi_dyn_mem_B0_ARPROT),
    .ARREADY(axi_dyn_mem_B0_ARREADY),
    .ARSIZE(axi_dyn_mem_B0_ARSIZE),
    .ARVALID(axi_dyn_mem_B0_ARVALID),
    .AWADDR(axi_dyn_mem_B0_AWADDR),
    .AWBURST(axi_dyn_mem_B0_AWBURST),
    .AWLEN(axi_dyn_mem_B0_AWLEN),
    .AWPROT(axi_dyn_mem_B0_AWPROT),
    .AWREADY(axi_dyn_mem_B0_AWREADY),
    .AWSIZE(axi_dyn_mem_B0_AWSIZE),
    .AWVALID(axi_dyn_mem_B0_AWVALID),
    .BREADY(axi_dyn_mem_B0_BREADY),
    .BRESP(axi_dyn_mem_B0_BRESP),
    .BVALID(axi_dyn_mem_B0_BVALID),
    .RDATA(axi_dyn_mem_B0_RDATA),
    .RLAST(axi_dyn_mem_B0_RLAST),
    .RREADY(axi_dyn_mem_B0_RREADY),
    .RRESP(axi_dyn_mem_B0_RRESP),
    .RVALID(axi_dyn_mem_B0_RVALID),
    .WDATA(axi_dyn_mem_B0_WDATA),
    .WLAST(axi_dyn_mem_B0_WLAST),
    .WREADY(axi_dyn_mem_B0_WREADY),
    .WVALID(axi_dyn_mem_B0_WVALID),
    .addr0(axi_dyn_mem_B0_addr0),
    .clk(axi_dyn_mem_B0_clk),
    .content_en(axi_dyn_mem_B0_content_en),
    .done(axi_dyn_mem_B0_done),
    .read_data(axi_dyn_mem_B0_read_data),
    .reset(axi_dyn_mem_B0_reset),
    .write_data(axi_dyn_mem_B0_write_data),
    .write_en(axi_dyn_mem_B0_write_en)
);
axi_dyn_mem_Sum0 axi_dyn_mem_Sum0 (
    .ARADDR(axi_dyn_mem_Sum0_ARADDR),
    .ARBURST(axi_dyn_mem_Sum0_ARBURST),
    .ARESETn(axi_dyn_mem_Sum0_ARESETn),
    .ARLEN(axi_dyn_mem_Sum0_ARLEN),
    .ARPROT(axi_dyn_mem_Sum0_ARPROT),
    .ARREADY(axi_dyn_mem_Sum0_ARREADY),
    .ARSIZE(axi_dyn_mem_Sum0_ARSIZE),
    .ARVALID(axi_dyn_mem_Sum0_ARVALID),
    .AWADDR(axi_dyn_mem_Sum0_AWADDR),
    .AWBURST(axi_dyn_mem_Sum0_AWBURST),
    .AWLEN(axi_dyn_mem_Sum0_AWLEN),
    .AWPROT(axi_dyn_mem_Sum0_AWPROT),
    .AWREADY(axi_dyn_mem_Sum0_AWREADY),
    .AWSIZE(axi_dyn_mem_Sum0_AWSIZE),
    .AWVALID(axi_dyn_mem_Sum0_AWVALID),
    .BREADY(axi_dyn_mem_Sum0_BREADY),
    .BRESP(axi_dyn_mem_Sum0_BRESP),
    .BVALID(axi_dyn_mem_Sum0_BVALID),
    .RDATA(axi_dyn_mem_Sum0_RDATA),
    .RLAST(axi_dyn_mem_Sum0_RLAST),
    .RREADY(axi_dyn_mem_Sum0_RREADY),
    .RRESP(axi_dyn_mem_Sum0_RRESP),
    .RVALID(axi_dyn_mem_Sum0_RVALID),
    .WDATA(axi_dyn_mem_Sum0_WDATA),
    .WLAST(axi_dyn_mem_Sum0_WLAST),
    .WREADY(axi_dyn_mem_Sum0_WREADY),
    .WVALID(axi_dyn_mem_Sum0_WVALID),
    .addr0(axi_dyn_mem_Sum0_addr0),
    .clk(axi_dyn_mem_Sum0_clk),
    .content_en(axi_dyn_mem_Sum0_content_en),
    .done(axi_dyn_mem_Sum0_done),
    .read_data(axi_dyn_mem_Sum0_read_data),
    .reset(axi_dyn_mem_Sum0_reset),
    .write_data(axi_dyn_mem_Sum0_write_data),
    .write_en(axi_dyn_mem_Sum0_write_en)
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
wire _guard3 = invoke0_go_out;
wire _guard4 = invoke0_go_out;
wire _guard5 = invoke0_go_out;
wire _guard6 = invoke0_go_out;
wire _guard7 = invoke0_go_out;
wire _guard8 = invoke0_go_out;
wire _guard9 = invoke0_done_out;
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
assign axi_dyn_mem_A0_WREADY = A0_WREADY;
assign axi_dyn_mem_A0_RVALID = A0_RVALID;
assign axi_dyn_mem_A0_RLAST = A0_RLAST;
assign axi_dyn_mem_A0_write_en =
  _guard1 ? main_compute_A0_write_en :
  1'd0;
assign axi_dyn_mem_A0_RDATA = A0_RDATA;
assign axi_dyn_mem_A0_clk = clk;
assign axi_dyn_mem_A0_addr0 =
  _guard2 ? main_compute_A0_addr0 :
  3'd0;
assign axi_dyn_mem_A0_content_en =
  _guard3 ? main_compute_A0_content_en :
  1'd0;
assign axi_dyn_mem_A0_AWREADY = A0_AWREADY;
assign axi_dyn_mem_A0_reset = reset;
assign axi_dyn_mem_A0_RRESP = A0_RRESP;
assign axi_dyn_mem_A0_write_data =
  _guard4 ? main_compute_A0_write_data :
  32'd0;
assign axi_dyn_mem_A0_ARREADY = A0_ARREADY;
assign axi_dyn_mem_A0_BVALID = A0_BVALID;
assign axi_dyn_mem_A0_ARESETn = A0_ARESETn;
assign axi_dyn_mem_Sum0_WREADY = Sum0_WREADY;
assign axi_dyn_mem_Sum0_RVALID = Sum0_RVALID;
assign axi_dyn_mem_Sum0_RLAST = Sum0_RLAST;
assign axi_dyn_mem_Sum0_write_en =
  _guard5 ? main_compute_Sum0_write_en :
  1'd0;
assign axi_dyn_mem_Sum0_RDATA = Sum0_RDATA;
assign axi_dyn_mem_Sum0_clk = clk;
assign axi_dyn_mem_Sum0_addr0 =
  _guard6 ? main_compute_Sum0_addr0 :
  3'd0;
assign axi_dyn_mem_Sum0_content_en =
  _guard7 ? main_compute_Sum0_content_en :
  1'd0;
assign axi_dyn_mem_Sum0_AWREADY = Sum0_AWREADY;
assign axi_dyn_mem_Sum0_reset = reset;
assign axi_dyn_mem_Sum0_RRESP = Sum0_RRESP;
assign axi_dyn_mem_Sum0_write_data =
  _guard8 ? main_compute_Sum0_write_data :
  32'd0;
assign axi_dyn_mem_Sum0_ARREADY = Sum0_ARREADY;
assign axi_dyn_mem_Sum0_BVALID = Sum0_BVALID;
assign axi_dyn_mem_Sum0_ARESETn = Sum0_ARESETn;
assign done = _guard9;
assign B0_WLAST = axi_dyn_mem_B0_WLAST;
assign Sum0_ARVALID = axi_dyn_mem_Sum0_ARVALID;
assign Sum0_ARBURST = axi_dyn_mem_Sum0_ARBURST;
assign Sum0_AWADDR = axi_dyn_mem_Sum0_AWADDR;
assign Sum0_AWSIZE = axi_dyn_mem_Sum0_AWSIZE;
assign Sum0_ARID = 1'd0;
assign A0_ARSIZE = axi_dyn_mem_A0_ARSIZE;
assign A0_AWBURST = axi_dyn_mem_A0_AWBURST;
assign B0_AWBURST = axi_dyn_mem_B0_AWBURST;
assign Sum0_WDATA = axi_dyn_mem_Sum0_WDATA;
assign A0_BREADY = axi_dyn_mem_A0_BREADY;
assign B0_AWLEN = axi_dyn_mem_B0_AWLEN;
assign Sum0_RREADY = axi_dyn_mem_Sum0_RREADY;
assign B0_ARID = 1'd0;
assign B0_ARBURST = axi_dyn_mem_B0_ARBURST;
assign B0_AWVALID = axi_dyn_mem_B0_AWVALID;
assign B0_WVALID = axi_dyn_mem_B0_WVALID;
assign Sum0_AWLEN = axi_dyn_mem_Sum0_AWLEN;
assign Sum0_BID = 1'd0;
assign A0_AWSIZE = axi_dyn_mem_A0_AWSIZE;
assign B0_ARLEN = axi_dyn_mem_B0_ARLEN;
assign B0_WID = 1'd0;
assign B0_BID = 1'd0;
assign A0_WLAST = axi_dyn_mem_A0_WLAST;
assign B0_ARVALID = axi_dyn_mem_B0_ARVALID;
assign B0_AWPROT = axi_dyn_mem_B0_AWPROT;
assign Sum0_AWPROT = axi_dyn_mem_Sum0_AWPROT;
assign A0_ARBURST = axi_dyn_mem_A0_ARBURST;
assign A0_AWPROT = axi_dyn_mem_A0_AWPROT;
assign Sum0_WLAST = axi_dyn_mem_Sum0_WLAST;
assign Sum0_BREADY = axi_dyn_mem_Sum0_BREADY;
assign Sum0_AWID = 1'd0;
assign A0_WVALID = axi_dyn_mem_A0_WVALID;
assign A0_WID = 1'd0;
assign B0_AWID = 1'd0;
assign A0_ARADDR = axi_dyn_mem_A0_ARADDR;
assign A0_WDATA = axi_dyn_mem_A0_WDATA;
assign Sum0_ARADDR = axi_dyn_mem_Sum0_ARADDR;
assign Sum0_AWBURST = axi_dyn_mem_Sum0_AWBURST;
assign Sum0_WVALID = axi_dyn_mem_Sum0_WVALID;
assign A0_RREADY = axi_dyn_mem_A0_RREADY;
assign A0_BID = 1'd0;
assign B0_RREADY = axi_dyn_mem_B0_RREADY;
assign B0_WDATA = axi_dyn_mem_B0_WDATA;
assign B0_BREADY = axi_dyn_mem_B0_BREADY;
assign A0_AWLEN = axi_dyn_mem_A0_AWLEN;
assign B0_ARSIZE = axi_dyn_mem_B0_ARSIZE;
assign A0_AWADDR = axi_dyn_mem_A0_AWADDR;
assign B0_ARADDR = axi_dyn_mem_B0_ARADDR;
assign Sum0_ARSIZE = axi_dyn_mem_Sum0_ARSIZE;
assign Sum0_ARLEN = axi_dyn_mem_Sum0_ARLEN;
assign Sum0_AWVALID = axi_dyn_mem_Sum0_AWVALID;
assign A0_ARVALID = axi_dyn_mem_A0_ARVALID;
assign A0_AWVALID = axi_dyn_mem_A0_AWVALID;
assign A0_ARID = 1'd0;
assign B0_AWADDR = axi_dyn_mem_B0_AWADDR;
assign B0_AWSIZE = axi_dyn_mem_B0_AWSIZE;
assign Sum0_WID = 1'd0;
assign A0_ARLEN = axi_dyn_mem_A0_ARLEN;
assign A0_AWID = 1'd0;
assign main_compute_A0_read_data =
  _guard10 ? axi_dyn_mem_A0_read_data :
  32'd0;
assign main_compute_B0_read_data =
  _guard11 ? axi_dyn_mem_B0_read_data :
  32'd0;
assign main_compute_Sum0_done =
  _guard12 ? axi_dyn_mem_Sum0_done :
  1'd0;
assign main_compute_clk = clk;
assign main_compute_B0_done =
  _guard13 ? axi_dyn_mem_B0_done :
  1'd0;
assign main_compute_go = _guard14;
assign main_compute_reset = reset;
assign main_compute_A0_done =
  _guard15 ? axi_dyn_mem_A0_done :
  1'd0;
assign main_compute_Sum0_read_data =
  _guard16 ? axi_dyn_mem_Sum0_read_data :
  32'd0;
assign invoke0_go_in = go;
assign invoke0_done_in = main_compute_done;
assign axi_dyn_mem_B0_WREADY = B0_WREADY;
assign axi_dyn_mem_B0_RVALID = B0_RVALID;
assign axi_dyn_mem_B0_RLAST = B0_RLAST;
assign axi_dyn_mem_B0_write_en =
  _guard17 ? main_compute_B0_write_en :
  1'd0;
assign axi_dyn_mem_B0_RDATA = B0_RDATA;
assign axi_dyn_mem_B0_clk = clk;
assign axi_dyn_mem_B0_addr0 =
  _guard18 ? main_compute_B0_addr0 :
  3'd0;
assign axi_dyn_mem_B0_content_en =
  _guard19 ? main_compute_B0_content_en :
  1'd0;
assign axi_dyn_mem_B0_AWREADY = B0_AWREADY;
assign axi_dyn_mem_B0_reset = reset;
assign axi_dyn_mem_B0_RRESP = B0_RRESP;
assign axi_dyn_mem_B0_write_data =
  _guard20 ? main_compute_B0_write_data :
  32'd0;
assign axi_dyn_mem_B0_ARREADY = B0_ARREADY;
assign axi_dyn_mem_B0_BVALID = B0_BVALID;
assign axi_dyn_mem_B0_ARESETn = B0_ARESETn;
// COMPONENT END: wrapper
endmodule
module main(
  input logic go,
  input logic clk,
  input logic reset,
  output logic done,
  output logic [2:0] A0_addr0,
  output logic A0_content_en,
  output logic A0_write_en,
  output logic [31:0] A0_write_data,
  input logic [31:0] A0_read_data,
  input logic A0_done,
  output logic [2:0] B0_addr0,
  output logic B0_content_en,
  output logic B0_write_en,
  output logic [31:0] B0_write_data,
  input logic [31:0] B0_read_data,
  input logic B0_done,
  output logic [2:0] Sum0_addr0,
  output logic Sum0_content_en,
  output logic Sum0_write_en,
  output logic [31:0] Sum0_write_data,
  input logic [31:0] Sum0_read_data,
  input logic Sum0_done
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
logic [3:0] const2_out;
logic [3:0] i0_in;
logic i0_write_en;
logic i0_clk;
logic i0_reset;
logic [3:0] i0_out;
logic i0_done;
logic [3:0] le0_left;
logic [3:0] le0_right;
logic le0_out;
logic [3:0] bit_slice_in;
logic [2:0] bit_slice_out;
logic comb_reg_in;
logic comb_reg_write_en;
logic comb_reg_clk;
logic comb_reg_reset;
logic comb_reg_out;
logic comb_reg_done;
logic fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic fsm_out;
logic fsm_done;
logic adder_left;
logic adder_right;
logic adder_out;
logic ud_out;
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
logic pd_in;
logic pd_write_en;
logic pd_clk;
logic pd_reset;
logic pd_out;
logic pd_done;
logic [1:0] fsm1_in;
logic fsm1_write_en;
logic fsm1_clk;
logic fsm1_reset;
logic [1:0] fsm1_out;
logic fsm1_done;
logic pd0_in;
logic pd0_write_en;
logic pd0_clk;
logic pd0_reset;
logic pd0_out;
logic pd0_done;
logic [2:0] fsm2_in;
logic fsm2_write_en;
logic fsm2_clk;
logic fsm2_reset;
logic [2:0] fsm2_out;
logic fsm2_done;
logic beg_spl_upd0_go_in;
logic beg_spl_upd0_go_out;
logic beg_spl_upd0_done_in;
logic beg_spl_upd0_done_out;
logic beg_spl_upd1_go_in;
logic beg_spl_upd1_go_out;
logic beg_spl_upd1_done_in;
logic beg_spl_upd1_done_out;
logic upd2_go_in;
logic upd2_go_out;
logic upd2_done_in;
logic upd2_done_out;
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
logic early_reset_cond00_go_in;
logic early_reset_cond00_go_out;
logic early_reset_cond00_done_in;
logic early_reset_cond00_done_out;
logic wrapper_early_reset_cond00_go_in;
logic wrapper_early_reset_cond00_go_out;
logic wrapper_early_reset_cond00_done_in;
logic wrapper_early_reset_cond00_done_out;
logic par0_go_in;
logic par0_go_out;
logic par0_done_in;
logic par0_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
logic tdcc0_go_in;
logic tdcc0_go_out;
logic tdcc0_done_in;
logic tdcc0_done_out;
logic tdcc1_go_in;
logic tdcc1_go_out;
logic tdcc1_done_in;
logic tdcc1_done_out;
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
    .VALUE(4'd1),
    .WIDTH(4)
) const2 (
    .out(const2_out)
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
std_bit_slice # (
    .END_IDX(2),
    .IN_WIDTH(4),
    .OUT_WIDTH(3),
    .START_IDX(0)
) bit_slice (
    .in(bit_slice_in),
    .out(bit_slice_out)
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
    .WIDTH(1)
) fsm (
    .clk(fsm_clk),
    .done(fsm_done),
    .in(fsm_in),
    .out(fsm_out),
    .reset(fsm_reset),
    .write_en(fsm_write_en)
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
) ud (
    .out(ud_out)
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
std_reg # (
    .WIDTH(1)
) pd (
    .clk(pd_clk),
    .done(pd_done),
    .in(pd_in),
    .out(pd_out),
    .reset(pd_reset),
    .write_en(pd_write_en)
);
std_reg # (
    .WIDTH(2)
) fsm1 (
    .clk(fsm1_clk),
    .done(fsm1_done),
    .in(fsm1_in),
    .out(fsm1_out),
    .reset(fsm1_reset),
    .write_en(fsm1_write_en)
);
std_reg # (
    .WIDTH(1)
) pd0 (
    .clk(pd0_clk),
    .done(pd0_done),
    .in(pd0_in),
    .out(pd0_out),
    .reset(pd0_reset),
    .write_en(pd0_write_en)
);
std_reg # (
    .WIDTH(3)
) fsm2 (
    .clk(fsm2_clk),
    .done(fsm2_done),
    .in(fsm2_in),
    .out(fsm2_out),
    .reset(fsm2_reset),
    .write_en(fsm2_write_en)
);
std_wire # (
    .WIDTH(1)
) beg_spl_upd0_go (
    .in(beg_spl_upd0_go_in),
    .out(beg_spl_upd0_go_out)
);
std_wire # (
    .WIDTH(1)
) beg_spl_upd0_done (
    .in(beg_spl_upd0_done_in),
    .out(beg_spl_upd0_done_out)
);
std_wire # (
    .WIDTH(1)
) beg_spl_upd1_go (
    .in(beg_spl_upd1_go_in),
    .out(beg_spl_upd1_go_out)
);
std_wire # (
    .WIDTH(1)
) beg_spl_upd1_done (
    .in(beg_spl_upd1_done_in),
    .out(beg_spl_upd1_done_out)
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
) early_reset_cond00_go (
    .in(early_reset_cond00_go_in),
    .out(early_reset_cond00_go_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_cond00_done (
    .in(early_reset_cond00_done_in),
    .out(early_reset_cond00_done_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_cond00_go (
    .in(wrapper_early_reset_cond00_go_in),
    .out(wrapper_early_reset_cond00_go_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_cond00_done (
    .in(wrapper_early_reset_cond00_done_in),
    .out(wrapper_early_reset_cond00_done_out)
);
std_wire # (
    .WIDTH(1)
) par0_go (
    .in(par0_go_in),
    .out(par0_go_out)
);
std_wire # (
    .WIDTH(1)
) par0_done (
    .in(par0_done_in),
    .out(par0_done_out)
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
std_wire # (
    .WIDTH(1)
) tdcc0_go (
    .in(tdcc0_go_in),
    .out(tdcc0_go_out)
);
std_wire # (
    .WIDTH(1)
) tdcc0_done (
    .in(tdcc0_done_in),
    .out(tdcc0_done_out)
);
std_wire # (
    .WIDTH(1)
) tdcc1_go (
    .in(tdcc1_go_in),
    .out(tdcc1_go_out)
);
std_wire # (
    .WIDTH(1)
) tdcc1_done (
    .in(tdcc1_done_in),
    .out(tdcc1_done_out)
);
wire _guard0 = 1;
wire _guard1 = invoke0_go_out;
wire _guard2 = invoke3_go_out;
wire _guard3 = _guard1 | _guard2;
wire _guard4 = invoke3_go_out;
wire _guard5 = invoke0_go_out;
wire _guard6 = wrapper_early_reset_cond00_go_out;
wire _guard7 = invoke3_go_out;
wire _guard8 = invoke3_go_out;
wire _guard9 = tdcc1_done_out;
wire _guard10 = upd2_go_out;
wire _guard11 = beg_spl_upd1_go_out;
wire _guard12 = beg_spl_upd1_go_out;
wire _guard13 = beg_spl_upd0_go_out;
wire _guard14 = upd2_go_out;
wire _guard15 = upd2_go_out;
wire _guard16 = upd2_go_out;
wire _guard17 = beg_spl_upd0_go_out;
wire _guard18 = early_reset_cond00_go_out;
wire _guard19 = fsm_out == 1'd0;
wire _guard20 = ~_guard19;
wire _guard21 = early_reset_cond00_go_out;
wire _guard22 = _guard20 & _guard21;
wire _guard23 = fsm_out == 1'd0;
wire _guard24 = early_reset_cond00_go_out;
wire _guard25 = _guard23 & _guard24;
wire _guard26 = early_reset_cond00_go_out;
wire _guard27 = early_reset_cond00_go_out;
wire _guard28 = beg_spl_upd0_done_out;
wire _guard29 = ~_guard28;
wire _guard30 = fsm0_out == 2'd0;
wire _guard31 = _guard29 & _guard30;
wire _guard32 = tdcc_go_out;
wire _guard33 = _guard31 & _guard32;
wire _guard34 = upd2_go_out;
wire _guard35 = upd2_go_out;
wire _guard36 = invoke2_done_out;
wire _guard37 = ~_guard36;
wire _guard38 = fsm1_out == 2'd1;
wire _guard39 = _guard37 & _guard38;
wire _guard40 = tdcc0_go_out;
wire _guard41 = _guard39 & _guard40;
wire _guard42 = fsm1_out == 2'd2;
wire _guard43 = early_reset_cond00_go_out;
wire _guard44 = early_reset_cond00_go_out;
wire _guard45 = fsm1_out == 2'd2;
wire _guard46 = fsm1_out == 2'd0;
wire _guard47 = beg_spl_upd1_done_out;
wire _guard48 = _guard46 & _guard47;
wire _guard49 = tdcc0_go_out;
wire _guard50 = _guard48 & _guard49;
wire _guard51 = _guard45 | _guard50;
wire _guard52 = fsm1_out == 2'd1;
wire _guard53 = invoke2_done_out;
wire _guard54 = _guard52 & _guard53;
wire _guard55 = tdcc0_go_out;
wire _guard56 = _guard54 & _guard55;
wire _guard57 = _guard51 | _guard56;
wire _guard58 = fsm1_out == 2'd0;
wire _guard59 = beg_spl_upd1_done_out;
wire _guard60 = _guard58 & _guard59;
wire _guard61 = tdcc0_go_out;
wire _guard62 = _guard60 & _guard61;
wire _guard63 = fsm1_out == 2'd2;
wire _guard64 = fsm1_out == 2'd1;
wire _guard65 = invoke2_done_out;
wire _guard66 = _guard64 & _guard65;
wire _guard67 = tdcc0_go_out;
wire _guard68 = _guard66 & _guard67;
wire _guard69 = pd_out;
wire _guard70 = tdcc_done_out;
wire _guard71 = _guard69 | _guard70;
wire _guard72 = ~_guard71;
wire _guard73 = par0_go_out;
wire _guard74 = _guard72 & _guard73;
wire _guard75 = invoke0_done_out;
wire _guard76 = ~_guard75;
wire _guard77 = fsm2_out == 3'd0;
wire _guard78 = _guard76 & _guard77;
wire _guard79 = tdcc1_go_out;
wire _guard80 = _guard78 & _guard79;
wire _guard81 = fsm0_out == 2'd2;
wire _guard82 = fsm0_out == 2'd0;
wire _guard83 = beg_spl_upd0_done_out;
wire _guard84 = _guard82 & _guard83;
wire _guard85 = tdcc_go_out;
wire _guard86 = _guard84 & _guard85;
wire _guard87 = _guard81 | _guard86;
wire _guard88 = fsm0_out == 2'd1;
wire _guard89 = invoke1_done_out;
wire _guard90 = _guard88 & _guard89;
wire _guard91 = tdcc_go_out;
wire _guard92 = _guard90 & _guard91;
wire _guard93 = _guard87 | _guard92;
wire _guard94 = fsm0_out == 2'd0;
wire _guard95 = beg_spl_upd0_done_out;
wire _guard96 = _guard94 & _guard95;
wire _guard97 = tdcc_go_out;
wire _guard98 = _guard96 & _guard97;
wire _guard99 = fsm0_out == 2'd2;
wire _guard100 = fsm0_out == 2'd1;
wire _guard101 = invoke1_done_out;
wire _guard102 = _guard100 & _guard101;
wire _guard103 = tdcc_go_out;
wire _guard104 = _guard102 & _guard103;
wire _guard105 = fsm2_out == 3'd6;
wire _guard106 = fsm2_out == 3'd0;
wire _guard107 = invoke0_done_out;
wire _guard108 = _guard106 & _guard107;
wire _guard109 = tdcc1_go_out;
wire _guard110 = _guard108 & _guard109;
wire _guard111 = _guard105 | _guard110;
wire _guard112 = fsm2_out == 3'd1;
wire _guard113 = wrapper_early_reset_cond00_done_out;
wire _guard114 = comb_reg_out;
wire _guard115 = _guard113 & _guard114;
wire _guard116 = _guard112 & _guard115;
wire _guard117 = tdcc1_go_out;
wire _guard118 = _guard116 & _guard117;
wire _guard119 = _guard111 | _guard118;
wire _guard120 = fsm2_out == 3'd5;
wire _guard121 = wrapper_early_reset_cond00_done_out;
wire _guard122 = comb_reg_out;
wire _guard123 = _guard121 & _guard122;
wire _guard124 = _guard120 & _guard123;
wire _guard125 = tdcc1_go_out;
wire _guard126 = _guard124 & _guard125;
wire _guard127 = _guard119 | _guard126;
wire _guard128 = fsm2_out == 3'd2;
wire _guard129 = par0_done_out;
wire _guard130 = _guard128 & _guard129;
wire _guard131 = tdcc1_go_out;
wire _guard132 = _guard130 & _guard131;
wire _guard133 = _guard127 | _guard132;
wire _guard134 = fsm2_out == 3'd3;
wire _guard135 = upd2_done_out;
wire _guard136 = _guard134 & _guard135;
wire _guard137 = tdcc1_go_out;
wire _guard138 = _guard136 & _guard137;
wire _guard139 = _guard133 | _guard138;
wire _guard140 = fsm2_out == 3'd4;
wire _guard141 = invoke3_done_out;
wire _guard142 = _guard140 & _guard141;
wire _guard143 = tdcc1_go_out;
wire _guard144 = _guard142 & _guard143;
wire _guard145 = _guard139 | _guard144;
wire _guard146 = fsm2_out == 3'd1;
wire _guard147 = wrapper_early_reset_cond00_done_out;
wire _guard148 = comb_reg_out;
wire _guard149 = ~_guard148;
wire _guard150 = _guard147 & _guard149;
wire _guard151 = _guard146 & _guard150;
wire _guard152 = tdcc1_go_out;
wire _guard153 = _guard151 & _guard152;
wire _guard154 = _guard145 | _guard153;
wire _guard155 = fsm2_out == 3'd5;
wire _guard156 = wrapper_early_reset_cond00_done_out;
wire _guard157 = comb_reg_out;
wire _guard158 = ~_guard157;
wire _guard159 = _guard156 & _guard158;
wire _guard160 = _guard155 & _guard159;
wire _guard161 = tdcc1_go_out;
wire _guard162 = _guard160 & _guard161;
wire _guard163 = _guard154 | _guard162;
wire _guard164 = fsm2_out == 3'd1;
wire _guard165 = wrapper_early_reset_cond00_done_out;
wire _guard166 = comb_reg_out;
wire _guard167 = ~_guard166;
wire _guard168 = _guard165 & _guard167;
wire _guard169 = _guard164 & _guard168;
wire _guard170 = tdcc1_go_out;
wire _guard171 = _guard169 & _guard170;
wire _guard172 = fsm2_out == 3'd5;
wire _guard173 = wrapper_early_reset_cond00_done_out;
wire _guard174 = comb_reg_out;
wire _guard175 = ~_guard174;
wire _guard176 = _guard173 & _guard175;
wire _guard177 = _guard172 & _guard176;
wire _guard178 = tdcc1_go_out;
wire _guard179 = _guard177 & _guard178;
wire _guard180 = _guard171 | _guard179;
wire _guard181 = fsm2_out == 3'd4;
wire _guard182 = invoke3_done_out;
wire _guard183 = _guard181 & _guard182;
wire _guard184 = tdcc1_go_out;
wire _guard185 = _guard183 & _guard184;
wire _guard186 = fsm2_out == 3'd1;
wire _guard187 = wrapper_early_reset_cond00_done_out;
wire _guard188 = comb_reg_out;
wire _guard189 = _guard187 & _guard188;
wire _guard190 = _guard186 & _guard189;
wire _guard191 = tdcc1_go_out;
wire _guard192 = _guard190 & _guard191;
wire _guard193 = fsm2_out == 3'd5;
wire _guard194 = wrapper_early_reset_cond00_done_out;
wire _guard195 = comb_reg_out;
wire _guard196 = _guard194 & _guard195;
wire _guard197 = _guard193 & _guard196;
wire _guard198 = tdcc1_go_out;
wire _guard199 = _guard197 & _guard198;
wire _guard200 = _guard192 | _guard199;
wire _guard201 = fsm2_out == 3'd3;
wire _guard202 = upd2_done_out;
wire _guard203 = _guard201 & _guard202;
wire _guard204 = tdcc1_go_out;
wire _guard205 = _guard203 & _guard204;
wire _guard206 = fsm2_out == 3'd6;
wire _guard207 = fsm2_out == 3'd0;
wire _guard208 = invoke0_done_out;
wire _guard209 = _guard207 & _guard208;
wire _guard210 = tdcc1_go_out;
wire _guard211 = _guard209 & _guard210;
wire _guard212 = fsm2_out == 3'd2;
wire _guard213 = par0_done_out;
wire _guard214 = _guard212 & _guard213;
wire _guard215 = tdcc1_go_out;
wire _guard216 = _guard214 & _guard215;
wire _guard217 = pd0_out;
wire _guard218 = tdcc0_done_out;
wire _guard219 = _guard217 | _guard218;
wire _guard220 = ~_guard219;
wire _guard221 = par0_go_out;
wire _guard222 = _guard220 & _guard221;
wire _guard223 = pd_out;
wire _guard224 = pd0_out;
wire _guard225 = _guard223 & _guard224;
wire _guard226 = invoke1_done_out;
wire _guard227 = ~_guard226;
wire _guard228 = fsm0_out == 2'd1;
wire _guard229 = _guard227 & _guard228;
wire _guard230 = tdcc_go_out;
wire _guard231 = _guard229 & _guard230;
wire _guard232 = beg_spl_upd1_done_out;
wire _guard233 = ~_guard232;
wire _guard234 = fsm1_out == 2'd0;
wire _guard235 = _guard233 & _guard234;
wire _guard236 = tdcc0_go_out;
wire _guard237 = _guard235 & _guard236;
wire _guard238 = early_reset_cond00_go_out;
wire _guard239 = early_reset_cond00_go_out;
wire _guard240 = fsm_out == 1'd0;
wire _guard241 = signal_reg_out;
wire _guard242 = _guard240 & _guard241;
wire _guard243 = fsm_out == 1'd0;
wire _guard244 = signal_reg_out;
wire _guard245 = ~_guard244;
wire _guard246 = _guard243 & _guard245;
wire _guard247 = wrapper_early_reset_cond00_go_out;
wire _guard248 = _guard246 & _guard247;
wire _guard249 = _guard242 | _guard248;
wire _guard250 = fsm_out == 1'd0;
wire _guard251 = signal_reg_out;
wire _guard252 = ~_guard251;
wire _guard253 = _guard250 & _guard252;
wire _guard254 = wrapper_early_reset_cond00_go_out;
wire _guard255 = _guard253 & _guard254;
wire _guard256 = fsm_out == 1'd0;
wire _guard257 = signal_reg_out;
wire _guard258 = _guard256 & _guard257;
wire _guard259 = fsm2_out == 3'd6;
wire _guard260 = invoke2_go_out;
wire _guard261 = invoke2_go_out;
wire _guard262 = pd_out;
wire _guard263 = pd0_out;
wire _guard264 = _guard262 & _guard263;
wire _guard265 = tdcc_done_out;
wire _guard266 = par0_go_out;
wire _guard267 = _guard265 & _guard266;
wire _guard268 = _guard264 | _guard267;
wire _guard269 = tdcc_done_out;
wire _guard270 = par0_go_out;
wire _guard271 = _guard269 & _guard270;
wire _guard272 = pd_out;
wire _guard273 = pd0_out;
wire _guard274 = _guard272 & _guard273;
wire _guard275 = pd_out;
wire _guard276 = pd0_out;
wire _guard277 = _guard275 & _guard276;
wire _guard278 = tdcc0_done_out;
wire _guard279 = par0_go_out;
wire _guard280 = _guard278 & _guard279;
wire _guard281 = _guard277 | _guard280;
wire _guard282 = tdcc0_done_out;
wire _guard283 = par0_go_out;
wire _guard284 = _guard282 & _guard283;
wire _guard285 = pd_out;
wire _guard286 = pd0_out;
wire _guard287 = _guard285 & _guard286;
wire _guard288 = wrapper_early_reset_cond00_done_out;
wire _guard289 = ~_guard288;
wire _guard290 = fsm2_out == 3'd1;
wire _guard291 = _guard289 & _guard290;
wire _guard292 = tdcc1_go_out;
wire _guard293 = _guard291 & _guard292;
wire _guard294 = wrapper_early_reset_cond00_done_out;
wire _guard295 = ~_guard294;
wire _guard296 = fsm2_out == 3'd5;
wire _guard297 = _guard295 & _guard296;
wire _guard298 = tdcc1_go_out;
wire _guard299 = _guard297 & _guard298;
wire _guard300 = _guard293 | _guard299;
wire _guard301 = fsm_out == 1'd0;
wire _guard302 = signal_reg_out;
wire _guard303 = _guard301 & _guard302;
wire _guard304 = fsm0_out == 2'd2;
wire _guard305 = upd2_done_out;
wire _guard306 = ~_guard305;
wire _guard307 = fsm2_out == 3'd3;
wire _guard308 = _guard306 & _guard307;
wire _guard309 = tdcc1_go_out;
wire _guard310 = _guard308 & _guard309;
wire _guard311 = invoke3_done_out;
wire _guard312 = ~_guard311;
wire _guard313 = fsm2_out == 3'd4;
wire _guard314 = _guard312 & _guard313;
wire _guard315 = tdcc1_go_out;
wire _guard316 = _guard314 & _guard315;
wire _guard317 = invoke1_go_out;
wire _guard318 = invoke1_go_out;
wire _guard319 = par0_done_out;
wire _guard320 = ~_guard319;
wire _guard321 = fsm2_out == 3'd2;
wire _guard322 = _guard320 & _guard321;
wire _guard323 = tdcc1_go_out;
wire _guard324 = _guard322 & _guard323;
assign i0_write_en = _guard3;
assign i0_clk = clk;
assign i0_reset = reset;
assign i0_in =
  _guard4 ? add1_out :
  _guard5 ? const0_out :
  'x;
assign upd2_done_in = Sum0_done;
assign early_reset_cond00_go_in = _guard6;
assign add1_left = i0_out;
assign add1_right = const2_out;
assign done = _guard9;
assign B0_write_en = 1'd0;
assign Sum0_addr0 = bit_slice_out;
assign A0_write_en = 1'd0;
assign B0_addr0 = bit_slice_out;
assign B0_content_en = _guard12;
assign A0_addr0 = bit_slice_out;
assign Sum0_write_en = _guard14;
assign Sum0_content_en = _guard15;
assign Sum0_write_data = add0_out;
assign A0_content_en = _guard17;
assign fsm_write_en = _guard18;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard22 ? adder_out :
  _guard25 ? 1'd0 :
  1'd0;
assign adder_left =
  _guard26 ? fsm_out :
  1'd0;
assign adder_right = _guard27;
assign beg_spl_upd0_go_in = _guard33;
assign add0_left = B_read0_0_out;
assign add0_right = A_read0_0_out;
assign invoke2_go_in = _guard41;
assign tdcc0_done_in = _guard42;
assign comb_reg_write_en = _guard43;
assign comb_reg_clk = clk;
assign comb_reg_reset = reset;
assign comb_reg_in =
  _guard44 ? le0_out :
  1'd0;
assign early_reset_cond00_done_in = ud_out;
assign fsm1_write_en = _guard57;
assign fsm1_clk = clk;
assign fsm1_reset = reset;
assign fsm1_in =
  _guard62 ? 2'd1 :
  _guard63 ? 2'd0 :
  _guard68 ? 2'd2 :
  2'd0;
assign tdcc_go_in = _guard74;
assign invoke0_go_in = _guard80;
assign beg_spl_upd0_done_in = A0_done;
assign bit_slice_in = i0_out;
assign fsm0_write_en = _guard93;
assign fsm0_clk = clk;
assign fsm0_reset = reset;
assign fsm0_in =
  _guard98 ? 2'd1 :
  _guard99 ? 2'd0 :
  _guard104 ? 2'd2 :
  2'd0;
assign fsm2_write_en = _guard163;
assign fsm2_clk = clk;
assign fsm2_reset = reset;
assign fsm2_in =
  _guard180 ? 3'd6 :
  _guard185 ? 3'd5 :
  _guard200 ? 3'd2 :
  _guard205 ? 3'd4 :
  _guard206 ? 3'd0 :
  _guard211 ? 3'd1 :
  _guard216 ? 3'd3 :
  3'd0;
assign invoke3_done_in = i0_done;
assign tdcc0_go_in = _guard222;
assign par0_done_in = _guard225;
assign invoke0_done_in = i0_done;
assign invoke1_go_in = _guard231;
assign beg_spl_upd1_go_in = _guard237;
assign le0_left =
  _guard238 ? i0_out :
  4'd0;
assign le0_right =
  _guard239 ? const1_out :
  4'd0;
assign signal_reg_write_en = _guard249;
assign signal_reg_clk = clk;
assign signal_reg_reset = reset;
assign signal_reg_in =
  _guard255 ? 1'd1 :
  _guard258 ? 1'd0 :
  1'd0;
assign invoke2_done_in = B_read0_0_done;
assign beg_spl_upd1_done_in = B0_done;
assign tdcc1_done_in = _guard259;
assign B_read0_0_write_en = _guard260;
assign B_read0_0_clk = clk;
assign B_read0_0_reset = reset;
assign B_read0_0_in = B0_read_data;
assign pd_write_en = _guard268;
assign pd_clk = clk;
assign pd_reset = reset;
assign pd_in =
  _guard271 ? 1'd1 :
  _guard274 ? 1'd0 :
  1'd0;
assign pd0_write_en = _guard281;
assign pd0_clk = clk;
assign pd0_reset = reset;
assign pd0_in =
  _guard284 ? 1'd1 :
  _guard287 ? 1'd0 :
  1'd0;
assign wrapper_early_reset_cond00_go_in = _guard300;
assign wrapper_early_reset_cond00_done_in = _guard303;
assign tdcc_done_in = _guard304;
assign upd2_go_in = _guard310;
assign invoke3_go_in = _guard316;
assign invoke1_done_in = A_read0_0_done;
assign tdcc1_go_in = go;
assign A_read0_0_write_en = _guard317;
assign A_read0_0_clk = clk;
assign A_read0_0_reset = reset;
assign A_read0_0_in = A0_read_data;
assign par0_go_in = _guard324;
// COMPONENT END: main
endmodule
