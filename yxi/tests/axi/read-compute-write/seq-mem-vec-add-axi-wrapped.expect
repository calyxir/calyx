/**
Implements a memory with sequential reads and writes.
- Both reads and writes take one cycle to perform.
- Attempting to read and write at the same time is an error.
- The out signal is registered to the last value requested by the read_en signal.
- The out signal is undefined once write_en is asserted.
*/
module seq_mem_d1 #(
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

module seq_mem_d2 #(
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

  seq_mem_d1 #(.WIDTH(WIDTH), .SIZE(D0_SIZE * D1_SIZE), .IDX_SIZE(D0_IDX_SIZE+D1_IDX_SIZE)) mem
     (.clk(clk), .reset(reset), .addr0(addr),
    .content_en(content_en), .read_data(read_data), .write_data(write_data), .write_en(write_en),
    .done(done));
endmodule

module seq_mem_d3 #(
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

  seq_mem_d1 #(.WIDTH(WIDTH), .SIZE(D0_SIZE * D1_SIZE * D2_SIZE), .IDX_SIZE(D0_IDX_SIZE+D1_IDX_SIZE+D2_IDX_SIZE)) mem
     (.clk(clk), .reset(reset), .addr0(addr),
    .content_en(content_en), .read_data(read_data), .write_data(write_data), .write_en(write_en),
    .done(done));
endmodule

module seq_mem_d4 #(
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

  seq_mem_d1 #(.WIDTH(WIDTH), .SIZE(D0_SIZE * D1_SIZE * D2_SIZE * D3_SIZE), .IDX_SIZE(D0_IDX_SIZE+D1_IDX_SIZE+D2_IDX_SIZE+D3_IDX_SIZE)) mem
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

module m_ar_channel(
  input logic ARESETn,
  input logic ARREADY,
  output logic ARVALID,
  output logic [63:0] ARADDR,
  output logic [2:0] ARSIZE,
  output logic [7:0] ARLEN,
  output logic [1:0] ARBURST,
  output logic [2:0] ARPROT,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done,
  output logic [63:0] curr_addr_axi_in,
  output logic curr_addr_axi_write_en,
  input logic [63:0] curr_addr_axi_out,
  input logic curr_addr_axi_done
);
// COMPONENT START: m_ar_channel
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
logic [7:0] arlen_in;
logic arlen_write_en;
logic arlen_clk;
logic arlen_reset;
logic [7:0] arlen_out;
logic arlen_done;
logic [31:0] txn_n_out;
logic [31:0] txn_count_in;
logic txn_count_write_en;
logic txn_count_clk;
logic txn_count_reset;
logic [31:0] txn_count_out;
logic txn_count_done;
logic [31:0] txn_adder_left;
logic [31:0] txn_adder_right;
logic [31:0] txn_adder_out;
logic bt_reg_in;
logic bt_reg_write_en;
logic bt_reg_clk;
logic bt_reg_reset;
logic bt_reg_out;
logic bt_reg_done;
logic [31:0] perform_reads_left;
logic [31:0] perform_reads_right;
logic perform_reads_out;
logic comb_reg_in;
logic comb_reg_write_en;
logic comb_reg_clk;
logic comb_reg_reset;
logic comb_reg_out;
logic comb_reg_done;
logic ud_out;
logic ud1_out;
logic ud2_out;
logic ud3_out;
logic signal_reg_in;
logic signal_reg_write_en;
logic signal_reg_clk;
logic signal_reg_reset;
logic signal_reg_out;
logic signal_reg_done;
logic signal_reg0_in;
logic signal_reg0_write_en;
logic signal_reg0_clk;
logic signal_reg0_reset;
logic signal_reg0_out;
logic signal_reg0_done;
logic signal_reg1_in;
logic signal_reg1_write_en;
logic signal_reg1_clk;
logic signal_reg1_reset;
logic signal_reg1_out;
logic signal_reg1_done;
logic signal_reg2_in;
logic signal_reg2_write_en;
logic signal_reg2_clk;
logic signal_reg2_reset;
logic signal_reg2_out;
logic signal_reg2_done;
logic [2:0] fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic [2:0] fsm_out;
logic fsm_done;
logic do_ar_transfer_go_in;
logic do_ar_transfer_go_out;
logic do_ar_transfer_done_in;
logic do_ar_transfer_done_out;
logic early_reset_static_par_thread_go_in;
logic early_reset_static_par_thread_go_out;
logic early_reset_static_par_thread_done_in;
logic early_reset_static_par_thread_done_out;
logic early_reset_static_par_thread0_go_in;
logic early_reset_static_par_thread0_go_out;
logic early_reset_static_par_thread0_done_in;
logic early_reset_static_par_thread0_done_out;
logic early_reset_static_par_thread1_go_in;
logic early_reset_static_par_thread1_go_out;
logic early_reset_static_par_thread1_done_in;
logic early_reset_static_par_thread1_done_out;
logic early_reset_perform_reads_group00_go_in;
logic early_reset_perform_reads_group00_go_out;
logic early_reset_perform_reads_group00_done_in;
logic early_reset_perform_reads_group00_done_out;
logic wrapper_early_reset_static_par_thread_go_in;
logic wrapper_early_reset_static_par_thread_go_out;
logic wrapper_early_reset_static_par_thread_done_in;
logic wrapper_early_reset_static_par_thread_done_out;
logic wrapper_early_reset_perform_reads_group00_go_in;
logic wrapper_early_reset_perform_reads_group00_go_out;
logic wrapper_early_reset_perform_reads_group00_done_in;
logic wrapper_early_reset_perform_reads_group00_done_out;
logic wrapper_early_reset_static_par_thread0_go_in;
logic wrapper_early_reset_static_par_thread0_go_out;
logic wrapper_early_reset_static_par_thread0_done_in;
logic wrapper_early_reset_static_par_thread0_done_out;
logic wrapper_early_reset_static_par_thread1_go_in;
logic wrapper_early_reset_static_par_thread1_go_out;
logic wrapper_early_reset_static_par_thread1_done_in;
logic wrapper_early_reset_static_par_thread1_done_out;
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
    .WIDTH(8)
) arlen (
    .clk(arlen_clk),
    .done(arlen_done),
    .in(arlen_in),
    .out(arlen_out),
    .reset(arlen_reset),
    .write_en(arlen_write_en)
);
std_const # (
    .VALUE(32'd1),
    .WIDTH(32)
) txn_n (
    .out(txn_n_out)
);
std_reg # (
    .WIDTH(32)
) txn_count (
    .clk(txn_count_clk),
    .done(txn_count_done),
    .in(txn_count_in),
    .out(txn_count_out),
    .reset(txn_count_reset),
    .write_en(txn_count_write_en)
);
std_add # (
    .WIDTH(32)
) txn_adder (
    .left(txn_adder_left),
    .out(txn_adder_out),
    .right(txn_adder_right)
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
std_neq # (
    .WIDTH(32)
) perform_reads (
    .left(perform_reads_left),
    .out(perform_reads_out),
    .right(perform_reads_right)
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
undef # (
    .WIDTH(1)
) ud (
    .out(ud_out)
);
undef # (
    .WIDTH(1)
) ud1 (
    .out(ud1_out)
);
undef # (
    .WIDTH(1)
) ud2 (
    .out(ud2_out)
);
undef # (
    .WIDTH(1)
) ud3 (
    .out(ud3_out)
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
    .WIDTH(1)
) signal_reg0 (
    .clk(signal_reg0_clk),
    .done(signal_reg0_done),
    .in(signal_reg0_in),
    .out(signal_reg0_out),
    .reset(signal_reg0_reset),
    .write_en(signal_reg0_write_en)
);
std_reg # (
    .WIDTH(1)
) signal_reg1 (
    .clk(signal_reg1_clk),
    .done(signal_reg1_done),
    .in(signal_reg1_in),
    .out(signal_reg1_out),
    .reset(signal_reg1_reset),
    .write_en(signal_reg1_write_en)
);
std_reg # (
    .WIDTH(1)
) signal_reg2 (
    .clk(signal_reg2_clk),
    .done(signal_reg2_done),
    .in(signal_reg2_in),
    .out(signal_reg2_out),
    .reset(signal_reg2_reset),
    .write_en(signal_reg2_write_en)
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
) early_reset_static_par_thread_go (
    .in(early_reset_static_par_thread_go_in),
    .out(early_reset_static_par_thread_go_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_static_par_thread_done (
    .in(early_reset_static_par_thread_done_in),
    .out(early_reset_static_par_thread_done_out)
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
) early_reset_static_par_thread1_go (
    .in(early_reset_static_par_thread1_go_in),
    .out(early_reset_static_par_thread1_go_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_static_par_thread1_done (
    .in(early_reset_static_par_thread1_done_in),
    .out(early_reset_static_par_thread1_done_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_perform_reads_group00_go (
    .in(early_reset_perform_reads_group00_go_in),
    .out(early_reset_perform_reads_group00_go_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_perform_reads_group00_done (
    .in(early_reset_perform_reads_group00_done_in),
    .out(early_reset_perform_reads_group00_done_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_static_par_thread_go (
    .in(wrapper_early_reset_static_par_thread_go_in),
    .out(wrapper_early_reset_static_par_thread_go_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_static_par_thread_done (
    .in(wrapper_early_reset_static_par_thread_done_in),
    .out(wrapper_early_reset_static_par_thread_done_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_perform_reads_group00_go (
    .in(wrapper_early_reset_perform_reads_group00_go_in),
    .out(wrapper_early_reset_perform_reads_group00_go_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_perform_reads_group00_done (
    .in(wrapper_early_reset_perform_reads_group00_done_in),
    .out(wrapper_early_reset_perform_reads_group00_done_out)
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
) wrapper_early_reset_static_par_thread1_go (
    .in(wrapper_early_reset_static_par_thread1_go_in),
    .out(wrapper_early_reset_static_par_thread1_go_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_static_par_thread1_done (
    .in(wrapper_early_reset_static_par_thread1_done_in),
    .out(wrapper_early_reset_static_par_thread1_done_out)
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
wire _guard1 = signal_reg1_out;
wire _guard2 = _guard0 & _guard0;
wire _guard3 = signal_reg1_out;
wire _guard4 = ~_guard3;
wire _guard5 = _guard2 & _guard4;
wire _guard6 = wrapper_early_reset_static_par_thread0_go_out;
wire _guard7 = _guard5 & _guard6;
wire _guard8 = _guard1 | _guard7;
wire _guard9 = _guard0 & _guard0;
wire _guard10 = signal_reg1_out;
wire _guard11 = ~_guard10;
wire _guard12 = _guard9 & _guard11;
wire _guard13 = wrapper_early_reset_static_par_thread0_go_out;
wire _guard14 = _guard12 & _guard13;
wire _guard15 = signal_reg1_out;
wire _guard16 = do_ar_transfer_done_out;
wire _guard17 = ~_guard16;
wire _guard18 = fsm_out == 3'd3;
wire _guard19 = _guard17 & _guard18;
wire _guard20 = tdcc_go_out;
wire _guard21 = _guard19 & _guard20;
wire _guard22 = tdcc_done_out;
wire _guard23 = do_ar_transfer_go_out;
wire _guard24 = do_ar_transfer_go_out;
wire _guard25 = do_ar_transfer_go_out;
wire _guard26 = do_ar_transfer_go_out;
wire _guard27 = do_ar_transfer_go_out;
wire _guard28 = fsm_out == 3'd6;
wire _guard29 = fsm_out == 3'd0;
wire _guard30 = wrapper_early_reset_static_par_thread_done_out;
wire _guard31 = _guard29 & _guard30;
wire _guard32 = tdcc_go_out;
wire _guard33 = _guard31 & _guard32;
wire _guard34 = _guard28 | _guard33;
wire _guard35 = fsm_out == 3'd1;
wire _guard36 = wrapper_early_reset_perform_reads_group00_done_out;
wire _guard37 = comb_reg_out;
wire _guard38 = _guard36 & _guard37;
wire _guard39 = _guard35 & _guard38;
wire _guard40 = tdcc_go_out;
wire _guard41 = _guard39 & _guard40;
wire _guard42 = _guard34 | _guard41;
wire _guard43 = fsm_out == 3'd5;
wire _guard44 = wrapper_early_reset_perform_reads_group00_done_out;
wire _guard45 = comb_reg_out;
wire _guard46 = _guard44 & _guard45;
wire _guard47 = _guard43 & _guard46;
wire _guard48 = tdcc_go_out;
wire _guard49 = _guard47 & _guard48;
wire _guard50 = _guard42 | _guard49;
wire _guard51 = fsm_out == 3'd2;
wire _guard52 = wrapper_early_reset_static_par_thread0_done_out;
wire _guard53 = _guard51 & _guard52;
wire _guard54 = tdcc_go_out;
wire _guard55 = _guard53 & _guard54;
wire _guard56 = _guard50 | _guard55;
wire _guard57 = fsm_out == 3'd3;
wire _guard58 = do_ar_transfer_done_out;
wire _guard59 = _guard57 & _guard58;
wire _guard60 = tdcc_go_out;
wire _guard61 = _guard59 & _guard60;
wire _guard62 = _guard56 | _guard61;
wire _guard63 = fsm_out == 3'd4;
wire _guard64 = wrapper_early_reset_static_par_thread1_done_out;
wire _guard65 = _guard63 & _guard64;
wire _guard66 = tdcc_go_out;
wire _guard67 = _guard65 & _guard66;
wire _guard68 = _guard62 | _guard67;
wire _guard69 = fsm_out == 3'd1;
wire _guard70 = wrapper_early_reset_perform_reads_group00_done_out;
wire _guard71 = comb_reg_out;
wire _guard72 = ~_guard71;
wire _guard73 = _guard70 & _guard72;
wire _guard74 = _guard69 & _guard73;
wire _guard75 = tdcc_go_out;
wire _guard76 = _guard74 & _guard75;
wire _guard77 = _guard68 | _guard76;
wire _guard78 = fsm_out == 3'd5;
wire _guard79 = wrapper_early_reset_perform_reads_group00_done_out;
wire _guard80 = comb_reg_out;
wire _guard81 = ~_guard80;
wire _guard82 = _guard79 & _guard81;
wire _guard83 = _guard78 & _guard82;
wire _guard84 = tdcc_go_out;
wire _guard85 = _guard83 & _guard84;
wire _guard86 = _guard77 | _guard85;
wire _guard87 = fsm_out == 3'd1;
wire _guard88 = wrapper_early_reset_perform_reads_group00_done_out;
wire _guard89 = comb_reg_out;
wire _guard90 = ~_guard89;
wire _guard91 = _guard88 & _guard90;
wire _guard92 = _guard87 & _guard91;
wire _guard93 = tdcc_go_out;
wire _guard94 = _guard92 & _guard93;
wire _guard95 = fsm_out == 3'd5;
wire _guard96 = wrapper_early_reset_perform_reads_group00_done_out;
wire _guard97 = comb_reg_out;
wire _guard98 = ~_guard97;
wire _guard99 = _guard96 & _guard98;
wire _guard100 = _guard95 & _guard99;
wire _guard101 = tdcc_go_out;
wire _guard102 = _guard100 & _guard101;
wire _guard103 = _guard94 | _guard102;
wire _guard104 = fsm_out == 3'd4;
wire _guard105 = wrapper_early_reset_static_par_thread1_done_out;
wire _guard106 = _guard104 & _guard105;
wire _guard107 = tdcc_go_out;
wire _guard108 = _guard106 & _guard107;
wire _guard109 = fsm_out == 3'd1;
wire _guard110 = wrapper_early_reset_perform_reads_group00_done_out;
wire _guard111 = comb_reg_out;
wire _guard112 = _guard110 & _guard111;
wire _guard113 = _guard109 & _guard112;
wire _guard114 = tdcc_go_out;
wire _guard115 = _guard113 & _guard114;
wire _guard116 = fsm_out == 3'd5;
wire _guard117 = wrapper_early_reset_perform_reads_group00_done_out;
wire _guard118 = comb_reg_out;
wire _guard119 = _guard117 & _guard118;
wire _guard120 = _guard116 & _guard119;
wire _guard121 = tdcc_go_out;
wire _guard122 = _guard120 & _guard121;
wire _guard123 = _guard115 | _guard122;
wire _guard124 = fsm_out == 3'd3;
wire _guard125 = do_ar_transfer_done_out;
wire _guard126 = _guard124 & _guard125;
wire _guard127 = tdcc_go_out;
wire _guard128 = _guard126 & _guard127;
wire _guard129 = fsm_out == 3'd0;
wire _guard130 = wrapper_early_reset_static_par_thread_done_out;
wire _guard131 = _guard129 & _guard130;
wire _guard132 = tdcc_go_out;
wire _guard133 = _guard131 & _guard132;
wire _guard134 = fsm_out == 3'd6;
wire _guard135 = fsm_out == 3'd2;
wire _guard136 = wrapper_early_reset_static_par_thread0_done_out;
wire _guard137 = _guard135 & _guard136;
wire _guard138 = tdcc_go_out;
wire _guard139 = _guard137 & _guard138;
wire _guard140 = wrapper_early_reset_static_par_thread1_go_out;
wire _guard141 = ar_handshake_occurred_out;
wire _guard142 = ~_guard141;
wire _guard143 = do_ar_transfer_go_out;
wire _guard144 = _guard142 & _guard143;
wire _guard145 = early_reset_static_par_thread0_go_out;
wire _guard146 = _guard144 | _guard145;
wire _guard147 = arvalid_out;
wire _guard148 = ARREADY;
wire _guard149 = _guard147 & _guard148;
wire _guard150 = do_ar_transfer_go_out;
wire _guard151 = _guard149 & _guard150;
wire _guard152 = early_reset_static_par_thread0_go_out;
wire _guard153 = signal_reg0_out;
wire _guard154 = _guard0 & _guard0;
wire _guard155 = signal_reg0_out;
wire _guard156 = ~_guard155;
wire _guard157 = _guard154 & _guard156;
wire _guard158 = wrapper_early_reset_perform_reads_group00_go_out;
wire _guard159 = _guard157 & _guard158;
wire _guard160 = _guard153 | _guard159;
wire _guard161 = _guard0 & _guard0;
wire _guard162 = signal_reg0_out;
wire _guard163 = ~_guard162;
wire _guard164 = _guard161 & _guard163;
wire _guard165 = wrapper_early_reset_perform_reads_group00_go_out;
wire _guard166 = _guard164 & _guard165;
wire _guard167 = signal_reg0_out;
wire _guard168 = wrapper_early_reset_static_par_thread_done_out;
wire _guard169 = ~_guard168;
wire _guard170 = fsm_out == 3'd0;
wire _guard171 = _guard169 & _guard170;
wire _guard172 = tdcc_go_out;
wire _guard173 = _guard171 & _guard172;
wire _guard174 = early_reset_perform_reads_group00_go_out;
wire _guard175 = early_reset_perform_reads_group00_go_out;
wire _guard176 = early_reset_perform_reads_group00_go_out;
wire _guard177 = early_reset_perform_reads_group00_go_out;
wire _guard178 = early_reset_static_par_thread_go_out;
wire _guard179 = early_reset_static_par_thread_go_out;
wire _guard180 = wrapper_early_reset_perform_reads_group00_done_out;
wire _guard181 = ~_guard180;
wire _guard182 = fsm_out == 3'd1;
wire _guard183 = _guard181 & _guard182;
wire _guard184 = tdcc_go_out;
wire _guard185 = _guard183 & _guard184;
wire _guard186 = wrapper_early_reset_perform_reads_group00_done_out;
wire _guard187 = ~_guard186;
wire _guard188 = fsm_out == 3'd5;
wire _guard189 = _guard187 & _guard188;
wire _guard190 = tdcc_go_out;
wire _guard191 = _guard189 & _guard190;
wire _guard192 = _guard185 | _guard191;
wire _guard193 = early_reset_static_par_thread_go_out;
wire _guard194 = early_reset_static_par_thread1_go_out;
wire _guard195 = _guard193 | _guard194;
wire _guard196 = early_reset_static_par_thread_go_out;
wire _guard197 = early_reset_static_par_thread1_go_out;
wire _guard198 = signal_reg0_out;
wire _guard199 = wrapper_early_reset_perform_reads_group00_go_out;
wire _guard200 = signal_reg2_out;
wire _guard201 = _guard0 & _guard0;
wire _guard202 = signal_reg2_out;
wire _guard203 = ~_guard202;
wire _guard204 = _guard201 & _guard203;
wire _guard205 = wrapper_early_reset_static_par_thread1_go_out;
wire _guard206 = _guard204 & _guard205;
wire _guard207 = _guard200 | _guard206;
wire _guard208 = _guard0 & _guard0;
wire _guard209 = signal_reg2_out;
wire _guard210 = ~_guard209;
wire _guard211 = _guard208 & _guard210;
wire _guard212 = wrapper_early_reset_static_par_thread1_go_out;
wire _guard213 = _guard211 & _guard212;
wire _guard214 = signal_reg2_out;
wire _guard215 = wrapper_early_reset_static_par_thread0_done_out;
wire _guard216 = ~_guard215;
wire _guard217 = fsm_out == 3'd2;
wire _guard218 = _guard216 & _guard217;
wire _guard219 = tdcc_go_out;
wire _guard220 = _guard218 & _guard219;
wire _guard221 = do_ar_transfer_go_out;
wire _guard222 = early_reset_static_par_thread0_go_out;
wire _guard223 = _guard221 | _guard222;
wire _guard224 = ARREADY;
wire _guard225 = arvalid_out;
wire _guard226 = _guard224 & _guard225;
wire _guard227 = do_ar_transfer_go_out;
wire _guard228 = _guard226 & _guard227;
wire _guard229 = ARREADY;
wire _guard230 = arvalid_out;
wire _guard231 = _guard229 & _guard230;
wire _guard232 = ~_guard231;
wire _guard233 = do_ar_transfer_go_out;
wire _guard234 = _guard232 & _guard233;
wire _guard235 = early_reset_static_par_thread0_go_out;
wire _guard236 = _guard234 | _guard235;
wire _guard237 = signal_reg_out;
wire _guard238 = _guard0 & _guard0;
wire _guard239 = signal_reg_out;
wire _guard240 = ~_guard239;
wire _guard241 = _guard238 & _guard240;
wire _guard242 = wrapper_early_reset_static_par_thread_go_out;
wire _guard243 = _guard241 & _guard242;
wire _guard244 = _guard237 | _guard243;
wire _guard245 = _guard0 & _guard0;
wire _guard246 = signal_reg_out;
wire _guard247 = ~_guard246;
wire _guard248 = _guard245 & _guard247;
wire _guard249 = wrapper_early_reset_static_par_thread_go_out;
wire _guard250 = _guard248 & _guard249;
wire _guard251 = signal_reg_out;
wire _guard252 = wrapper_early_reset_static_par_thread_go_out;
wire _guard253 = signal_reg1_out;
wire _guard254 = wrapper_early_reset_static_par_thread1_done_out;
wire _guard255 = ~_guard254;
wire _guard256 = fsm_out == 3'd4;
wire _guard257 = _guard255 & _guard256;
wire _guard258 = tdcc_go_out;
wire _guard259 = _guard257 & _guard258;
wire _guard260 = wrapper_early_reset_static_par_thread0_go_out;
wire _guard261 = signal_reg_out;
wire _guard262 = do_ar_transfer_go_out;
wire _guard263 = early_reset_static_par_thread1_go_out;
wire _guard264 = _guard262 | _guard263;
wire _guard265 = arvalid_out;
wire _guard266 = ARREADY;
wire _guard267 = _guard265 & _guard266;
wire _guard268 = ~_guard267;
wire _guard269 = ar_handshake_occurred_out;
wire _guard270 = ~_guard269;
wire _guard271 = _guard268 & _guard270;
wire _guard272 = do_ar_transfer_go_out;
wire _guard273 = _guard271 & _guard272;
wire _guard274 = arvalid_out;
wire _guard275 = ARREADY;
wire _guard276 = _guard274 & _guard275;
wire _guard277 = ar_handshake_occurred_out;
wire _guard278 = _guard276 | _guard277;
wire _guard279 = do_ar_transfer_go_out;
wire _guard280 = _guard278 & _guard279;
wire _guard281 = early_reset_static_par_thread1_go_out;
wire _guard282 = _guard280 | _guard281;
wire _guard283 = early_reset_static_par_thread1_go_out;
wire _guard284 = early_reset_static_par_thread1_go_out;
wire _guard285 = fsm_out == 3'd6;
wire _guard286 = signal_reg2_out;
assign signal_reg1_write_en = _guard8;
assign signal_reg1_clk = clk;
assign signal_reg1_reset = reset;
assign signal_reg1_in =
  _guard14 ? 1'd1 :
  _guard15 ? 1'd0 :
  1'd0;
assign do_ar_transfer_go_in = _guard21;
assign done = _guard22;
assign ARPROT =
  _guard23 ? 3'd6 :
  3'd0;
assign ARSIZE =
  _guard24 ? 3'd2 :
  3'd0;
assign ARLEN =
  _guard25 ? arlen_out :
  8'd0;
assign ARADDR =
  _guard26 ? curr_addr_axi_out :
  64'd0;
assign ARBURST =
  _guard27 ? 2'd1 :
  2'd0;
assign ARVALID = arvalid_out;
assign fsm_write_en = _guard86;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard103 ? 3'd6 :
  _guard108 ? 3'd5 :
  _guard123 ? 3'd2 :
  _guard128 ? 3'd4 :
  _guard133 ? 3'd1 :
  _guard134 ? 3'd0 :
  _guard139 ? 3'd3 :
  3'd0;
assign early_reset_static_par_thread1_go_in = _guard140;
assign ar_handshake_occurred_write_en = _guard146;
assign ar_handshake_occurred_clk = clk;
assign ar_handshake_occurred_reset = reset;
assign ar_handshake_occurred_in =
  _guard151 ? 1'd1 :
  _guard152 ? 1'd0 :
  'x;
assign signal_reg0_write_en = _guard160;
assign signal_reg0_clk = clk;
assign signal_reg0_reset = reset;
assign signal_reg0_in =
  _guard166 ? 1'd1 :
  _guard167 ? 1'd0 :
  1'd0;
assign wrapper_early_reset_static_par_thread_go_in = _guard173;
assign comb_reg_write_en = _guard174;
assign comb_reg_clk = clk;
assign comb_reg_reset = reset;
assign comb_reg_in =
  _guard175 ? perform_reads_out :
  1'd0;
assign early_reset_static_par_thread0_done_in = ud1_out;
assign perform_reads_left =
  _guard176 ? txn_count_out :
  32'd0;
assign perform_reads_right =
  _guard177 ? txn_n_out :
  32'd0;
assign arlen_write_en = _guard178;
assign arlen_clk = clk;
assign arlen_reset = reset;
assign arlen_in = 8'd7;
assign wrapper_early_reset_perform_reads_group00_go_in = _guard192;
assign txn_count_write_en = _guard195;
assign txn_count_clk = clk;
assign txn_count_reset = reset;
assign txn_count_in =
  _guard196 ? 32'd0 :
  _guard197 ? txn_adder_out :
  'x;
assign wrapper_early_reset_perform_reads_group00_done_in = _guard198;
assign tdcc_go_in = go;
assign early_reset_static_par_thread1_done_in = ud2_out;
assign early_reset_perform_reads_group00_go_in = _guard199;
assign signal_reg2_write_en = _guard207;
assign signal_reg2_clk = clk;
assign signal_reg2_reset = reset;
assign signal_reg2_in =
  _guard213 ? 1'd1 :
  _guard214 ? 1'd0 :
  1'd0;
assign wrapper_early_reset_static_par_thread0_go_in = _guard220;
assign bt_reg_write_en = _guard223;
assign bt_reg_clk = clk;
assign bt_reg_reset = reset;
assign bt_reg_in =
  _guard228 ? 1'd1 :
  _guard236 ? 1'd0 :
  'x;
assign signal_reg_write_en = _guard244;
assign signal_reg_clk = clk;
assign signal_reg_reset = reset;
assign signal_reg_in =
  _guard250 ? 1'd1 :
  _guard251 ? 1'd0 :
  1'd0;
assign early_reset_static_par_thread_go_in = _guard252;
assign wrapper_early_reset_static_par_thread0_done_in = _guard253;
assign wrapper_early_reset_static_par_thread1_go_in = _guard259;
assign early_reset_static_par_thread0_go_in = _guard260;
assign wrapper_early_reset_static_par_thread_done_in = _guard261;
assign arvalid_write_en = _guard264;
assign arvalid_clk = clk;
assign arvalid_reset = reset;
assign arvalid_in =
  _guard273 ? 1'd1 :
  _guard282 ? 1'd0 :
  'x;
assign txn_adder_left = txn_count_out;
assign txn_adder_right = 32'd1;
assign tdcc_done_in = _guard285;
assign early_reset_perform_reads_group00_done_in = ud3_out;
assign wrapper_early_reset_static_par_thread1_done_in = _guard286;
assign early_reset_static_par_thread_done_in = ud_out;
assign do_ar_transfer_done_in = bt_reg_out;
// COMPONENT END: m_ar_channel
endmodule
module m_aw_channel(
  input logic ARESETn,
  input logic AWREADY,
  output logic AWVALID,
  output logic [63:0] AWADDR,
  output logic [2:0] AWSIZE,
  output logic [7:0] AWLEN,
  output logic [1:0] AWBURST,
  output logic [2:0] AWPROT,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done,
  output logic [63:0] curr_addr_axi_in,
  output logic curr_addr_axi_write_en,
  input logic [63:0] curr_addr_axi_out,
  input logic curr_addr_axi_done,
  output logic [7:0] max_transfers_in,
  output logic max_transfers_write_en,
  input logic [7:0] max_transfers_out,
  input logic max_transfers_done
);
// COMPONENT START: m_aw_channel
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
logic [7:0] awlen_in;
logic awlen_write_en;
logic awlen_clk;
logic awlen_reset;
logic [7:0] awlen_out;
logic awlen_done;
logic [31:0] txn_n_out;
logic [31:0] txn_count_in;
logic txn_count_write_en;
logic txn_count_clk;
logic txn_count_reset;
logic [31:0] txn_count_out;
logic txn_count_done;
logic [31:0] txn_adder_left;
logic [31:0] txn_adder_right;
logic [31:0] txn_adder_out;
logic bt_reg_in;
logic bt_reg_write_en;
logic bt_reg_clk;
logic bt_reg_reset;
logic bt_reg_out;
logic bt_reg_done;
logic [31:0] perform_writes_left;
logic [31:0] perform_writes_right;
logic perform_writes_out;
logic comb_reg_in;
logic comb_reg_write_en;
logic comb_reg_clk;
logic comb_reg_reset;
logic comb_reg_out;
logic comb_reg_done;
logic ud_out;
logic ud1_out;
logic ud2_out;
logic ud3_out;
logic signal_reg_in;
logic signal_reg_write_en;
logic signal_reg_clk;
logic signal_reg_reset;
logic signal_reg_out;
logic signal_reg_done;
logic signal_reg0_in;
logic signal_reg0_write_en;
logic signal_reg0_clk;
logic signal_reg0_reset;
logic signal_reg0_out;
logic signal_reg0_done;
logic signal_reg1_in;
logic signal_reg1_write_en;
logic signal_reg1_clk;
logic signal_reg1_reset;
logic signal_reg1_out;
logic signal_reg1_done;
logic signal_reg2_in;
logic signal_reg2_write_en;
logic signal_reg2_clk;
logic signal_reg2_reset;
logic signal_reg2_out;
logic signal_reg2_done;
logic [2:0] fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic [2:0] fsm_out;
logic fsm_done;
logic do_aw_transfer_go_in;
logic do_aw_transfer_go_out;
logic do_aw_transfer_done_in;
logic do_aw_transfer_done_out;
logic early_reset_static_par_thread_go_in;
logic early_reset_static_par_thread_go_out;
logic early_reset_static_par_thread_done_in;
logic early_reset_static_par_thread_done_out;
logic early_reset_static_par_thread0_go_in;
logic early_reset_static_par_thread0_go_out;
logic early_reset_static_par_thread0_done_in;
logic early_reset_static_par_thread0_done_out;
logic early_reset_static_par_thread1_go_in;
logic early_reset_static_par_thread1_go_out;
logic early_reset_static_par_thread1_done_in;
logic early_reset_static_par_thread1_done_out;
logic early_reset_perform_writes_group00_go_in;
logic early_reset_perform_writes_group00_go_out;
logic early_reset_perform_writes_group00_done_in;
logic early_reset_perform_writes_group00_done_out;
logic wrapper_early_reset_static_par_thread_go_in;
logic wrapper_early_reset_static_par_thread_go_out;
logic wrapper_early_reset_static_par_thread_done_in;
logic wrapper_early_reset_static_par_thread_done_out;
logic wrapper_early_reset_perform_writes_group00_go_in;
logic wrapper_early_reset_perform_writes_group00_go_out;
logic wrapper_early_reset_perform_writes_group00_done_in;
logic wrapper_early_reset_perform_writes_group00_done_out;
logic wrapper_early_reset_static_par_thread0_go_in;
logic wrapper_early_reset_static_par_thread0_go_out;
logic wrapper_early_reset_static_par_thread0_done_in;
logic wrapper_early_reset_static_par_thread0_done_out;
logic wrapper_early_reset_static_par_thread1_go_in;
logic wrapper_early_reset_static_par_thread1_go_out;
logic wrapper_early_reset_static_par_thread1_done_in;
logic wrapper_early_reset_static_par_thread1_done_out;
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
    .WIDTH(8)
) awlen (
    .clk(awlen_clk),
    .done(awlen_done),
    .in(awlen_in),
    .out(awlen_out),
    .reset(awlen_reset),
    .write_en(awlen_write_en)
);
std_const # (
    .VALUE(32'd1),
    .WIDTH(32)
) txn_n (
    .out(txn_n_out)
);
std_reg # (
    .WIDTH(32)
) txn_count (
    .clk(txn_count_clk),
    .done(txn_count_done),
    .in(txn_count_in),
    .out(txn_count_out),
    .reset(txn_count_reset),
    .write_en(txn_count_write_en)
);
std_add # (
    .WIDTH(32)
) txn_adder (
    .left(txn_adder_left),
    .out(txn_adder_out),
    .right(txn_adder_right)
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
std_neq # (
    .WIDTH(32)
) perform_writes (
    .left(perform_writes_left),
    .out(perform_writes_out),
    .right(perform_writes_right)
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
undef # (
    .WIDTH(1)
) ud (
    .out(ud_out)
);
undef # (
    .WIDTH(1)
) ud1 (
    .out(ud1_out)
);
undef # (
    .WIDTH(1)
) ud2 (
    .out(ud2_out)
);
undef # (
    .WIDTH(1)
) ud3 (
    .out(ud3_out)
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
    .WIDTH(1)
) signal_reg0 (
    .clk(signal_reg0_clk),
    .done(signal_reg0_done),
    .in(signal_reg0_in),
    .out(signal_reg0_out),
    .reset(signal_reg0_reset),
    .write_en(signal_reg0_write_en)
);
std_reg # (
    .WIDTH(1)
) signal_reg1 (
    .clk(signal_reg1_clk),
    .done(signal_reg1_done),
    .in(signal_reg1_in),
    .out(signal_reg1_out),
    .reset(signal_reg1_reset),
    .write_en(signal_reg1_write_en)
);
std_reg # (
    .WIDTH(1)
) signal_reg2 (
    .clk(signal_reg2_clk),
    .done(signal_reg2_done),
    .in(signal_reg2_in),
    .out(signal_reg2_out),
    .reset(signal_reg2_reset),
    .write_en(signal_reg2_write_en)
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
) early_reset_static_par_thread_go (
    .in(early_reset_static_par_thread_go_in),
    .out(early_reset_static_par_thread_go_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_static_par_thread_done (
    .in(early_reset_static_par_thread_done_in),
    .out(early_reset_static_par_thread_done_out)
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
) early_reset_static_par_thread1_go (
    .in(early_reset_static_par_thread1_go_in),
    .out(early_reset_static_par_thread1_go_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_static_par_thread1_done (
    .in(early_reset_static_par_thread1_done_in),
    .out(early_reset_static_par_thread1_done_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_perform_writes_group00_go (
    .in(early_reset_perform_writes_group00_go_in),
    .out(early_reset_perform_writes_group00_go_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_perform_writes_group00_done (
    .in(early_reset_perform_writes_group00_done_in),
    .out(early_reset_perform_writes_group00_done_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_static_par_thread_go (
    .in(wrapper_early_reset_static_par_thread_go_in),
    .out(wrapper_early_reset_static_par_thread_go_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_static_par_thread_done (
    .in(wrapper_early_reset_static_par_thread_done_in),
    .out(wrapper_early_reset_static_par_thread_done_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_perform_writes_group00_go (
    .in(wrapper_early_reset_perform_writes_group00_go_in),
    .out(wrapper_early_reset_perform_writes_group00_go_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_perform_writes_group00_done (
    .in(wrapper_early_reset_perform_writes_group00_done_in),
    .out(wrapper_early_reset_perform_writes_group00_done_out)
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
) wrapper_early_reset_static_par_thread1_go (
    .in(wrapper_early_reset_static_par_thread1_go_in),
    .out(wrapper_early_reset_static_par_thread1_go_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_static_par_thread1_done (
    .in(wrapper_early_reset_static_par_thread1_done_in),
    .out(wrapper_early_reset_static_par_thread1_done_out)
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
wire _guard1 = signal_reg1_out;
wire _guard2 = _guard0 & _guard0;
wire _guard3 = signal_reg1_out;
wire _guard4 = ~_guard3;
wire _guard5 = _guard2 & _guard4;
wire _guard6 = wrapper_early_reset_static_par_thread0_go_out;
wire _guard7 = _guard5 & _guard6;
wire _guard8 = _guard1 | _guard7;
wire _guard9 = _guard0 & _guard0;
wire _guard10 = signal_reg1_out;
wire _guard11 = ~_guard10;
wire _guard12 = _guard9 & _guard11;
wire _guard13 = wrapper_early_reset_static_par_thread0_go_out;
wire _guard14 = _guard12 & _guard13;
wire _guard15 = signal_reg1_out;
wire _guard16 = tdcc_done_out;
wire _guard17 = do_aw_transfer_go_out;
wire _guard18 = do_aw_transfer_go_out;
wire _guard19 = do_aw_transfer_go_out;
wire _guard20 = do_aw_transfer_go_out;
wire _guard21 = do_aw_transfer_go_out;
wire _guard22 = do_aw_transfer_go_out;
wire _guard23 = do_aw_transfer_go_out;
wire _guard24 = fsm_out == 3'd6;
wire _guard25 = fsm_out == 3'd0;
wire _guard26 = wrapper_early_reset_static_par_thread_done_out;
wire _guard27 = _guard25 & _guard26;
wire _guard28 = tdcc_go_out;
wire _guard29 = _guard27 & _guard28;
wire _guard30 = _guard24 | _guard29;
wire _guard31 = fsm_out == 3'd1;
wire _guard32 = wrapper_early_reset_perform_writes_group00_done_out;
wire _guard33 = comb_reg_out;
wire _guard34 = _guard32 & _guard33;
wire _guard35 = _guard31 & _guard34;
wire _guard36 = tdcc_go_out;
wire _guard37 = _guard35 & _guard36;
wire _guard38 = _guard30 | _guard37;
wire _guard39 = fsm_out == 3'd5;
wire _guard40 = wrapper_early_reset_perform_writes_group00_done_out;
wire _guard41 = comb_reg_out;
wire _guard42 = _guard40 & _guard41;
wire _guard43 = _guard39 & _guard42;
wire _guard44 = tdcc_go_out;
wire _guard45 = _guard43 & _guard44;
wire _guard46 = _guard38 | _guard45;
wire _guard47 = fsm_out == 3'd2;
wire _guard48 = wrapper_early_reset_static_par_thread0_done_out;
wire _guard49 = _guard47 & _guard48;
wire _guard50 = tdcc_go_out;
wire _guard51 = _guard49 & _guard50;
wire _guard52 = _guard46 | _guard51;
wire _guard53 = fsm_out == 3'd3;
wire _guard54 = do_aw_transfer_done_out;
wire _guard55 = _guard53 & _guard54;
wire _guard56 = tdcc_go_out;
wire _guard57 = _guard55 & _guard56;
wire _guard58 = _guard52 | _guard57;
wire _guard59 = fsm_out == 3'd4;
wire _guard60 = wrapper_early_reset_static_par_thread1_done_out;
wire _guard61 = _guard59 & _guard60;
wire _guard62 = tdcc_go_out;
wire _guard63 = _guard61 & _guard62;
wire _guard64 = _guard58 | _guard63;
wire _guard65 = fsm_out == 3'd1;
wire _guard66 = wrapper_early_reset_perform_writes_group00_done_out;
wire _guard67 = comb_reg_out;
wire _guard68 = ~_guard67;
wire _guard69 = _guard66 & _guard68;
wire _guard70 = _guard65 & _guard69;
wire _guard71 = tdcc_go_out;
wire _guard72 = _guard70 & _guard71;
wire _guard73 = _guard64 | _guard72;
wire _guard74 = fsm_out == 3'd5;
wire _guard75 = wrapper_early_reset_perform_writes_group00_done_out;
wire _guard76 = comb_reg_out;
wire _guard77 = ~_guard76;
wire _guard78 = _guard75 & _guard77;
wire _guard79 = _guard74 & _guard78;
wire _guard80 = tdcc_go_out;
wire _guard81 = _guard79 & _guard80;
wire _guard82 = _guard73 | _guard81;
wire _guard83 = fsm_out == 3'd1;
wire _guard84 = wrapper_early_reset_perform_writes_group00_done_out;
wire _guard85 = comb_reg_out;
wire _guard86 = ~_guard85;
wire _guard87 = _guard84 & _guard86;
wire _guard88 = _guard83 & _guard87;
wire _guard89 = tdcc_go_out;
wire _guard90 = _guard88 & _guard89;
wire _guard91 = fsm_out == 3'd5;
wire _guard92 = wrapper_early_reset_perform_writes_group00_done_out;
wire _guard93 = comb_reg_out;
wire _guard94 = ~_guard93;
wire _guard95 = _guard92 & _guard94;
wire _guard96 = _guard91 & _guard95;
wire _guard97 = tdcc_go_out;
wire _guard98 = _guard96 & _guard97;
wire _guard99 = _guard90 | _guard98;
wire _guard100 = fsm_out == 3'd4;
wire _guard101 = wrapper_early_reset_static_par_thread1_done_out;
wire _guard102 = _guard100 & _guard101;
wire _guard103 = tdcc_go_out;
wire _guard104 = _guard102 & _guard103;
wire _guard105 = fsm_out == 3'd1;
wire _guard106 = wrapper_early_reset_perform_writes_group00_done_out;
wire _guard107 = comb_reg_out;
wire _guard108 = _guard106 & _guard107;
wire _guard109 = _guard105 & _guard108;
wire _guard110 = tdcc_go_out;
wire _guard111 = _guard109 & _guard110;
wire _guard112 = fsm_out == 3'd5;
wire _guard113 = wrapper_early_reset_perform_writes_group00_done_out;
wire _guard114 = comb_reg_out;
wire _guard115 = _guard113 & _guard114;
wire _guard116 = _guard112 & _guard115;
wire _guard117 = tdcc_go_out;
wire _guard118 = _guard116 & _guard117;
wire _guard119 = _guard111 | _guard118;
wire _guard120 = fsm_out == 3'd3;
wire _guard121 = do_aw_transfer_done_out;
wire _guard122 = _guard120 & _guard121;
wire _guard123 = tdcc_go_out;
wire _guard124 = _guard122 & _guard123;
wire _guard125 = fsm_out == 3'd0;
wire _guard126 = wrapper_early_reset_static_par_thread_done_out;
wire _guard127 = _guard125 & _guard126;
wire _guard128 = tdcc_go_out;
wire _guard129 = _guard127 & _guard128;
wire _guard130 = fsm_out == 3'd6;
wire _guard131 = fsm_out == 3'd2;
wire _guard132 = wrapper_early_reset_static_par_thread0_done_out;
wire _guard133 = _guard131 & _guard132;
wire _guard134 = tdcc_go_out;
wire _guard135 = _guard133 & _guard134;
wire _guard136 = wrapper_early_reset_static_par_thread1_go_out;
wire _guard137 = signal_reg0_out;
wire _guard138 = _guard0 & _guard0;
wire _guard139 = signal_reg0_out;
wire _guard140 = ~_guard139;
wire _guard141 = _guard138 & _guard140;
wire _guard142 = wrapper_early_reset_perform_writes_group00_go_out;
wire _guard143 = _guard141 & _guard142;
wire _guard144 = _guard137 | _guard143;
wire _guard145 = _guard0 & _guard0;
wire _guard146 = signal_reg0_out;
wire _guard147 = ~_guard146;
wire _guard148 = _guard145 & _guard147;
wire _guard149 = wrapper_early_reset_perform_writes_group00_go_out;
wire _guard150 = _guard148 & _guard149;
wire _guard151 = signal_reg0_out;
wire _guard152 = wrapper_early_reset_static_par_thread_done_out;
wire _guard153 = ~_guard152;
wire _guard154 = fsm_out == 3'd0;
wire _guard155 = _guard153 & _guard154;
wire _guard156 = tdcc_go_out;
wire _guard157 = _guard155 & _guard156;
wire _guard158 = early_reset_perform_writes_group00_go_out;
wire _guard159 = early_reset_perform_writes_group00_go_out;
wire _guard160 = wrapper_early_reset_perform_writes_group00_done_out;
wire _guard161 = ~_guard160;
wire _guard162 = fsm_out == 3'd1;
wire _guard163 = _guard161 & _guard162;
wire _guard164 = tdcc_go_out;
wire _guard165 = _guard163 & _guard164;
wire _guard166 = wrapper_early_reset_perform_writes_group00_done_out;
wire _guard167 = ~_guard166;
wire _guard168 = fsm_out == 3'd5;
wire _guard169 = _guard167 & _guard168;
wire _guard170 = tdcc_go_out;
wire _guard171 = _guard169 & _guard170;
wire _guard172 = _guard165 | _guard171;
wire _guard173 = early_reset_static_par_thread_go_out;
wire _guard174 = early_reset_static_par_thread1_go_out;
wire _guard175 = _guard173 | _guard174;
wire _guard176 = early_reset_static_par_thread_go_out;
wire _guard177 = early_reset_static_par_thread1_go_out;
wire _guard178 = do_aw_transfer_done_out;
wire _guard179 = ~_guard178;
wire _guard180 = fsm_out == 3'd3;
wire _guard181 = _guard179 & _guard180;
wire _guard182 = tdcc_go_out;
wire _guard183 = _guard181 & _guard182;
wire _guard184 = signal_reg0_out;
wire _guard185 = early_reset_perform_writes_group00_go_out;
wire _guard186 = early_reset_perform_writes_group00_go_out;
wire _guard187 = signal_reg2_out;
wire _guard188 = _guard0 & _guard0;
wire _guard189 = signal_reg2_out;
wire _guard190 = ~_guard189;
wire _guard191 = _guard188 & _guard190;
wire _guard192 = wrapper_early_reset_static_par_thread1_go_out;
wire _guard193 = _guard191 & _guard192;
wire _guard194 = _guard187 | _guard193;
wire _guard195 = _guard0 & _guard0;
wire _guard196 = signal_reg2_out;
wire _guard197 = ~_guard196;
wire _guard198 = _guard195 & _guard197;
wire _guard199 = wrapper_early_reset_static_par_thread1_go_out;
wire _guard200 = _guard198 & _guard199;
wire _guard201 = signal_reg2_out;
wire _guard202 = wrapper_early_reset_static_par_thread0_done_out;
wire _guard203 = ~_guard202;
wire _guard204 = fsm_out == 3'd2;
wire _guard205 = _guard203 & _guard204;
wire _guard206 = tdcc_go_out;
wire _guard207 = _guard205 & _guard206;
wire _guard208 = do_aw_transfer_go_out;
wire _guard209 = early_reset_static_par_thread0_go_out;
wire _guard210 = _guard208 | _guard209;
wire _guard211 = AWREADY;
wire _guard212 = awvalid_out;
wire _guard213 = _guard211 & _guard212;
wire _guard214 = do_aw_transfer_go_out;
wire _guard215 = _guard213 & _guard214;
wire _guard216 = AWREADY;
wire _guard217 = awvalid_out;
wire _guard218 = _guard216 & _guard217;
wire _guard219 = ~_guard218;
wire _guard220 = do_aw_transfer_go_out;
wire _guard221 = _guard219 & _guard220;
wire _guard222 = early_reset_static_par_thread0_go_out;
wire _guard223 = _guard221 | _guard222;
wire _guard224 = early_reset_static_par_thread_go_out;
wire _guard225 = early_reset_static_par_thread_go_out;
wire _guard226 = signal_reg_out;
wire _guard227 = _guard0 & _guard0;
wire _guard228 = signal_reg_out;
wire _guard229 = ~_guard228;
wire _guard230 = _guard227 & _guard229;
wire _guard231 = wrapper_early_reset_static_par_thread_go_out;
wire _guard232 = _guard230 & _guard231;
wire _guard233 = _guard226 | _guard232;
wire _guard234 = _guard0 & _guard0;
wire _guard235 = signal_reg_out;
wire _guard236 = ~_guard235;
wire _guard237 = _guard234 & _guard236;
wire _guard238 = wrapper_early_reset_static_par_thread_go_out;
wire _guard239 = _guard237 & _guard238;
wire _guard240 = signal_reg_out;
wire _guard241 = wrapper_early_reset_static_par_thread_go_out;
wire _guard242 = signal_reg1_out;
wire _guard243 = wrapper_early_reset_static_par_thread1_done_out;
wire _guard244 = ~_guard243;
wire _guard245 = fsm_out == 3'd4;
wire _guard246 = _guard244 & _guard245;
wire _guard247 = tdcc_go_out;
wire _guard248 = _guard246 & _guard247;
wire _guard249 = wrapper_early_reset_static_par_thread0_go_out;
wire _guard250 = signal_reg_out;
wire _guard251 = early_reset_static_par_thread1_go_out;
wire _guard252 = early_reset_static_par_thread1_go_out;
wire _guard253 = aw_handshake_occurred_out;
wire _guard254 = ~_guard253;
wire _guard255 = do_aw_transfer_go_out;
wire _guard256 = _guard254 & _guard255;
wire _guard257 = early_reset_static_par_thread0_go_out;
wire _guard258 = _guard256 | _guard257;
wire _guard259 = awvalid_out;
wire _guard260 = AWREADY;
wire _guard261 = _guard259 & _guard260;
wire _guard262 = do_aw_transfer_go_out;
wire _guard263 = _guard261 & _guard262;
wire _guard264 = early_reset_static_par_thread0_go_out;
wire _guard265 = fsm_out == 3'd6;
wire _guard266 = wrapper_early_reset_perform_writes_group00_go_out;
wire _guard267 = do_aw_transfer_go_out;
wire _guard268 = early_reset_static_par_thread1_go_out;
wire _guard269 = _guard267 | _guard268;
wire _guard270 = awvalid_out;
wire _guard271 = AWREADY;
wire _guard272 = _guard270 & _guard271;
wire _guard273 = ~_guard272;
wire _guard274 = aw_handshake_occurred_out;
wire _guard275 = ~_guard274;
wire _guard276 = _guard273 & _guard275;
wire _guard277 = do_aw_transfer_go_out;
wire _guard278 = _guard276 & _guard277;
wire _guard279 = awvalid_out;
wire _guard280 = AWREADY;
wire _guard281 = _guard279 & _guard280;
wire _guard282 = aw_handshake_occurred_out;
wire _guard283 = _guard281 | _guard282;
wire _guard284 = do_aw_transfer_go_out;
wire _guard285 = _guard283 & _guard284;
wire _guard286 = early_reset_static_par_thread1_go_out;
wire _guard287 = _guard285 | _guard286;
wire _guard288 = signal_reg2_out;
assign signal_reg1_write_en = _guard8;
assign signal_reg1_clk = clk;
assign signal_reg1_reset = reset;
assign signal_reg1_in =
  _guard14 ? 1'd1 :
  _guard15 ? 1'd0 :
  1'd0;
assign done = _guard16;
assign AWADDR =
  _guard17 ? curr_addr_axi_out :
  64'd0;
assign AWPROT =
  _guard18 ? 3'd6 :
  3'd0;
assign AWSIZE =
  _guard19 ? 3'd2 :
  3'd0;
assign max_transfers_in = 8'd7;
assign AWVALID = awvalid_out;
assign AWBURST =
  _guard21 ? 2'd1 :
  2'd0;
assign AWLEN =
  _guard22 ? awlen_out :
  8'd0;
assign max_transfers_write_en = _guard23;
assign fsm_write_en = _guard82;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard99 ? 3'd6 :
  _guard104 ? 3'd5 :
  _guard119 ? 3'd2 :
  _guard124 ? 3'd4 :
  _guard129 ? 3'd1 :
  _guard130 ? 3'd0 :
  _guard135 ? 3'd3 :
  3'd0;
assign early_reset_static_par_thread1_go_in = _guard136;
assign signal_reg0_write_en = _guard144;
assign signal_reg0_clk = clk;
assign signal_reg0_reset = reset;
assign signal_reg0_in =
  _guard150 ? 1'd1 :
  _guard151 ? 1'd0 :
  1'd0;
assign wrapper_early_reset_static_par_thread_go_in = _guard157;
assign comb_reg_write_en = _guard158;
assign comb_reg_clk = clk;
assign comb_reg_reset = reset;
assign comb_reg_in =
  _guard159 ? perform_writes_out :
  1'd0;
assign early_reset_static_par_thread0_done_in = ud1_out;
assign wrapper_early_reset_perform_writes_group00_go_in = _guard172;
assign txn_count_write_en = _guard175;
assign txn_count_clk = clk;
assign txn_count_reset = reset;
assign txn_count_in =
  _guard176 ? 32'd0 :
  _guard177 ? txn_adder_out :
  'x;
assign tdcc_go_in = go;
assign early_reset_static_par_thread1_done_in = ud2_out;
assign do_aw_transfer_go_in = _guard183;
assign do_aw_transfer_done_in = bt_reg_out;
assign wrapper_early_reset_perform_writes_group00_done_in = _guard184;
assign perform_writes_left =
  _guard185 ? txn_count_out :
  32'd0;
assign perform_writes_right =
  _guard186 ? txn_n_out :
  32'd0;
assign signal_reg2_write_en = _guard194;
assign signal_reg2_clk = clk;
assign signal_reg2_reset = reset;
assign signal_reg2_in =
  _guard200 ? 1'd1 :
  _guard201 ? 1'd0 :
  1'd0;
assign wrapper_early_reset_static_par_thread0_go_in = _guard207;
assign bt_reg_write_en = _guard210;
assign bt_reg_clk = clk;
assign bt_reg_reset = reset;
assign bt_reg_in =
  _guard215 ? 1'd1 :
  _guard223 ? 1'd0 :
  'x;
assign awlen_write_en = _guard224;
assign awlen_clk = clk;
assign awlen_reset = reset;
assign awlen_in = 8'd7;
assign signal_reg_write_en = _guard233;
assign signal_reg_clk = clk;
assign signal_reg_reset = reset;
assign signal_reg_in =
  _guard239 ? 1'd1 :
  _guard240 ? 1'd0 :
  1'd0;
assign early_reset_static_par_thread_go_in = _guard241;
assign wrapper_early_reset_static_par_thread0_done_in = _guard242;
assign wrapper_early_reset_static_par_thread1_go_in = _guard248;
assign early_reset_static_par_thread0_go_in = _guard249;
assign wrapper_early_reset_static_par_thread_done_in = _guard250;
assign txn_adder_left = txn_count_out;
assign txn_adder_right = 32'd1;
assign aw_handshake_occurred_write_en = _guard258;
assign aw_handshake_occurred_clk = clk;
assign aw_handshake_occurred_reset = reset;
assign aw_handshake_occurred_in =
  _guard263 ? 1'd1 :
  _guard264 ? 1'd0 :
  'x;
assign tdcc_done_in = _guard265;
assign early_reset_perform_writes_group00_go_in = _guard266;
assign awvalid_write_en = _guard269;
assign awvalid_clk = clk;
assign awvalid_reset = reset;
assign awvalid_in =
  _guard278 ? 1'd1 :
  _guard287 ? 1'd0 :
  'x;
assign wrapper_early_reset_static_par_thread1_done_in = _guard288;
assign early_reset_static_par_thread_done_in = ud_out;
assign early_reset_perform_writes_group00_done_in = ud3_out;
// COMPONENT END: m_aw_channel
endmodule
module m_read_channel(
  input logic ARESETn,
  input logic RVALID,
  input logic RLAST,
  input logic [31:0] RDATA,
  input logic [1:0] RRESP,
  output logic RREADY,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done,
  output logic [2:0] mem_ref_addr0,
  output logic mem_ref_content_en,
  output logic mem_ref_write_en,
  output logic [31:0] mem_ref_write_data,
  input logic [31:0] mem_ref_read_data,
  input logic mem_ref_done,
  output logic [2:0] curr_addr_internal_mem_in,
  output logic curr_addr_internal_mem_write_en,
  input logic [2:0] curr_addr_internal_mem_out,
  input logic curr_addr_internal_mem_done,
  output logic [63:0] curr_addr_axi_in,
  output logic curr_addr_axi_write_en,
  input logic [63:0] curr_addr_axi_out,
  input logic curr_addr_axi_done
);
// COMPONENT START: m_read_channel
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
logic [31:0] read_data_reg_in;
logic read_data_reg_write_en;
logic read_data_reg_clk;
logic read_data_reg_reset;
logic [31:0] read_data_reg_out;
logic read_data_reg_done;
logic bt_reg_in;
logic bt_reg_write_en;
logic bt_reg_clk;
logic bt_reg_reset;
logic bt_reg_out;
logic bt_reg_done;
logic [2:0] curr_addr_internal_mem_incr_1_1_left;
logic [2:0] curr_addr_internal_mem_incr_1_1_right;
logic [2:0] curr_addr_internal_mem_incr_1_1_out;
logic [63:0] curr_addr_axi_incr_4_2_left;
logic [63:0] curr_addr_axi_incr_4_2_right;
logic [63:0] curr_addr_axi_incr_4_2_out;
logic pd_in;
logic pd_write_en;
logic pd_clk;
logic pd_reset;
logic pd_out;
logic pd_done;
logic pd0_in;
logic pd0_write_en;
logic pd0_clk;
logic pd0_reset;
logic pd0_out;
logic pd0_done;
logic [2:0] fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic [2:0] fsm_out;
logic fsm_done;
logic block_transfer_go_in;
logic block_transfer_go_out;
logic block_transfer_done_in;
logic block_transfer_done_out;
logic service_read_transfer_go_in;
logic service_read_transfer_go_out;
logic service_read_transfer_done_in;
logic service_read_transfer_done_out;
logic curr_addr_internal_mem_incr_1_1_group_go_in;
logic curr_addr_internal_mem_incr_1_1_group_go_out;
logic curr_addr_internal_mem_incr_1_1_group_done_in;
logic curr_addr_internal_mem_incr_1_1_group_done_out;
logic curr_addr_axi_incr_4_2_group_go_in;
logic curr_addr_axi_incr_4_2_group_go_out;
logic curr_addr_axi_incr_4_2_group_done_in;
logic curr_addr_axi_incr_4_2_group_done_out;
logic invoke0_go_in;
logic invoke0_go_out;
logic invoke0_done_in;
logic invoke0_done_out;
logic invoke1_go_in;
logic invoke1_go_out;
logic invoke1_done_in;
logic invoke1_done_out;
logic par0_go_in;
logic par0_go_out;
logic par0_done_in;
logic par0_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
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
    .WIDTH(32)
) read_data_reg (
    .clk(read_data_reg_clk),
    .done(read_data_reg_done),
    .in(read_data_reg_in),
    .out(read_data_reg_out),
    .reset(read_data_reg_reset),
    .write_en(read_data_reg_write_en)
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
std_add # (
    .WIDTH(3)
) curr_addr_internal_mem_incr_1_1 (
    .left(curr_addr_internal_mem_incr_1_1_left),
    .out(curr_addr_internal_mem_incr_1_1_out),
    .right(curr_addr_internal_mem_incr_1_1_right)
);
std_add # (
    .WIDTH(64)
) curr_addr_axi_incr_4_2 (
    .left(curr_addr_axi_incr_4_2_left),
    .out(curr_addr_axi_incr_4_2_out),
    .right(curr_addr_axi_incr_4_2_right)
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
) service_read_transfer_go (
    .in(service_read_transfer_go_in),
    .out(service_read_transfer_go_out)
);
std_wire # (
    .WIDTH(1)
) service_read_transfer_done (
    .in(service_read_transfer_done_in),
    .out(service_read_transfer_done_out)
);
std_wire # (
    .WIDTH(1)
) curr_addr_internal_mem_incr_1_1_group_go (
    .in(curr_addr_internal_mem_incr_1_1_group_go_in),
    .out(curr_addr_internal_mem_incr_1_1_group_go_out)
);
std_wire # (
    .WIDTH(1)
) curr_addr_internal_mem_incr_1_1_group_done (
    .in(curr_addr_internal_mem_incr_1_1_group_done_in),
    .out(curr_addr_internal_mem_incr_1_1_group_done_out)
);
std_wire # (
    .WIDTH(1)
) curr_addr_axi_incr_4_2_group_go (
    .in(curr_addr_axi_incr_4_2_group_go_in),
    .out(curr_addr_axi_incr_4_2_group_go_out)
);
std_wire # (
    .WIDTH(1)
) curr_addr_axi_incr_4_2_group_done (
    .in(curr_addr_axi_incr_4_2_group_done_in),
    .out(curr_addr_axi_incr_4_2_group_done_out)
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
wire _guard0 = 1;
wire _guard1 = tdcc_done_out;
wire _guard2 = service_read_transfer_go_out;
wire _guard3 = curr_addr_axi_incr_4_2_group_go_out;
wire _guard4 = curr_addr_axi_incr_4_2_group_go_out;
wire _guard5 = curr_addr_internal_mem_incr_1_1_group_go_out;
wire _guard6 = service_read_transfer_go_out;
wire _guard7 = service_read_transfer_go_out;
wire _guard8 = service_read_transfer_go_out;
wire _guard9 = curr_addr_internal_mem_incr_1_1_group_go_out;
wire _guard10 = fsm_out == 3'd5;
wire _guard11 = fsm_out == 3'd0;
wire _guard12 = invoke0_done_out;
wire _guard13 = n_RLAST_out;
wire _guard14 = _guard12 & _guard13;
wire _guard15 = _guard11 & _guard14;
wire _guard16 = tdcc_go_out;
wire _guard17 = _guard15 & _guard16;
wire _guard18 = _guard10 | _guard17;
wire _guard19 = fsm_out == 3'd4;
wire _guard20 = par0_done_out;
wire _guard21 = n_RLAST_out;
wire _guard22 = _guard20 & _guard21;
wire _guard23 = _guard19 & _guard22;
wire _guard24 = tdcc_go_out;
wire _guard25 = _guard23 & _guard24;
wire _guard26 = _guard18 | _guard25;
wire _guard27 = fsm_out == 3'd1;
wire _guard28 = invoke1_done_out;
wire _guard29 = _guard27 & _guard28;
wire _guard30 = tdcc_go_out;
wire _guard31 = _guard29 & _guard30;
wire _guard32 = _guard26 | _guard31;
wire _guard33 = fsm_out == 3'd2;
wire _guard34 = block_transfer_done_out;
wire _guard35 = _guard33 & _guard34;
wire _guard36 = tdcc_go_out;
wire _guard37 = _guard35 & _guard36;
wire _guard38 = _guard32 | _guard37;
wire _guard39 = fsm_out == 3'd3;
wire _guard40 = service_read_transfer_done_out;
wire _guard41 = _guard39 & _guard40;
wire _guard42 = tdcc_go_out;
wire _guard43 = _guard41 & _guard42;
wire _guard44 = _guard38 | _guard43;
wire _guard45 = fsm_out == 3'd0;
wire _guard46 = invoke0_done_out;
wire _guard47 = n_RLAST_out;
wire _guard48 = ~_guard47;
wire _guard49 = _guard46 & _guard48;
wire _guard50 = _guard45 & _guard49;
wire _guard51 = tdcc_go_out;
wire _guard52 = _guard50 & _guard51;
wire _guard53 = _guard44 | _guard52;
wire _guard54 = fsm_out == 3'd4;
wire _guard55 = par0_done_out;
wire _guard56 = n_RLAST_out;
wire _guard57 = ~_guard56;
wire _guard58 = _guard55 & _guard57;
wire _guard59 = _guard54 & _guard58;
wire _guard60 = tdcc_go_out;
wire _guard61 = _guard59 & _guard60;
wire _guard62 = _guard53 | _guard61;
wire _guard63 = fsm_out == 3'd0;
wire _guard64 = invoke0_done_out;
wire _guard65 = n_RLAST_out;
wire _guard66 = ~_guard65;
wire _guard67 = _guard64 & _guard66;
wire _guard68 = _guard63 & _guard67;
wire _guard69 = tdcc_go_out;
wire _guard70 = _guard68 & _guard69;
wire _guard71 = fsm_out == 3'd4;
wire _guard72 = par0_done_out;
wire _guard73 = n_RLAST_out;
wire _guard74 = ~_guard73;
wire _guard75 = _guard72 & _guard74;
wire _guard76 = _guard71 & _guard75;
wire _guard77 = tdcc_go_out;
wire _guard78 = _guard76 & _guard77;
wire _guard79 = _guard70 | _guard78;
wire _guard80 = fsm_out == 3'd1;
wire _guard81 = invoke1_done_out;
wire _guard82 = _guard80 & _guard81;
wire _guard83 = tdcc_go_out;
wire _guard84 = _guard82 & _guard83;
wire _guard85 = fsm_out == 3'd3;
wire _guard86 = service_read_transfer_done_out;
wire _guard87 = _guard85 & _guard86;
wire _guard88 = tdcc_go_out;
wire _guard89 = _guard87 & _guard88;
wire _guard90 = fsm_out == 3'd0;
wire _guard91 = invoke0_done_out;
wire _guard92 = n_RLAST_out;
wire _guard93 = _guard91 & _guard92;
wire _guard94 = _guard90 & _guard93;
wire _guard95 = tdcc_go_out;
wire _guard96 = _guard94 & _guard95;
wire _guard97 = fsm_out == 3'd4;
wire _guard98 = par0_done_out;
wire _guard99 = n_RLAST_out;
wire _guard100 = _guard98 & _guard99;
wire _guard101 = _guard97 & _guard100;
wire _guard102 = tdcc_go_out;
wire _guard103 = _guard101 & _guard102;
wire _guard104 = _guard96 | _guard103;
wire _guard105 = fsm_out == 3'd5;
wire _guard106 = fsm_out == 3'd2;
wire _guard107 = block_transfer_done_out;
wire _guard108 = _guard106 & _guard107;
wire _guard109 = tdcc_go_out;
wire _guard110 = _guard108 & _guard109;
wire _guard111 = curr_addr_axi_incr_4_2_group_go_out;
wire _guard112 = curr_addr_axi_incr_4_2_group_go_out;
wire _guard113 = rready_out;
wire _guard114 = RVALID;
wire _guard115 = _guard113 & _guard114;
wire _guard116 = block_transfer_go_out;
wire _guard117 = _guard115 & _guard116;
wire _guard118 = rready_out;
wire _guard119 = RVALID;
wire _guard120 = _guard118 & _guard119;
wire _guard121 = ~_guard120;
wire _guard122 = block_transfer_go_out;
wire _guard123 = _guard121 & _guard122;
wire _guard124 = block_transfer_go_out;
wire _guard125 = pd0_out;
wire _guard126 = curr_addr_axi_incr_4_2_group_done_out;
wire _guard127 = _guard125 | _guard126;
wire _guard128 = ~_guard127;
wire _guard129 = par0_go_out;
wire _guard130 = _guard128 & _guard129;
wire _guard131 = curr_addr_internal_mem_incr_1_1_group_go_out;
wire _guard132 = curr_addr_internal_mem_incr_1_1_group_go_out;
wire _guard133 = invoke0_done_out;
wire _guard134 = ~_guard133;
wire _guard135 = fsm_out == 3'd0;
wire _guard136 = _guard134 & _guard135;
wire _guard137 = tdcc_go_out;
wire _guard138 = _guard136 & _guard137;
wire _guard139 = pd_out;
wire _guard140 = pd0_out;
wire _guard141 = _guard139 & _guard140;
wire _guard142 = block_transfer_go_out;
wire _guard143 = invoke0_go_out;
wire _guard144 = _guard142 | _guard143;
wire _guard145 = RLAST;
wire _guard146 = ~_guard145;
wire _guard147 = block_transfer_go_out;
wire _guard148 = _guard146 & _guard147;
wire _guard149 = invoke0_go_out;
wire _guard150 = _guard148 | _guard149;
wire _guard151 = RLAST;
wire _guard152 = block_transfer_go_out;
wire _guard153 = _guard151 & _guard152;
wire _guard154 = invoke1_done_out;
wire _guard155 = ~_guard154;
wire _guard156 = fsm_out == 3'd1;
wire _guard157 = _guard155 & _guard156;
wire _guard158 = tdcc_go_out;
wire _guard159 = _guard157 & _guard158;
wire _guard160 = block_transfer_go_out;
wire _guard161 = invoke1_go_out;
wire _guard162 = _guard160 | _guard161;
wire _guard163 = rready_out;
wire _guard164 = RVALID;
wire _guard165 = _guard163 & _guard164;
wire _guard166 = block_transfer_go_out;
wire _guard167 = _guard165 & _guard166;
wire _guard168 = rready_out;
wire _guard169 = RVALID;
wire _guard170 = _guard168 & _guard169;
wire _guard171 = ~_guard170;
wire _guard172 = block_transfer_go_out;
wire _guard173 = _guard171 & _guard172;
wire _guard174 = invoke1_go_out;
wire _guard175 = _guard173 | _guard174;
wire _guard176 = pd_out;
wire _guard177 = pd0_out;
wire _guard178 = _guard176 & _guard177;
wire _guard179 = curr_addr_internal_mem_incr_1_1_group_done_out;
wire _guard180 = par0_go_out;
wire _guard181 = _guard179 & _guard180;
wire _guard182 = _guard178 | _guard181;
wire _guard183 = curr_addr_internal_mem_incr_1_1_group_done_out;
wire _guard184 = par0_go_out;
wire _guard185 = _guard183 & _guard184;
wire _guard186 = pd_out;
wire _guard187 = pd0_out;
wire _guard188 = _guard186 & _guard187;
wire _guard189 = pd_out;
wire _guard190 = pd0_out;
wire _guard191 = _guard189 & _guard190;
wire _guard192 = curr_addr_axi_incr_4_2_group_done_out;
wire _guard193 = par0_go_out;
wire _guard194 = _guard192 & _guard193;
wire _guard195 = _guard191 | _guard194;
wire _guard196 = curr_addr_axi_incr_4_2_group_done_out;
wire _guard197 = par0_go_out;
wire _guard198 = _guard196 & _guard197;
wire _guard199 = pd_out;
wire _guard200 = pd0_out;
wire _guard201 = _guard199 & _guard200;
wire _guard202 = fsm_out == 3'd5;
wire _guard203 = pd_out;
wire _guard204 = curr_addr_internal_mem_incr_1_1_group_done_out;
wire _guard205 = _guard203 | _guard204;
wire _guard206 = ~_guard205;
wire _guard207 = par0_go_out;
wire _guard208 = _guard206 & _guard207;
wire _guard209 = block_transfer_done_out;
wire _guard210 = ~_guard209;
wire _guard211 = fsm_out == 3'd2;
wire _guard212 = _guard210 & _guard211;
wire _guard213 = tdcc_go_out;
wire _guard214 = _guard212 & _guard213;
wire _guard215 = block_transfer_go_out;
wire _guard216 = service_read_transfer_go_out;
wire _guard217 = _guard215 | _guard216;
wire _guard218 = rready_out;
wire _guard219 = RVALID;
wire _guard220 = _guard218 & _guard219;
wire _guard221 = ~_guard220;
wire _guard222 = block_transfer_go_out;
wire _guard223 = _guard221 & _guard222;
wire _guard224 = rready_out;
wire _guard225 = RVALID;
wire _guard226 = _guard224 & _guard225;
wire _guard227 = block_transfer_go_out;
wire _guard228 = _guard226 & _guard227;
wire _guard229 = service_read_transfer_go_out;
wire _guard230 = _guard228 | _guard229;
wire _guard231 = service_read_transfer_done_out;
wire _guard232 = ~_guard231;
wire _guard233 = fsm_out == 3'd3;
wire _guard234 = _guard232 & _guard233;
wire _guard235 = tdcc_go_out;
wire _guard236 = _guard234 & _guard235;
wire _guard237 = par0_done_out;
wire _guard238 = ~_guard237;
wire _guard239 = fsm_out == 3'd4;
wire _guard240 = _guard238 & _guard239;
wire _guard241 = tdcc_go_out;
wire _guard242 = _guard240 & _guard241;
assign done = _guard1;
assign mem_ref_content_en = _guard2;
assign curr_addr_axi_write_en = _guard3;
assign curr_addr_axi_in = curr_addr_axi_incr_4_2_out;
assign RREADY = rready_out;
assign curr_addr_internal_mem_write_en = _guard5;
assign mem_ref_write_data = read_data_reg_out;
assign mem_ref_write_en = _guard7;
assign mem_ref_addr0 = curr_addr_internal_mem_out;
assign curr_addr_internal_mem_in = curr_addr_internal_mem_incr_1_1_out;
assign fsm_write_en = _guard62;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard79 ? 3'd5 :
  _guard84 ? 3'd2 :
  _guard89 ? 3'd4 :
  _guard104 ? 3'd1 :
  _guard105 ? 3'd0 :
  _guard110 ? 3'd3 :
  3'd0;
assign curr_addr_axi_incr_4_2_group_done_in = curr_addr_axi_done;
assign curr_addr_axi_incr_4_2_left = curr_addr_axi_out;
assign curr_addr_axi_incr_4_2_right = 64'd4;
assign block_transfer_done_in = bt_reg_out;
assign service_read_transfer_done_in = mem_ref_done;
assign read_data_reg_write_en =
  _guard117 ? 1'd1 :
  _guard123 ? 1'd0 :
  1'd0;
assign read_data_reg_clk = clk;
assign read_data_reg_reset = reset;
assign read_data_reg_in = RDATA;
assign curr_addr_axi_incr_4_2_group_go_in = _guard130;
assign curr_addr_internal_mem_incr_1_1_left = curr_addr_internal_mem_out;
assign curr_addr_internal_mem_incr_1_1_right = 3'd1;
assign tdcc_go_in = go;
assign invoke0_go_in = _guard138;
assign par0_done_in = _guard141;
assign n_RLAST_write_en = _guard144;
assign n_RLAST_clk = clk;
assign n_RLAST_reset = reset;
assign n_RLAST_in =
  _guard150 ? 1'd1 :
  _guard153 ? 1'd0 :
  'x;
assign invoke0_done_in = n_RLAST_done;
assign invoke1_go_in = _guard159;
assign bt_reg_write_en = _guard162;
assign bt_reg_clk = clk;
assign bt_reg_reset = reset;
assign bt_reg_in =
  _guard167 ? 1'd1 :
  _guard175 ? 1'd0 :
  'x;
assign curr_addr_internal_mem_incr_1_1_group_done_in = curr_addr_internal_mem_done;
assign pd_write_en = _guard182;
assign pd_clk = clk;
assign pd_reset = reset;
assign pd_in =
  _guard185 ? 1'd1 :
  _guard188 ? 1'd0 :
  1'd0;
assign pd0_write_en = _guard195;
assign pd0_clk = clk;
assign pd0_reset = reset;
assign pd0_in =
  _guard198 ? 1'd1 :
  _guard201 ? 1'd0 :
  1'd0;
assign tdcc_done_in = _guard202;
assign curr_addr_internal_mem_incr_1_1_group_go_in = _guard208;
assign block_transfer_go_in = _guard214;
assign invoke1_done_in = bt_reg_done;
assign rready_write_en = _guard217;
assign rready_clk = clk;
assign rready_reset = reset;
assign rready_in =
  _guard223 ? 1'd1 :
  _guard230 ? 1'd0 :
  'x;
assign service_read_transfer_go_in = _guard236;
assign par0_go_in = _guard242;
// COMPONENT END: m_read_channel
endmodule
module m_write_channel(
  input logic ARESETn,
  input logic WREADY,
  output logic WVALID,
  output logic WLAST,
  output logic [31:0] WDATA,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done,
  output logic [2:0] mem_ref_addr0,
  output logic mem_ref_content_en,
  output logic mem_ref_write_en,
  output logic [31:0] mem_ref_write_data,
  input logic [31:0] mem_ref_read_data,
  input logic mem_ref_done,
  output logic [2:0] curr_addr_internal_mem_in,
  output logic curr_addr_internal_mem_write_en,
  input logic [2:0] curr_addr_internal_mem_out,
  input logic curr_addr_internal_mem_done,
  output logic [63:0] curr_addr_axi_in,
  output logic curr_addr_axi_write_en,
  input logic [63:0] curr_addr_axi_out,
  input logic curr_addr_axi_done,
  output logic [7:0] max_transfers_in,
  output logic max_transfers_write_en,
  input logic [7:0] max_transfers_out,
  input logic max_transfers_done
);
// COMPONENT START: m_write_channel
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
logic [7:0] curr_transfer_count_in;
logic curr_transfer_count_write_en;
logic curr_transfer_count_clk;
logic curr_transfer_count_reset;
logic [7:0] curr_transfer_count_out;
logic curr_transfer_count_done;
logic n_finished_last_transfer_in;
logic n_finished_last_transfer_write_en;
logic n_finished_last_transfer_clk;
logic n_finished_last_transfer_reset;
logic n_finished_last_transfer_out;
logic n_finished_last_transfer_done;
logic bt_reg_in;
logic bt_reg_write_en;
logic bt_reg_clk;
logic bt_reg_reset;
logic bt_reg_out;
logic bt_reg_done;
logic [2:0] curr_addr_internal_mem_incr_1_1_left;
logic [2:0] curr_addr_internal_mem_incr_1_1_right;
logic [2:0] curr_addr_internal_mem_incr_1_1_out;
logic [63:0] curr_addr_axi_incr_4_2_left;
logic [63:0] curr_addr_axi_incr_4_2_right;
logic [63:0] curr_addr_axi_incr_4_2_out;
logic [7:0] curr_transfer_count_incr_1_3_left;
logic [7:0] curr_transfer_count_incr_1_3_right;
logic [7:0] curr_transfer_count_incr_1_3_out;
logic ud_out;
logic signal_reg_in;
logic signal_reg_write_en;
logic signal_reg_clk;
logic signal_reg_reset;
logic signal_reg_out;
logic signal_reg_done;
logic pd_in;
logic pd_write_en;
logic pd_clk;
logic pd_reset;
logic pd_out;
logic pd_done;
logic pd0_in;
logic pd0_write_en;
logic pd0_clk;
logic pd0_reset;
logic pd0_out;
logic pd0_done;
logic pd1_in;
logic pd1_write_en;
logic pd1_clk;
logic pd1_reset;
logic pd1_out;
logic pd1_done;
logic [2:0] fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic [2:0] fsm_out;
logic fsm_done;
logic service_write_transfer_go_in;
logic service_write_transfer_go_out;
logic service_write_transfer_done_in;
logic service_write_transfer_done_out;
logic curr_addr_internal_mem_incr_1_1_group_go_in;
logic curr_addr_internal_mem_incr_1_1_group_go_out;
logic curr_addr_internal_mem_incr_1_1_group_done_in;
logic curr_addr_internal_mem_incr_1_1_group_done_out;
logic curr_addr_axi_incr_4_2_group_go_in;
logic curr_addr_axi_incr_4_2_group_go_out;
logic curr_addr_axi_incr_4_2_group_done_in;
logic curr_addr_axi_incr_4_2_group_done_out;
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
logic early_reset_static_par_thread_go_in;
logic early_reset_static_par_thread_go_out;
logic early_reset_static_par_thread_done_in;
logic early_reset_static_par_thread_done_out;
logic wrapper_early_reset_static_par_thread_go_in;
logic wrapper_early_reset_static_par_thread_go_out;
logic wrapper_early_reset_static_par_thread_done_in;
logic wrapper_early_reset_static_par_thread_done_out;
logic par0_go_in;
logic par0_go_out;
logic par0_done_in;
logic par0_done_out;
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
    .WIDTH(8)
) curr_transfer_count (
    .clk(curr_transfer_count_clk),
    .done(curr_transfer_count_done),
    .in(curr_transfer_count_in),
    .out(curr_transfer_count_out),
    .reset(curr_transfer_count_reset),
    .write_en(curr_transfer_count_write_en)
);
std_reg # (
    .WIDTH(1)
) n_finished_last_transfer (
    .clk(n_finished_last_transfer_clk),
    .done(n_finished_last_transfer_done),
    .in(n_finished_last_transfer_in),
    .out(n_finished_last_transfer_out),
    .reset(n_finished_last_transfer_reset),
    .write_en(n_finished_last_transfer_write_en)
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
std_add # (
    .WIDTH(3)
) curr_addr_internal_mem_incr_1_1 (
    .left(curr_addr_internal_mem_incr_1_1_left),
    .out(curr_addr_internal_mem_incr_1_1_out),
    .right(curr_addr_internal_mem_incr_1_1_right)
);
std_add # (
    .WIDTH(64)
) curr_addr_axi_incr_4_2 (
    .left(curr_addr_axi_incr_4_2_left),
    .out(curr_addr_axi_incr_4_2_out),
    .right(curr_addr_axi_incr_4_2_right)
);
std_add # (
    .WIDTH(8)
) curr_transfer_count_incr_1_3 (
    .left(curr_transfer_count_incr_1_3_left),
    .out(curr_transfer_count_incr_1_3_out),
    .right(curr_transfer_count_incr_1_3_right)
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
    .WIDTH(1)
) pd1 (
    .clk(pd1_clk),
    .done(pd1_done),
    .in(pd1_in),
    .out(pd1_out),
    .reset(pd1_reset),
    .write_en(pd1_write_en)
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
) curr_addr_internal_mem_incr_1_1_group_go (
    .in(curr_addr_internal_mem_incr_1_1_group_go_in),
    .out(curr_addr_internal_mem_incr_1_1_group_go_out)
);
std_wire # (
    .WIDTH(1)
) curr_addr_internal_mem_incr_1_1_group_done (
    .in(curr_addr_internal_mem_incr_1_1_group_done_in),
    .out(curr_addr_internal_mem_incr_1_1_group_done_out)
);
std_wire # (
    .WIDTH(1)
) curr_addr_axi_incr_4_2_group_go (
    .in(curr_addr_axi_incr_4_2_group_go_in),
    .out(curr_addr_axi_incr_4_2_group_go_out)
);
std_wire # (
    .WIDTH(1)
) curr_addr_axi_incr_4_2_group_done (
    .in(curr_addr_axi_incr_4_2_group_done_in),
    .out(curr_addr_axi_incr_4_2_group_done_out)
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
) early_reset_static_par_thread_go (
    .in(early_reset_static_par_thread_go_in),
    .out(early_reset_static_par_thread_go_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_static_par_thread_done (
    .in(early_reset_static_par_thread_done_in),
    .out(early_reset_static_par_thread_done_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_static_par_thread_go (
    .in(wrapper_early_reset_static_par_thread_go_in),
    .out(wrapper_early_reset_static_par_thread_go_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_static_par_thread_done (
    .in(wrapper_early_reset_static_par_thread_done_in),
    .out(wrapper_early_reset_static_par_thread_done_out)
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
wire _guard0 = 1;
wire _guard1 = tdcc_done_out;
wire _guard2 = service_write_transfer_go_out;
wire _guard3 = curr_addr_axi_incr_4_2_group_go_out;
wire _guard4 = service_write_transfer_go_out;
wire _guard5 = curr_addr_axi_incr_4_2_group_go_out;
wire _guard6 = curr_addr_internal_mem_incr_1_1_group_go_out;
wire _guard7 = invoke0_go_out;
wire _guard8 = _guard6 | _guard7;
wire _guard9 = max_transfers_out == curr_transfer_count_out;
wire _guard10 = service_write_transfer_go_out;
wire _guard11 = _guard9 & _guard10;
wire _guard12 = max_transfers_out != curr_transfer_count_out;
wire _guard13 = service_write_transfer_go_out;
wire _guard14 = _guard12 & _guard13;
wire _guard15 = service_write_transfer_go_out;
wire _guard16 = curr_addr_internal_mem_incr_1_1_group_go_out;
wire _guard17 = invoke0_go_out;
wire _guard18 = fsm_out == 3'd5;
wire _guard19 = fsm_out == 3'd0;
wire _guard20 = invoke0_done_out;
wire _guard21 = _guard19 & _guard20;
wire _guard22 = tdcc_go_out;
wire _guard23 = _guard21 & _guard22;
wire _guard24 = _guard18 | _guard23;
wire _guard25 = fsm_out == 3'd1;
wire _guard26 = invoke1_done_out;
wire _guard27 = n_finished_last_transfer_out;
wire _guard28 = _guard26 & _guard27;
wire _guard29 = _guard25 & _guard28;
wire _guard30 = tdcc_go_out;
wire _guard31 = _guard29 & _guard30;
wire _guard32 = _guard24 | _guard31;
wire _guard33 = fsm_out == 3'd4;
wire _guard34 = par0_done_out;
wire _guard35 = n_finished_last_transfer_out;
wire _guard36 = _guard34 & _guard35;
wire _guard37 = _guard33 & _guard36;
wire _guard38 = tdcc_go_out;
wire _guard39 = _guard37 & _guard38;
wire _guard40 = _guard32 | _guard39;
wire _guard41 = fsm_out == 3'd2;
wire _guard42 = invoke2_done_out;
wire _guard43 = _guard41 & _guard42;
wire _guard44 = tdcc_go_out;
wire _guard45 = _guard43 & _guard44;
wire _guard46 = _guard40 | _guard45;
wire _guard47 = fsm_out == 3'd3;
wire _guard48 = service_write_transfer_done_out;
wire _guard49 = _guard47 & _guard48;
wire _guard50 = tdcc_go_out;
wire _guard51 = _guard49 & _guard50;
wire _guard52 = _guard46 | _guard51;
wire _guard53 = fsm_out == 3'd1;
wire _guard54 = invoke1_done_out;
wire _guard55 = n_finished_last_transfer_out;
wire _guard56 = ~_guard55;
wire _guard57 = _guard54 & _guard56;
wire _guard58 = _guard53 & _guard57;
wire _guard59 = tdcc_go_out;
wire _guard60 = _guard58 & _guard59;
wire _guard61 = _guard52 | _guard60;
wire _guard62 = fsm_out == 3'd4;
wire _guard63 = par0_done_out;
wire _guard64 = n_finished_last_transfer_out;
wire _guard65 = ~_guard64;
wire _guard66 = _guard63 & _guard65;
wire _guard67 = _guard62 & _guard66;
wire _guard68 = tdcc_go_out;
wire _guard69 = _guard67 & _guard68;
wire _guard70 = _guard61 | _guard69;
wire _guard71 = fsm_out == 3'd1;
wire _guard72 = invoke1_done_out;
wire _guard73 = n_finished_last_transfer_out;
wire _guard74 = ~_guard73;
wire _guard75 = _guard72 & _guard74;
wire _guard76 = _guard71 & _guard75;
wire _guard77 = tdcc_go_out;
wire _guard78 = _guard76 & _guard77;
wire _guard79 = fsm_out == 3'd4;
wire _guard80 = par0_done_out;
wire _guard81 = n_finished_last_transfer_out;
wire _guard82 = ~_guard81;
wire _guard83 = _guard80 & _guard82;
wire _guard84 = _guard79 & _guard83;
wire _guard85 = tdcc_go_out;
wire _guard86 = _guard84 & _guard85;
wire _guard87 = _guard78 | _guard86;
wire _guard88 = fsm_out == 3'd1;
wire _guard89 = invoke1_done_out;
wire _guard90 = n_finished_last_transfer_out;
wire _guard91 = _guard89 & _guard90;
wire _guard92 = _guard88 & _guard91;
wire _guard93 = tdcc_go_out;
wire _guard94 = _guard92 & _guard93;
wire _guard95 = fsm_out == 3'd4;
wire _guard96 = par0_done_out;
wire _guard97 = n_finished_last_transfer_out;
wire _guard98 = _guard96 & _guard97;
wire _guard99 = _guard95 & _guard98;
wire _guard100 = tdcc_go_out;
wire _guard101 = _guard99 & _guard100;
wire _guard102 = _guard94 | _guard101;
wire _guard103 = fsm_out == 3'd3;
wire _guard104 = service_write_transfer_done_out;
wire _guard105 = _guard103 & _guard104;
wire _guard106 = tdcc_go_out;
wire _guard107 = _guard105 & _guard106;
wire _guard108 = fsm_out == 3'd0;
wire _guard109 = invoke0_done_out;
wire _guard110 = _guard108 & _guard109;
wire _guard111 = tdcc_go_out;
wire _guard112 = _guard110 & _guard111;
wire _guard113 = fsm_out == 3'd5;
wire _guard114 = fsm_out == 3'd2;
wire _guard115 = invoke2_done_out;
wire _guard116 = _guard114 & _guard115;
wire _guard117 = tdcc_go_out;
wire _guard118 = _guard116 & _guard117;
wire _guard119 = early_reset_static_par_thread_go_out;
wire _guard120 = early_reset_static_par_thread_go_out;
wire _guard121 = pd_out;
wire _guard122 = wrapper_early_reset_static_par_thread_done_out;
wire _guard123 = _guard121 | _guard122;
wire _guard124 = ~_guard123;
wire _guard125 = par0_go_out;
wire _guard126 = _guard124 & _guard125;
wire _guard127 = invoke2_done_out;
wire _guard128 = ~_guard127;
wire _guard129 = fsm_out == 3'd2;
wire _guard130 = _guard128 & _guard129;
wire _guard131 = tdcc_go_out;
wire _guard132 = _guard130 & _guard131;
wire _guard133 = curr_addr_axi_incr_4_2_group_go_out;
wire _guard134 = curr_addr_axi_incr_4_2_group_go_out;
wire _guard135 = early_reset_static_par_thread_go_out;
wire _guard136 = early_reset_static_par_thread_go_out;
wire _guard137 = pd1_out;
wire _guard138 = curr_addr_axi_incr_4_2_group_done_out;
wire _guard139 = _guard137 | _guard138;
wire _guard140 = ~_guard139;
wire _guard141 = par0_go_out;
wire _guard142 = _guard140 & _guard141;
wire _guard143 = curr_addr_internal_mem_incr_1_1_group_go_out;
wire _guard144 = curr_addr_internal_mem_incr_1_1_group_go_out;
wire _guard145 = service_write_transfer_go_out;
wire _guard146 = wvalid_out;
wire _guard147 = WREADY;
wire _guard148 = _guard146 & _guard147;
wire _guard149 = ~_guard148;
wire _guard150 = w_handshake_occurred_out;
wire _guard151 = ~_guard150;
wire _guard152 = _guard149 & _guard151;
wire _guard153 = service_write_transfer_go_out;
wire _guard154 = _guard152 & _guard153;
wire _guard155 = wvalid_out;
wire _guard156 = WREADY;
wire _guard157 = _guard155 & _guard156;
wire _guard158 = w_handshake_occurred_out;
wire _guard159 = _guard157 | _guard158;
wire _guard160 = service_write_transfer_go_out;
wire _guard161 = _guard159 & _guard160;
wire _guard162 = max_transfers_out == curr_transfer_count_out;
wire _guard163 = wvalid_out;
wire _guard164 = WREADY;
wire _guard165 = _guard163 & _guard164;
wire _guard166 = _guard162 & _guard165;
wire _guard167 = service_write_transfer_go_out;
wire _guard168 = _guard166 & _guard167;
wire _guard169 = invoke1_go_out;
wire _guard170 = _guard168 | _guard169;
wire _guard171 = invoke1_go_out;
wire _guard172 = max_transfers_out == curr_transfer_count_out;
wire _guard173 = wvalid_out;
wire _guard174 = WREADY;
wire _guard175 = _guard173 & _guard174;
wire _guard176 = _guard172 & _guard175;
wire _guard177 = service_write_transfer_go_out;
wire _guard178 = _guard176 & _guard177;
wire _guard179 = pd_out;
wire _guard180 = pd0_out;
wire _guard181 = _guard179 & _guard180;
wire _guard182 = pd1_out;
wire _guard183 = _guard181 & _guard182;
wire _guard184 = curr_addr_axi_incr_4_2_group_done_out;
wire _guard185 = par0_go_out;
wire _guard186 = _guard184 & _guard185;
wire _guard187 = _guard183 | _guard186;
wire _guard188 = curr_addr_axi_incr_4_2_group_done_out;
wire _guard189 = par0_go_out;
wire _guard190 = _guard188 & _guard189;
wire _guard191 = pd_out;
wire _guard192 = pd0_out;
wire _guard193 = _guard191 & _guard192;
wire _guard194 = pd1_out;
wire _guard195 = _guard193 & _guard194;
wire _guard196 = invoke0_done_out;
wire _guard197 = ~_guard196;
wire _guard198 = fsm_out == 3'd0;
wire _guard199 = _guard197 & _guard198;
wire _guard200 = tdcc_go_out;
wire _guard201 = _guard199 & _guard200;
wire _guard202 = pd_out;
wire _guard203 = pd0_out;
wire _guard204 = _guard202 & _guard203;
wire _guard205 = pd1_out;
wire _guard206 = _guard204 & _guard205;
wire _guard207 = service_write_transfer_done_out;
wire _guard208 = ~_guard207;
wire _guard209 = fsm_out == 3'd3;
wire _guard210 = _guard208 & _guard209;
wire _guard211 = tdcc_go_out;
wire _guard212 = _guard210 & _guard211;
wire _guard213 = invoke1_done_out;
wire _guard214 = ~_guard213;
wire _guard215 = fsm_out == 3'd1;
wire _guard216 = _guard214 & _guard215;
wire _guard217 = tdcc_go_out;
wire _guard218 = _guard216 & _guard217;
wire _guard219 = service_write_transfer_go_out;
wire _guard220 = invoke2_go_out;
wire _guard221 = _guard219 | _guard220;
wire _guard222 = wvalid_out;
wire _guard223 = WREADY;
wire _guard224 = _guard222 & _guard223;
wire _guard225 = service_write_transfer_go_out;
wire _guard226 = _guard224 & _guard225;
wire _guard227 = wvalid_out;
wire _guard228 = WREADY;
wire _guard229 = _guard227 & _guard228;
wire _guard230 = ~_guard229;
wire _guard231 = service_write_transfer_go_out;
wire _guard232 = _guard230 & _guard231;
wire _guard233 = invoke2_go_out;
wire _guard234 = _guard232 | _guard233;
wire _guard235 = signal_reg_out;
wire _guard236 = _guard0 & _guard0;
wire _guard237 = signal_reg_out;
wire _guard238 = ~_guard237;
wire _guard239 = _guard236 & _guard238;
wire _guard240 = wrapper_early_reset_static_par_thread_go_out;
wire _guard241 = _guard239 & _guard240;
wire _guard242 = _guard235 | _guard241;
wire _guard243 = _guard0 & _guard0;
wire _guard244 = signal_reg_out;
wire _guard245 = ~_guard244;
wire _guard246 = _guard243 & _guard245;
wire _guard247 = wrapper_early_reset_static_par_thread_go_out;
wire _guard248 = _guard246 & _guard247;
wire _guard249 = signal_reg_out;
wire _guard250 = wrapper_early_reset_static_par_thread_go_out;
wire _guard251 = pd_out;
wire _guard252 = pd0_out;
wire _guard253 = _guard251 & _guard252;
wire _guard254 = pd1_out;
wire _guard255 = _guard253 & _guard254;
wire _guard256 = wrapper_early_reset_static_par_thread_done_out;
wire _guard257 = par0_go_out;
wire _guard258 = _guard256 & _guard257;
wire _guard259 = _guard255 | _guard258;
wire _guard260 = wrapper_early_reset_static_par_thread_done_out;
wire _guard261 = par0_go_out;
wire _guard262 = _guard260 & _guard261;
wire _guard263 = pd_out;
wire _guard264 = pd0_out;
wire _guard265 = _guard263 & _guard264;
wire _guard266 = pd1_out;
wire _guard267 = _guard265 & _guard266;
wire _guard268 = pd_out;
wire _guard269 = pd0_out;
wire _guard270 = _guard268 & _guard269;
wire _guard271 = pd1_out;
wire _guard272 = _guard270 & _guard271;
wire _guard273 = curr_addr_internal_mem_incr_1_1_group_done_out;
wire _guard274 = par0_go_out;
wire _guard275 = _guard273 & _guard274;
wire _guard276 = _guard272 | _guard275;
wire _guard277 = curr_addr_internal_mem_incr_1_1_group_done_out;
wire _guard278 = par0_go_out;
wire _guard279 = _guard277 & _guard278;
wire _guard280 = pd_out;
wire _guard281 = pd0_out;
wire _guard282 = _guard280 & _guard281;
wire _guard283 = pd1_out;
wire _guard284 = _guard282 & _guard283;
wire _guard285 = signal_reg_out;
wire _guard286 = w_handshake_occurred_out;
wire _guard287 = ~_guard286;
wire _guard288 = service_write_transfer_go_out;
wire _guard289 = _guard287 & _guard288;
wire _guard290 = early_reset_static_par_thread_go_out;
wire _guard291 = _guard289 | _guard290;
wire _guard292 = wvalid_out;
wire _guard293 = WREADY;
wire _guard294 = _guard292 & _guard293;
wire _guard295 = service_write_transfer_go_out;
wire _guard296 = _guard294 & _guard295;
wire _guard297 = wvalid_out;
wire _guard298 = WREADY;
wire _guard299 = _guard297 & _guard298;
wire _guard300 = ~_guard299;
wire _guard301 = service_write_transfer_go_out;
wire _guard302 = _guard300 & _guard301;
wire _guard303 = early_reset_static_par_thread_go_out;
wire _guard304 = _guard302 | _guard303;
wire _guard305 = fsm_out == 3'd5;
wire _guard306 = pd0_out;
wire _guard307 = curr_addr_internal_mem_incr_1_1_group_done_out;
wire _guard308 = _guard306 | _guard307;
wire _guard309 = ~_guard308;
wire _guard310 = par0_go_out;
wire _guard311 = _guard309 & _guard310;
wire _guard312 = par0_done_out;
wire _guard313 = ~_guard312;
wire _guard314 = fsm_out == 3'd4;
wire _guard315 = _guard313 & _guard314;
wire _guard316 = tdcc_go_out;
wire _guard317 = _guard315 & _guard316;
assign done = _guard1;
assign mem_ref_content_en = _guard2;
assign curr_addr_axi_write_en = _guard3;
assign WVALID = wvalid_out;
assign WDATA =
  _guard4 ? mem_ref_read_data :
  32'd0;
assign curr_addr_axi_in = curr_addr_axi_incr_4_2_out;
assign curr_addr_internal_mem_write_en = _guard8;
assign mem_ref_write_en = 1'd0;
assign WLAST =
  _guard11 ? 1'd1 :
  _guard14 ? 1'd0 :
  1'd0;
assign mem_ref_addr0 = curr_addr_internal_mem_out;
assign curr_addr_internal_mem_in =
  _guard16 ? curr_addr_internal_mem_incr_1_1_out :
  _guard17 ? 3'd0 :
  'x;
assign fsm_write_en = _guard70;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard87 ? 3'd5 :
  _guard102 ? 3'd2 :
  _guard107 ? 3'd4 :
  _guard112 ? 3'd1 :
  _guard113 ? 3'd0 :
  _guard118 ? 3'd3 :
  3'd0;
assign curr_transfer_count_incr_1_3_left = curr_transfer_count_out;
assign curr_transfer_count_incr_1_3_right = 8'd1;
assign wrapper_early_reset_static_par_thread_go_in = _guard126;
assign curr_addr_axi_incr_4_2_group_done_in = curr_addr_axi_done;
assign invoke2_go_in = _guard132;
assign curr_addr_axi_incr_4_2_left = curr_addr_axi_out;
assign curr_addr_axi_incr_4_2_right = 64'd4;
assign curr_transfer_count_write_en = _guard135;
assign curr_transfer_count_clk = clk;
assign curr_transfer_count_reset = reset;
assign curr_transfer_count_in = curr_transfer_count_incr_1_3_out;
assign curr_addr_axi_incr_4_2_group_go_in = _guard142;
assign curr_addr_internal_mem_incr_1_1_left = curr_addr_internal_mem_out;
assign curr_addr_internal_mem_incr_1_1_right = 3'd1;
assign wvalid_write_en = _guard145;
assign wvalid_clk = clk;
assign wvalid_reset = reset;
assign wvalid_in =
  _guard154 ? 1'd1 :
  _guard161 ? 1'd0 :
  'x;
assign n_finished_last_transfer_write_en = _guard170;
assign n_finished_last_transfer_clk = clk;
assign n_finished_last_transfer_reset = reset;
assign n_finished_last_transfer_in =
  _guard171 ? 1'd1 :
  _guard178 ? 1'd0 :
  'x;
assign pd1_write_en = _guard187;
assign pd1_clk = clk;
assign pd1_reset = reset;
assign pd1_in =
  _guard190 ? 1'd1 :
  _guard195 ? 1'd0 :
  1'd0;
assign tdcc_go_in = go;
assign invoke0_go_in = _guard201;
assign service_write_transfer_done_in = bt_reg_out;
assign par0_done_in = _guard206;
assign service_write_transfer_go_in = _guard212;
assign invoke0_done_in = curr_addr_internal_mem_done;
assign invoke1_go_in = _guard218;
assign bt_reg_write_en = _guard221;
assign bt_reg_clk = clk;
assign bt_reg_reset = reset;
assign bt_reg_in =
  _guard226 ? 1'd1 :
  _guard234 ? 1'd0 :
  'x;
assign signal_reg_write_en = _guard242;
assign signal_reg_clk = clk;
assign signal_reg_reset = reset;
assign signal_reg_in =
  _guard248 ? 1'd1 :
  _guard249 ? 1'd0 :
  1'd0;
assign early_reset_static_par_thread_go_in = _guard250;
assign curr_addr_internal_mem_incr_1_1_group_done_in = curr_addr_internal_mem_done;
assign invoke2_done_in = bt_reg_done;
assign pd_write_en = _guard259;
assign pd_clk = clk;
assign pd_reset = reset;
assign pd_in =
  _guard262 ? 1'd1 :
  _guard267 ? 1'd0 :
  1'd0;
assign pd0_write_en = _guard276;
assign pd0_clk = clk;
assign pd0_reset = reset;
assign pd0_in =
  _guard279 ? 1'd1 :
  _guard284 ? 1'd0 :
  1'd0;
assign wrapper_early_reset_static_par_thread_done_in = _guard285;
assign w_handshake_occurred_write_en = _guard291;
assign w_handshake_occurred_clk = clk;
assign w_handshake_occurred_reset = reset;
assign w_handshake_occurred_in =
  _guard296 ? 1'd1 :
  _guard304 ? 1'd0 :
  'x;
assign tdcc_done_in = _guard305;
assign curr_addr_internal_mem_incr_1_1_group_go_in = _guard311;
assign early_reset_static_par_thread_done_in = ud_out;
assign invoke1_done_in = n_finished_last_transfer_done;
assign par0_go_in = _guard317;
// COMPONENT END: m_write_channel
endmodule
module m_bresp_channel(
  input logic ARESETn,
  input logic BVALID,
  output logic BREADY,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: m_bresp_channel
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
// COMPONENT END: m_bresp_channel
endmodule
module wrapper(
  input logic ap_clk,
  input logic A0_ARESETn,
  input logic A0_ARREADY,
  input logic A0_RVALID,
  input logic A0_RLAST,
  input logic [31:0] A0_RDATA,
  input logic [1:0] A0_RRESP,
  input logic A0_AWREADY,
  input logic [1:0] A0_WRESP,
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
  input logic [1:0] B0_WRESP,
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
  input logic [1:0] Sum0_WRESP,
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
logic [2:0] curr_addr_internal_mem_A0_in;
logic curr_addr_internal_mem_A0_write_en;
logic curr_addr_internal_mem_A0_clk;
logic curr_addr_internal_mem_A0_reset;
logic [2:0] curr_addr_internal_mem_A0_out;
logic curr_addr_internal_mem_A0_done;
logic [63:0] curr_addr_axi_A0_in;
logic curr_addr_axi_A0_write_en;
logic curr_addr_axi_A0_clk;
logic curr_addr_axi_A0_reset;
logic [63:0] curr_addr_axi_A0_out;
logic curr_addr_axi_A0_done;
logic ar_channel_A0_ARESETn;
logic ar_channel_A0_ARREADY;
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
logic [63:0] ar_channel_A0_curr_addr_axi_out;
logic ar_channel_A0_curr_addr_axi_write_en;
logic [63:0] ar_channel_A0_curr_addr_axi_in;
logic ar_channel_A0_curr_addr_axi_done;
logic read_channel_A0_ARESETn;
logic read_channel_A0_RVALID;
logic read_channel_A0_RLAST;
logic [31:0] read_channel_A0_RDATA;
logic [1:0] read_channel_A0_RRESP;
logic read_channel_A0_RREADY;
logic read_channel_A0_go;
logic read_channel_A0_clk;
logic read_channel_A0_reset;
logic read_channel_A0_done;
logic read_channel_A0_mem_ref_content_en;
logic [2:0] read_channel_A0_curr_addr_internal_mem_out;
logic [63:0] read_channel_A0_curr_addr_axi_out;
logic read_channel_A0_curr_addr_axi_write_en;
logic [63:0] read_channel_A0_curr_addr_axi_in;
logic read_channel_A0_curr_addr_internal_mem_write_en;
logic [31:0] read_channel_A0_mem_ref_write_data;
logic read_channel_A0_mem_ref_write_en;
logic [31:0] read_channel_A0_mem_ref_read_data;
logic read_channel_A0_mem_ref_done;
logic [2:0] read_channel_A0_mem_ref_addr0;
logic [2:0] read_channel_A0_curr_addr_internal_mem_in;
logic read_channel_A0_curr_addr_internal_mem_done;
logic read_channel_A0_curr_addr_axi_done;
logic internal_mem_A0_clk;
logic internal_mem_A0_reset;
logic [2:0] internal_mem_A0_addr0;
logic internal_mem_A0_content_en;
logic internal_mem_A0_write_en;
logic [31:0] internal_mem_A0_write_data;
logic [31:0] internal_mem_A0_read_data;
logic internal_mem_A0_done;
logic [7:0] max_transfers_A0_in;
logic max_transfers_A0_write_en;
logic max_transfers_A0_clk;
logic max_transfers_A0_reset;
logic [7:0] max_transfers_A0_out;
logic max_transfers_A0_done;
logic aw_channel_A0_ARESETn;
logic aw_channel_A0_AWREADY;
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
logic [63:0] aw_channel_A0_curr_addr_axi_out;
logic [7:0] aw_channel_A0_max_transfers_out;
logic aw_channel_A0_curr_addr_axi_write_en;
logic [63:0] aw_channel_A0_curr_addr_axi_in;
logic aw_channel_A0_max_transfers_done;
logic [7:0] aw_channel_A0_max_transfers_in;
logic aw_channel_A0_max_transfers_write_en;
logic aw_channel_A0_curr_addr_axi_done;
logic write_channel_A0_ARESETn;
logic write_channel_A0_WREADY;
logic write_channel_A0_WVALID;
logic write_channel_A0_WLAST;
logic [31:0] write_channel_A0_WDATA;
logic write_channel_A0_go;
logic write_channel_A0_clk;
logic write_channel_A0_reset;
logic write_channel_A0_done;
logic write_channel_A0_mem_ref_content_en;
logic [2:0] write_channel_A0_curr_addr_internal_mem_out;
logic [63:0] write_channel_A0_curr_addr_axi_out;
logic [7:0] write_channel_A0_max_transfers_out;
logic write_channel_A0_curr_addr_axi_write_en;
logic [63:0] write_channel_A0_curr_addr_axi_in;
logic write_channel_A0_max_transfers_done;
logic write_channel_A0_curr_addr_internal_mem_write_en;
logic [31:0] write_channel_A0_mem_ref_write_data;
logic write_channel_A0_mem_ref_write_en;
logic [31:0] write_channel_A0_mem_ref_read_data;
logic [7:0] write_channel_A0_max_transfers_in;
logic write_channel_A0_mem_ref_done;
logic [2:0] write_channel_A0_mem_ref_addr0;
logic [2:0] write_channel_A0_curr_addr_internal_mem_in;
logic write_channel_A0_max_transfers_write_en;
logic write_channel_A0_curr_addr_internal_mem_done;
logic write_channel_A0_curr_addr_axi_done;
logic bresp_channel_A0_ARESETn;
logic bresp_channel_A0_BVALID;
logic bresp_channel_A0_BREADY;
logic bresp_channel_A0_go;
logic bresp_channel_A0_clk;
logic bresp_channel_A0_reset;
logic bresp_channel_A0_done;
logic [2:0] curr_addr_internal_mem_B0_in;
logic curr_addr_internal_mem_B0_write_en;
logic curr_addr_internal_mem_B0_clk;
logic curr_addr_internal_mem_B0_reset;
logic [2:0] curr_addr_internal_mem_B0_out;
logic curr_addr_internal_mem_B0_done;
logic [63:0] curr_addr_axi_B0_in;
logic curr_addr_axi_B0_write_en;
logic curr_addr_axi_B0_clk;
logic curr_addr_axi_B0_reset;
logic [63:0] curr_addr_axi_B0_out;
logic curr_addr_axi_B0_done;
logic ar_channel_B0_ARESETn;
logic ar_channel_B0_ARREADY;
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
logic [63:0] ar_channel_B0_curr_addr_axi_out;
logic ar_channel_B0_curr_addr_axi_write_en;
logic [63:0] ar_channel_B0_curr_addr_axi_in;
logic ar_channel_B0_curr_addr_axi_done;
logic read_channel_B0_ARESETn;
logic read_channel_B0_RVALID;
logic read_channel_B0_RLAST;
logic [31:0] read_channel_B0_RDATA;
logic [1:0] read_channel_B0_RRESP;
logic read_channel_B0_RREADY;
logic read_channel_B0_go;
logic read_channel_B0_clk;
logic read_channel_B0_reset;
logic read_channel_B0_done;
logic read_channel_B0_mem_ref_content_en;
logic [2:0] read_channel_B0_curr_addr_internal_mem_out;
logic [63:0] read_channel_B0_curr_addr_axi_out;
logic read_channel_B0_curr_addr_axi_write_en;
logic [63:0] read_channel_B0_curr_addr_axi_in;
logic read_channel_B0_curr_addr_internal_mem_write_en;
logic [31:0] read_channel_B0_mem_ref_write_data;
logic read_channel_B0_mem_ref_write_en;
logic [31:0] read_channel_B0_mem_ref_read_data;
logic read_channel_B0_mem_ref_done;
logic [2:0] read_channel_B0_mem_ref_addr0;
logic [2:0] read_channel_B0_curr_addr_internal_mem_in;
logic read_channel_B0_curr_addr_internal_mem_done;
logic read_channel_B0_curr_addr_axi_done;
logic internal_mem_B0_clk;
logic internal_mem_B0_reset;
logic [2:0] internal_mem_B0_addr0;
logic internal_mem_B0_content_en;
logic internal_mem_B0_write_en;
logic [31:0] internal_mem_B0_write_data;
logic [31:0] internal_mem_B0_read_data;
logic internal_mem_B0_done;
logic [7:0] max_transfers_B0_in;
logic max_transfers_B0_write_en;
logic max_transfers_B0_clk;
logic max_transfers_B0_reset;
logic [7:0] max_transfers_B0_out;
logic max_transfers_B0_done;
logic aw_channel_B0_ARESETn;
logic aw_channel_B0_AWREADY;
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
logic [63:0] aw_channel_B0_curr_addr_axi_out;
logic [7:0] aw_channel_B0_max_transfers_out;
logic aw_channel_B0_curr_addr_axi_write_en;
logic [63:0] aw_channel_B0_curr_addr_axi_in;
logic aw_channel_B0_max_transfers_done;
logic [7:0] aw_channel_B0_max_transfers_in;
logic aw_channel_B0_max_transfers_write_en;
logic aw_channel_B0_curr_addr_axi_done;
logic write_channel_B0_ARESETn;
logic write_channel_B0_WREADY;
logic write_channel_B0_WVALID;
logic write_channel_B0_WLAST;
logic [31:0] write_channel_B0_WDATA;
logic write_channel_B0_go;
logic write_channel_B0_clk;
logic write_channel_B0_reset;
logic write_channel_B0_done;
logic write_channel_B0_mem_ref_content_en;
logic [2:0] write_channel_B0_curr_addr_internal_mem_out;
logic [63:0] write_channel_B0_curr_addr_axi_out;
logic [7:0] write_channel_B0_max_transfers_out;
logic write_channel_B0_curr_addr_axi_write_en;
logic [63:0] write_channel_B0_curr_addr_axi_in;
logic write_channel_B0_max_transfers_done;
logic write_channel_B0_curr_addr_internal_mem_write_en;
logic [31:0] write_channel_B0_mem_ref_write_data;
logic write_channel_B0_mem_ref_write_en;
logic [31:0] write_channel_B0_mem_ref_read_data;
logic [7:0] write_channel_B0_max_transfers_in;
logic write_channel_B0_mem_ref_done;
logic [2:0] write_channel_B0_mem_ref_addr0;
logic [2:0] write_channel_B0_curr_addr_internal_mem_in;
logic write_channel_B0_max_transfers_write_en;
logic write_channel_B0_curr_addr_internal_mem_done;
logic write_channel_B0_curr_addr_axi_done;
logic bresp_channel_B0_ARESETn;
logic bresp_channel_B0_BVALID;
logic bresp_channel_B0_BREADY;
logic bresp_channel_B0_go;
logic bresp_channel_B0_clk;
logic bresp_channel_B0_reset;
logic bresp_channel_B0_done;
logic [2:0] curr_addr_internal_mem_Sum0_in;
logic curr_addr_internal_mem_Sum0_write_en;
logic curr_addr_internal_mem_Sum0_clk;
logic curr_addr_internal_mem_Sum0_reset;
logic [2:0] curr_addr_internal_mem_Sum0_out;
logic curr_addr_internal_mem_Sum0_done;
logic [63:0] curr_addr_axi_Sum0_in;
logic curr_addr_axi_Sum0_write_en;
logic curr_addr_axi_Sum0_clk;
logic curr_addr_axi_Sum0_reset;
logic [63:0] curr_addr_axi_Sum0_out;
logic curr_addr_axi_Sum0_done;
logic ar_channel_Sum0_ARESETn;
logic ar_channel_Sum0_ARREADY;
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
logic [63:0] ar_channel_Sum0_curr_addr_axi_out;
logic ar_channel_Sum0_curr_addr_axi_write_en;
logic [63:0] ar_channel_Sum0_curr_addr_axi_in;
logic ar_channel_Sum0_curr_addr_axi_done;
logic read_channel_Sum0_ARESETn;
logic read_channel_Sum0_RVALID;
logic read_channel_Sum0_RLAST;
logic [31:0] read_channel_Sum0_RDATA;
logic [1:0] read_channel_Sum0_RRESP;
logic read_channel_Sum0_RREADY;
logic read_channel_Sum0_go;
logic read_channel_Sum0_clk;
logic read_channel_Sum0_reset;
logic read_channel_Sum0_done;
logic read_channel_Sum0_mem_ref_content_en;
logic [2:0] read_channel_Sum0_curr_addr_internal_mem_out;
logic [63:0] read_channel_Sum0_curr_addr_axi_out;
logic read_channel_Sum0_curr_addr_axi_write_en;
logic [63:0] read_channel_Sum0_curr_addr_axi_in;
logic read_channel_Sum0_curr_addr_internal_mem_write_en;
logic [31:0] read_channel_Sum0_mem_ref_write_data;
logic read_channel_Sum0_mem_ref_write_en;
logic [31:0] read_channel_Sum0_mem_ref_read_data;
logic read_channel_Sum0_mem_ref_done;
logic [2:0] read_channel_Sum0_mem_ref_addr0;
logic [2:0] read_channel_Sum0_curr_addr_internal_mem_in;
logic read_channel_Sum0_curr_addr_internal_mem_done;
logic read_channel_Sum0_curr_addr_axi_done;
logic internal_mem_Sum0_clk;
logic internal_mem_Sum0_reset;
logic [2:0] internal_mem_Sum0_addr0;
logic internal_mem_Sum0_content_en;
logic internal_mem_Sum0_write_en;
logic [31:0] internal_mem_Sum0_write_data;
logic [31:0] internal_mem_Sum0_read_data;
logic internal_mem_Sum0_done;
logic [7:0] max_transfers_Sum0_in;
logic max_transfers_Sum0_write_en;
logic max_transfers_Sum0_clk;
logic max_transfers_Sum0_reset;
logic [7:0] max_transfers_Sum0_out;
logic max_transfers_Sum0_done;
logic aw_channel_Sum0_ARESETn;
logic aw_channel_Sum0_AWREADY;
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
logic [63:0] aw_channel_Sum0_curr_addr_axi_out;
logic [7:0] aw_channel_Sum0_max_transfers_out;
logic aw_channel_Sum0_curr_addr_axi_write_en;
logic [63:0] aw_channel_Sum0_curr_addr_axi_in;
logic aw_channel_Sum0_max_transfers_done;
logic [7:0] aw_channel_Sum0_max_transfers_in;
logic aw_channel_Sum0_max_transfers_write_en;
logic aw_channel_Sum0_curr_addr_axi_done;
logic write_channel_Sum0_ARESETn;
logic write_channel_Sum0_WREADY;
logic write_channel_Sum0_WVALID;
logic write_channel_Sum0_WLAST;
logic [31:0] write_channel_Sum0_WDATA;
logic write_channel_Sum0_go;
logic write_channel_Sum0_clk;
logic write_channel_Sum0_reset;
logic write_channel_Sum0_done;
logic write_channel_Sum0_mem_ref_content_en;
logic [2:0] write_channel_Sum0_curr_addr_internal_mem_out;
logic [63:0] write_channel_Sum0_curr_addr_axi_out;
logic [7:0] write_channel_Sum0_max_transfers_out;
logic write_channel_Sum0_curr_addr_axi_write_en;
logic [63:0] write_channel_Sum0_curr_addr_axi_in;
logic write_channel_Sum0_max_transfers_done;
logic write_channel_Sum0_curr_addr_internal_mem_write_en;
logic [31:0] write_channel_Sum0_mem_ref_write_data;
logic write_channel_Sum0_mem_ref_write_en;
logic [31:0] write_channel_Sum0_mem_ref_read_data;
logic [7:0] write_channel_Sum0_max_transfers_in;
logic write_channel_Sum0_mem_ref_done;
logic [2:0] write_channel_Sum0_mem_ref_addr0;
logic [2:0] write_channel_Sum0_curr_addr_internal_mem_in;
logic write_channel_Sum0_max_transfers_write_en;
logic write_channel_Sum0_curr_addr_internal_mem_done;
logic write_channel_Sum0_curr_addr_axi_done;
logic bresp_channel_Sum0_ARESETn;
logic bresp_channel_Sum0_BVALID;
logic bresp_channel_Sum0_BREADY;
logic bresp_channel_Sum0_go;
logic bresp_channel_Sum0_clk;
logic bresp_channel_Sum0_reset;
logic bresp_channel_Sum0_done;
logic ud_out;
logic ud0_out;
logic signal_reg_in;
logic signal_reg_write_en;
logic signal_reg_clk;
logic signal_reg_reset;
logic signal_reg_out;
logic signal_reg_done;
logic signal_reg0_in;
logic signal_reg0_write_en;
logic signal_reg0_clk;
logic signal_reg0_reset;
logic signal_reg0_out;
logic signal_reg0_done;
logic [1:0] fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic [1:0] fsm_out;
logic fsm_done;
logic pd_in;
logic pd_write_en;
logic pd_clk;
logic pd_reset;
logic pd_out;
logic pd_done;
logic [1:0] fsm0_in;
logic fsm0_write_en;
logic fsm0_clk;
logic fsm0_reset;
logic [1:0] fsm0_out;
logic fsm0_done;
logic pd0_in;
logic pd0_write_en;
logic pd0_clk;
logic pd0_reset;
logic pd0_out;
logic pd0_done;
logic [1:0] fsm1_in;
logic fsm1_write_en;
logic fsm1_clk;
logic fsm1_reset;
logic [1:0] fsm1_out;
logic fsm1_done;
logic pd1_in;
logic pd1_write_en;
logic pd1_clk;
logic pd1_reset;
logic pd1_out;
logic pd1_done;
logic [1:0] fsm2_in;
logic fsm2_write_en;
logic fsm2_clk;
logic fsm2_reset;
logic [1:0] fsm2_out;
logic fsm2_done;
logic pd2_in;
logic pd2_write_en;
logic pd2_clk;
logic pd2_reset;
logic pd2_out;
logic pd2_done;
logic [1:0] fsm3_in;
logic fsm3_write_en;
logic fsm3_clk;
logic fsm3_reset;
logic [1:0] fsm3_out;
logic fsm3_done;
logic pd3_in;
logic pd3_write_en;
logic pd3_clk;
logic pd3_reset;
logic pd3_out;
logic pd3_done;
logic [1:0] fsm4_in;
logic fsm4_write_en;
logic fsm4_clk;
logic fsm4_reset;
logic [1:0] fsm4_out;
logic fsm4_done;
logic pd4_in;
logic pd4_write_en;
logic pd4_clk;
logic pd4_reset;
logic pd4_out;
logic pd4_done;
logic [2:0] fsm5_in;
logic fsm5_write_en;
logic fsm5_clk;
logic fsm5_reset;
logic [2:0] fsm5_out;
logic fsm5_done;
logic invoke6_go_in;
logic invoke6_go_out;
logic invoke6_done_in;
logic invoke6_done_out;
logic invoke7_go_in;
logic invoke7_go_out;
logic invoke7_done_in;
logic invoke7_done_out;
logic invoke8_go_in;
logic invoke8_go_out;
logic invoke8_done_in;
logic invoke8_done_out;
logic invoke9_go_in;
logic invoke9_go_out;
logic invoke9_done_in;
logic invoke9_done_out;
logic invoke10_go_in;
logic invoke10_go_out;
logic invoke10_done_in;
logic invoke10_done_out;
logic invoke11_go_in;
logic invoke11_go_out;
logic invoke11_done_in;
logic invoke11_done_out;
logic invoke12_go_in;
logic invoke12_go_out;
logic invoke12_done_in;
logic invoke12_done_out;
logic invoke16_go_in;
logic invoke16_go_out;
logic invoke16_done_in;
logic invoke16_done_out;
logic invoke17_go_in;
logic invoke17_go_out;
logic invoke17_done_in;
logic invoke17_done_out;
logic invoke18_go_in;
logic invoke18_go_out;
logic invoke18_done_in;
logic invoke18_done_out;
logic invoke19_go_in;
logic invoke19_go_out;
logic invoke19_done_in;
logic invoke19_done_out;
logic invoke20_go_in;
logic invoke20_go_out;
logic invoke20_done_in;
logic invoke20_done_out;
logic invoke21_go_in;
logic invoke21_go_out;
logic invoke21_done_in;
logic invoke21_done_out;
logic invoke22_go_in;
logic invoke22_go_out;
logic invoke22_done_in;
logic invoke22_done_out;
logic invoke23_go_in;
logic invoke23_go_out;
logic invoke23_done_in;
logic invoke23_done_out;
logic invoke24_go_in;
logic invoke24_go_out;
logic invoke24_done_in;
logic invoke24_done_out;
logic early_reset_static_par_thread_go_in;
logic early_reset_static_par_thread_go_out;
logic early_reset_static_par_thread_done_in;
logic early_reset_static_par_thread_done_out;
logic early_reset_static_par_thread0_go_in;
logic early_reset_static_par_thread0_go_out;
logic early_reset_static_par_thread0_done_in;
logic early_reset_static_par_thread0_done_out;
logic wrapper_early_reset_static_par_thread_go_in;
logic wrapper_early_reset_static_par_thread_go_out;
logic wrapper_early_reset_static_par_thread_done_in;
logic wrapper_early_reset_static_par_thread_done_out;
logic wrapper_early_reset_static_par_thread0_go_in;
logic wrapper_early_reset_static_par_thread0_go_out;
logic wrapper_early_reset_static_par_thread0_done_in;
logic wrapper_early_reset_static_par_thread0_done_out;
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
logic par1_go_in;
logic par1_go_out;
logic par1_done_in;
logic par1_done_out;
logic tdcc2_go_in;
logic tdcc2_go_out;
logic tdcc2_done_in;
logic tdcc2_done_out;
logic tdcc3_go_in;
logic tdcc3_go_out;
logic tdcc3_done_in;
logic tdcc3_done_out;
logic tdcc4_go_in;
logic tdcc4_go_out;
logic tdcc4_done_in;
logic tdcc4_done_out;
logic tdcc5_go_in;
logic tdcc5_go_out;
logic tdcc5_done_in;
logic tdcc5_done_out;
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
std_reg # (
    .WIDTH(3)
) curr_addr_internal_mem_A0 (
    .clk(curr_addr_internal_mem_A0_clk),
    .done(curr_addr_internal_mem_A0_done),
    .in(curr_addr_internal_mem_A0_in),
    .out(curr_addr_internal_mem_A0_out),
    .reset(curr_addr_internal_mem_A0_reset),
    .write_en(curr_addr_internal_mem_A0_write_en)
);
std_reg # (
    .WIDTH(64)
) curr_addr_axi_A0 (
    .clk(curr_addr_axi_A0_clk),
    .done(curr_addr_axi_A0_done),
    .in(curr_addr_axi_A0_in),
    .out(curr_addr_axi_A0_out),
    .reset(curr_addr_axi_A0_reset),
    .write_en(curr_addr_axi_A0_write_en)
);
m_ar_channel ar_channel_A0 (
    .ARADDR(ar_channel_A0_ARADDR),
    .ARBURST(ar_channel_A0_ARBURST),
    .ARESETn(ar_channel_A0_ARESETn),
    .ARLEN(ar_channel_A0_ARLEN),
    .ARPROT(ar_channel_A0_ARPROT),
    .ARREADY(ar_channel_A0_ARREADY),
    .ARSIZE(ar_channel_A0_ARSIZE),
    .ARVALID(ar_channel_A0_ARVALID),
    .clk(ar_channel_A0_clk),
    .curr_addr_axi_done(ar_channel_A0_curr_addr_axi_done),
    .curr_addr_axi_in(ar_channel_A0_curr_addr_axi_in),
    .curr_addr_axi_out(ar_channel_A0_curr_addr_axi_out),
    .curr_addr_axi_write_en(ar_channel_A0_curr_addr_axi_write_en),
    .done(ar_channel_A0_done),
    .go(ar_channel_A0_go),
    .reset(ar_channel_A0_reset)
);
m_read_channel read_channel_A0 (
    .ARESETn(read_channel_A0_ARESETn),
    .RDATA(read_channel_A0_RDATA),
    .RLAST(read_channel_A0_RLAST),
    .RREADY(read_channel_A0_RREADY),
    .RRESP(read_channel_A0_RRESP),
    .RVALID(read_channel_A0_RVALID),
    .clk(read_channel_A0_clk),
    .curr_addr_axi_done(read_channel_A0_curr_addr_axi_done),
    .curr_addr_axi_in(read_channel_A0_curr_addr_axi_in),
    .curr_addr_axi_out(read_channel_A0_curr_addr_axi_out),
    .curr_addr_axi_write_en(read_channel_A0_curr_addr_axi_write_en),
    .curr_addr_internal_mem_done(read_channel_A0_curr_addr_internal_mem_done),
    .curr_addr_internal_mem_in(read_channel_A0_curr_addr_internal_mem_in),
    .curr_addr_internal_mem_out(read_channel_A0_curr_addr_internal_mem_out),
    .curr_addr_internal_mem_write_en(read_channel_A0_curr_addr_internal_mem_write_en),
    .done(read_channel_A0_done),
    .go(read_channel_A0_go),
    .mem_ref_addr0(read_channel_A0_mem_ref_addr0),
    .mem_ref_content_en(read_channel_A0_mem_ref_content_en),
    .mem_ref_done(read_channel_A0_mem_ref_done),
    .mem_ref_read_data(read_channel_A0_mem_ref_read_data),
    .mem_ref_write_data(read_channel_A0_mem_ref_write_data),
    .mem_ref_write_en(read_channel_A0_mem_ref_write_en),
    .reset(read_channel_A0_reset)
);
seq_mem_d1 # (
    .IDX_SIZE(3),
    .SIZE(8),
    .WIDTH(32)
) internal_mem_A0 (
    .addr0(internal_mem_A0_addr0),
    .clk(internal_mem_A0_clk),
    .content_en(internal_mem_A0_content_en),
    .done(internal_mem_A0_done),
    .read_data(internal_mem_A0_read_data),
    .reset(internal_mem_A0_reset),
    .write_data(internal_mem_A0_write_data),
    .write_en(internal_mem_A0_write_en)
);
std_reg # (
    .WIDTH(8)
) max_transfers_A0 (
    .clk(max_transfers_A0_clk),
    .done(max_transfers_A0_done),
    .in(max_transfers_A0_in),
    .out(max_transfers_A0_out),
    .reset(max_transfers_A0_reset),
    .write_en(max_transfers_A0_write_en)
);
m_aw_channel aw_channel_A0 (
    .ARESETn(aw_channel_A0_ARESETn),
    .AWADDR(aw_channel_A0_AWADDR),
    .AWBURST(aw_channel_A0_AWBURST),
    .AWLEN(aw_channel_A0_AWLEN),
    .AWPROT(aw_channel_A0_AWPROT),
    .AWREADY(aw_channel_A0_AWREADY),
    .AWSIZE(aw_channel_A0_AWSIZE),
    .AWVALID(aw_channel_A0_AWVALID),
    .clk(aw_channel_A0_clk),
    .curr_addr_axi_done(aw_channel_A0_curr_addr_axi_done),
    .curr_addr_axi_in(aw_channel_A0_curr_addr_axi_in),
    .curr_addr_axi_out(aw_channel_A0_curr_addr_axi_out),
    .curr_addr_axi_write_en(aw_channel_A0_curr_addr_axi_write_en),
    .done(aw_channel_A0_done),
    .go(aw_channel_A0_go),
    .max_transfers_done(aw_channel_A0_max_transfers_done),
    .max_transfers_in(aw_channel_A0_max_transfers_in),
    .max_transfers_out(aw_channel_A0_max_transfers_out),
    .max_transfers_write_en(aw_channel_A0_max_transfers_write_en),
    .reset(aw_channel_A0_reset)
);
m_write_channel write_channel_A0 (
    .ARESETn(write_channel_A0_ARESETn),
    .WDATA(write_channel_A0_WDATA),
    .WLAST(write_channel_A0_WLAST),
    .WREADY(write_channel_A0_WREADY),
    .WVALID(write_channel_A0_WVALID),
    .clk(write_channel_A0_clk),
    .curr_addr_axi_done(write_channel_A0_curr_addr_axi_done),
    .curr_addr_axi_in(write_channel_A0_curr_addr_axi_in),
    .curr_addr_axi_out(write_channel_A0_curr_addr_axi_out),
    .curr_addr_axi_write_en(write_channel_A0_curr_addr_axi_write_en),
    .curr_addr_internal_mem_done(write_channel_A0_curr_addr_internal_mem_done),
    .curr_addr_internal_mem_in(write_channel_A0_curr_addr_internal_mem_in),
    .curr_addr_internal_mem_out(write_channel_A0_curr_addr_internal_mem_out),
    .curr_addr_internal_mem_write_en(write_channel_A0_curr_addr_internal_mem_write_en),
    .done(write_channel_A0_done),
    .go(write_channel_A0_go),
    .max_transfers_done(write_channel_A0_max_transfers_done),
    .max_transfers_in(write_channel_A0_max_transfers_in),
    .max_transfers_out(write_channel_A0_max_transfers_out),
    .max_transfers_write_en(write_channel_A0_max_transfers_write_en),
    .mem_ref_addr0(write_channel_A0_mem_ref_addr0),
    .mem_ref_content_en(write_channel_A0_mem_ref_content_en),
    .mem_ref_done(write_channel_A0_mem_ref_done),
    .mem_ref_read_data(write_channel_A0_mem_ref_read_data),
    .mem_ref_write_data(write_channel_A0_mem_ref_write_data),
    .mem_ref_write_en(write_channel_A0_mem_ref_write_en),
    .reset(write_channel_A0_reset)
);
m_bresp_channel bresp_channel_A0 (
    .ARESETn(bresp_channel_A0_ARESETn),
    .BREADY(bresp_channel_A0_BREADY),
    .BVALID(bresp_channel_A0_BVALID),
    .clk(bresp_channel_A0_clk),
    .done(bresp_channel_A0_done),
    .go(bresp_channel_A0_go),
    .reset(bresp_channel_A0_reset)
);
std_reg # (
    .WIDTH(3)
) curr_addr_internal_mem_B0 (
    .clk(curr_addr_internal_mem_B0_clk),
    .done(curr_addr_internal_mem_B0_done),
    .in(curr_addr_internal_mem_B0_in),
    .out(curr_addr_internal_mem_B0_out),
    .reset(curr_addr_internal_mem_B0_reset),
    .write_en(curr_addr_internal_mem_B0_write_en)
);
std_reg # (
    .WIDTH(64)
) curr_addr_axi_B0 (
    .clk(curr_addr_axi_B0_clk),
    .done(curr_addr_axi_B0_done),
    .in(curr_addr_axi_B0_in),
    .out(curr_addr_axi_B0_out),
    .reset(curr_addr_axi_B0_reset),
    .write_en(curr_addr_axi_B0_write_en)
);
m_ar_channel ar_channel_B0 (
    .ARADDR(ar_channel_B0_ARADDR),
    .ARBURST(ar_channel_B0_ARBURST),
    .ARESETn(ar_channel_B0_ARESETn),
    .ARLEN(ar_channel_B0_ARLEN),
    .ARPROT(ar_channel_B0_ARPROT),
    .ARREADY(ar_channel_B0_ARREADY),
    .ARSIZE(ar_channel_B0_ARSIZE),
    .ARVALID(ar_channel_B0_ARVALID),
    .clk(ar_channel_B0_clk),
    .curr_addr_axi_done(ar_channel_B0_curr_addr_axi_done),
    .curr_addr_axi_in(ar_channel_B0_curr_addr_axi_in),
    .curr_addr_axi_out(ar_channel_B0_curr_addr_axi_out),
    .curr_addr_axi_write_en(ar_channel_B0_curr_addr_axi_write_en),
    .done(ar_channel_B0_done),
    .go(ar_channel_B0_go),
    .reset(ar_channel_B0_reset)
);
m_read_channel read_channel_B0 (
    .ARESETn(read_channel_B0_ARESETn),
    .RDATA(read_channel_B0_RDATA),
    .RLAST(read_channel_B0_RLAST),
    .RREADY(read_channel_B0_RREADY),
    .RRESP(read_channel_B0_RRESP),
    .RVALID(read_channel_B0_RVALID),
    .clk(read_channel_B0_clk),
    .curr_addr_axi_done(read_channel_B0_curr_addr_axi_done),
    .curr_addr_axi_in(read_channel_B0_curr_addr_axi_in),
    .curr_addr_axi_out(read_channel_B0_curr_addr_axi_out),
    .curr_addr_axi_write_en(read_channel_B0_curr_addr_axi_write_en),
    .curr_addr_internal_mem_done(read_channel_B0_curr_addr_internal_mem_done),
    .curr_addr_internal_mem_in(read_channel_B0_curr_addr_internal_mem_in),
    .curr_addr_internal_mem_out(read_channel_B0_curr_addr_internal_mem_out),
    .curr_addr_internal_mem_write_en(read_channel_B0_curr_addr_internal_mem_write_en),
    .done(read_channel_B0_done),
    .go(read_channel_B0_go),
    .mem_ref_addr0(read_channel_B0_mem_ref_addr0),
    .mem_ref_content_en(read_channel_B0_mem_ref_content_en),
    .mem_ref_done(read_channel_B0_mem_ref_done),
    .mem_ref_read_data(read_channel_B0_mem_ref_read_data),
    .mem_ref_write_data(read_channel_B0_mem_ref_write_data),
    .mem_ref_write_en(read_channel_B0_mem_ref_write_en),
    .reset(read_channel_B0_reset)
);
seq_mem_d1 # (
    .IDX_SIZE(3),
    .SIZE(8),
    .WIDTH(32)
) internal_mem_B0 (
    .addr0(internal_mem_B0_addr0),
    .clk(internal_mem_B0_clk),
    .content_en(internal_mem_B0_content_en),
    .done(internal_mem_B0_done),
    .read_data(internal_mem_B0_read_data),
    .reset(internal_mem_B0_reset),
    .write_data(internal_mem_B0_write_data),
    .write_en(internal_mem_B0_write_en)
);
std_reg # (
    .WIDTH(8)
) max_transfers_B0 (
    .clk(max_transfers_B0_clk),
    .done(max_transfers_B0_done),
    .in(max_transfers_B0_in),
    .out(max_transfers_B0_out),
    .reset(max_transfers_B0_reset),
    .write_en(max_transfers_B0_write_en)
);
m_aw_channel aw_channel_B0 (
    .ARESETn(aw_channel_B0_ARESETn),
    .AWADDR(aw_channel_B0_AWADDR),
    .AWBURST(aw_channel_B0_AWBURST),
    .AWLEN(aw_channel_B0_AWLEN),
    .AWPROT(aw_channel_B0_AWPROT),
    .AWREADY(aw_channel_B0_AWREADY),
    .AWSIZE(aw_channel_B0_AWSIZE),
    .AWVALID(aw_channel_B0_AWVALID),
    .clk(aw_channel_B0_clk),
    .curr_addr_axi_done(aw_channel_B0_curr_addr_axi_done),
    .curr_addr_axi_in(aw_channel_B0_curr_addr_axi_in),
    .curr_addr_axi_out(aw_channel_B0_curr_addr_axi_out),
    .curr_addr_axi_write_en(aw_channel_B0_curr_addr_axi_write_en),
    .done(aw_channel_B0_done),
    .go(aw_channel_B0_go),
    .max_transfers_done(aw_channel_B0_max_transfers_done),
    .max_transfers_in(aw_channel_B0_max_transfers_in),
    .max_transfers_out(aw_channel_B0_max_transfers_out),
    .max_transfers_write_en(aw_channel_B0_max_transfers_write_en),
    .reset(aw_channel_B0_reset)
);
m_write_channel write_channel_B0 (
    .ARESETn(write_channel_B0_ARESETn),
    .WDATA(write_channel_B0_WDATA),
    .WLAST(write_channel_B0_WLAST),
    .WREADY(write_channel_B0_WREADY),
    .WVALID(write_channel_B0_WVALID),
    .clk(write_channel_B0_clk),
    .curr_addr_axi_done(write_channel_B0_curr_addr_axi_done),
    .curr_addr_axi_in(write_channel_B0_curr_addr_axi_in),
    .curr_addr_axi_out(write_channel_B0_curr_addr_axi_out),
    .curr_addr_axi_write_en(write_channel_B0_curr_addr_axi_write_en),
    .curr_addr_internal_mem_done(write_channel_B0_curr_addr_internal_mem_done),
    .curr_addr_internal_mem_in(write_channel_B0_curr_addr_internal_mem_in),
    .curr_addr_internal_mem_out(write_channel_B0_curr_addr_internal_mem_out),
    .curr_addr_internal_mem_write_en(write_channel_B0_curr_addr_internal_mem_write_en),
    .done(write_channel_B0_done),
    .go(write_channel_B0_go),
    .max_transfers_done(write_channel_B0_max_transfers_done),
    .max_transfers_in(write_channel_B0_max_transfers_in),
    .max_transfers_out(write_channel_B0_max_transfers_out),
    .max_transfers_write_en(write_channel_B0_max_transfers_write_en),
    .mem_ref_addr0(write_channel_B0_mem_ref_addr0),
    .mem_ref_content_en(write_channel_B0_mem_ref_content_en),
    .mem_ref_done(write_channel_B0_mem_ref_done),
    .mem_ref_read_data(write_channel_B0_mem_ref_read_data),
    .mem_ref_write_data(write_channel_B0_mem_ref_write_data),
    .mem_ref_write_en(write_channel_B0_mem_ref_write_en),
    .reset(write_channel_B0_reset)
);
m_bresp_channel bresp_channel_B0 (
    .ARESETn(bresp_channel_B0_ARESETn),
    .BREADY(bresp_channel_B0_BREADY),
    .BVALID(bresp_channel_B0_BVALID),
    .clk(bresp_channel_B0_clk),
    .done(bresp_channel_B0_done),
    .go(bresp_channel_B0_go),
    .reset(bresp_channel_B0_reset)
);
std_reg # (
    .WIDTH(3)
) curr_addr_internal_mem_Sum0 (
    .clk(curr_addr_internal_mem_Sum0_clk),
    .done(curr_addr_internal_mem_Sum0_done),
    .in(curr_addr_internal_mem_Sum0_in),
    .out(curr_addr_internal_mem_Sum0_out),
    .reset(curr_addr_internal_mem_Sum0_reset),
    .write_en(curr_addr_internal_mem_Sum0_write_en)
);
std_reg # (
    .WIDTH(64)
) curr_addr_axi_Sum0 (
    .clk(curr_addr_axi_Sum0_clk),
    .done(curr_addr_axi_Sum0_done),
    .in(curr_addr_axi_Sum0_in),
    .out(curr_addr_axi_Sum0_out),
    .reset(curr_addr_axi_Sum0_reset),
    .write_en(curr_addr_axi_Sum0_write_en)
);
m_ar_channel ar_channel_Sum0 (
    .ARADDR(ar_channel_Sum0_ARADDR),
    .ARBURST(ar_channel_Sum0_ARBURST),
    .ARESETn(ar_channel_Sum0_ARESETn),
    .ARLEN(ar_channel_Sum0_ARLEN),
    .ARPROT(ar_channel_Sum0_ARPROT),
    .ARREADY(ar_channel_Sum0_ARREADY),
    .ARSIZE(ar_channel_Sum0_ARSIZE),
    .ARVALID(ar_channel_Sum0_ARVALID),
    .clk(ar_channel_Sum0_clk),
    .curr_addr_axi_done(ar_channel_Sum0_curr_addr_axi_done),
    .curr_addr_axi_in(ar_channel_Sum0_curr_addr_axi_in),
    .curr_addr_axi_out(ar_channel_Sum0_curr_addr_axi_out),
    .curr_addr_axi_write_en(ar_channel_Sum0_curr_addr_axi_write_en),
    .done(ar_channel_Sum0_done),
    .go(ar_channel_Sum0_go),
    .reset(ar_channel_Sum0_reset)
);
m_read_channel read_channel_Sum0 (
    .ARESETn(read_channel_Sum0_ARESETn),
    .RDATA(read_channel_Sum0_RDATA),
    .RLAST(read_channel_Sum0_RLAST),
    .RREADY(read_channel_Sum0_RREADY),
    .RRESP(read_channel_Sum0_RRESP),
    .RVALID(read_channel_Sum0_RVALID),
    .clk(read_channel_Sum0_clk),
    .curr_addr_axi_done(read_channel_Sum0_curr_addr_axi_done),
    .curr_addr_axi_in(read_channel_Sum0_curr_addr_axi_in),
    .curr_addr_axi_out(read_channel_Sum0_curr_addr_axi_out),
    .curr_addr_axi_write_en(read_channel_Sum0_curr_addr_axi_write_en),
    .curr_addr_internal_mem_done(read_channel_Sum0_curr_addr_internal_mem_done),
    .curr_addr_internal_mem_in(read_channel_Sum0_curr_addr_internal_mem_in),
    .curr_addr_internal_mem_out(read_channel_Sum0_curr_addr_internal_mem_out),
    .curr_addr_internal_mem_write_en(read_channel_Sum0_curr_addr_internal_mem_write_en),
    .done(read_channel_Sum0_done),
    .go(read_channel_Sum0_go),
    .mem_ref_addr0(read_channel_Sum0_mem_ref_addr0),
    .mem_ref_content_en(read_channel_Sum0_mem_ref_content_en),
    .mem_ref_done(read_channel_Sum0_mem_ref_done),
    .mem_ref_read_data(read_channel_Sum0_mem_ref_read_data),
    .mem_ref_write_data(read_channel_Sum0_mem_ref_write_data),
    .mem_ref_write_en(read_channel_Sum0_mem_ref_write_en),
    .reset(read_channel_Sum0_reset)
);
seq_mem_d1 # (
    .IDX_SIZE(3),
    .SIZE(8),
    .WIDTH(32)
) internal_mem_Sum0 (
    .addr0(internal_mem_Sum0_addr0),
    .clk(internal_mem_Sum0_clk),
    .content_en(internal_mem_Sum0_content_en),
    .done(internal_mem_Sum0_done),
    .read_data(internal_mem_Sum0_read_data),
    .reset(internal_mem_Sum0_reset),
    .write_data(internal_mem_Sum0_write_data),
    .write_en(internal_mem_Sum0_write_en)
);
std_reg # (
    .WIDTH(8)
) max_transfers_Sum0 (
    .clk(max_transfers_Sum0_clk),
    .done(max_transfers_Sum0_done),
    .in(max_transfers_Sum0_in),
    .out(max_transfers_Sum0_out),
    .reset(max_transfers_Sum0_reset),
    .write_en(max_transfers_Sum0_write_en)
);
m_aw_channel aw_channel_Sum0 (
    .ARESETn(aw_channel_Sum0_ARESETn),
    .AWADDR(aw_channel_Sum0_AWADDR),
    .AWBURST(aw_channel_Sum0_AWBURST),
    .AWLEN(aw_channel_Sum0_AWLEN),
    .AWPROT(aw_channel_Sum0_AWPROT),
    .AWREADY(aw_channel_Sum0_AWREADY),
    .AWSIZE(aw_channel_Sum0_AWSIZE),
    .AWVALID(aw_channel_Sum0_AWVALID),
    .clk(aw_channel_Sum0_clk),
    .curr_addr_axi_done(aw_channel_Sum0_curr_addr_axi_done),
    .curr_addr_axi_in(aw_channel_Sum0_curr_addr_axi_in),
    .curr_addr_axi_out(aw_channel_Sum0_curr_addr_axi_out),
    .curr_addr_axi_write_en(aw_channel_Sum0_curr_addr_axi_write_en),
    .done(aw_channel_Sum0_done),
    .go(aw_channel_Sum0_go),
    .max_transfers_done(aw_channel_Sum0_max_transfers_done),
    .max_transfers_in(aw_channel_Sum0_max_transfers_in),
    .max_transfers_out(aw_channel_Sum0_max_transfers_out),
    .max_transfers_write_en(aw_channel_Sum0_max_transfers_write_en),
    .reset(aw_channel_Sum0_reset)
);
m_write_channel write_channel_Sum0 (
    .ARESETn(write_channel_Sum0_ARESETn),
    .WDATA(write_channel_Sum0_WDATA),
    .WLAST(write_channel_Sum0_WLAST),
    .WREADY(write_channel_Sum0_WREADY),
    .WVALID(write_channel_Sum0_WVALID),
    .clk(write_channel_Sum0_clk),
    .curr_addr_axi_done(write_channel_Sum0_curr_addr_axi_done),
    .curr_addr_axi_in(write_channel_Sum0_curr_addr_axi_in),
    .curr_addr_axi_out(write_channel_Sum0_curr_addr_axi_out),
    .curr_addr_axi_write_en(write_channel_Sum0_curr_addr_axi_write_en),
    .curr_addr_internal_mem_done(write_channel_Sum0_curr_addr_internal_mem_done),
    .curr_addr_internal_mem_in(write_channel_Sum0_curr_addr_internal_mem_in),
    .curr_addr_internal_mem_out(write_channel_Sum0_curr_addr_internal_mem_out),
    .curr_addr_internal_mem_write_en(write_channel_Sum0_curr_addr_internal_mem_write_en),
    .done(write_channel_Sum0_done),
    .go(write_channel_Sum0_go),
    .max_transfers_done(write_channel_Sum0_max_transfers_done),
    .max_transfers_in(write_channel_Sum0_max_transfers_in),
    .max_transfers_out(write_channel_Sum0_max_transfers_out),
    .max_transfers_write_en(write_channel_Sum0_max_transfers_write_en),
    .mem_ref_addr0(write_channel_Sum0_mem_ref_addr0),
    .mem_ref_content_en(write_channel_Sum0_mem_ref_content_en),
    .mem_ref_done(write_channel_Sum0_mem_ref_done),
    .mem_ref_read_data(write_channel_Sum0_mem_ref_read_data),
    .mem_ref_write_data(write_channel_Sum0_mem_ref_write_data),
    .mem_ref_write_en(write_channel_Sum0_mem_ref_write_en),
    .reset(write_channel_Sum0_reset)
);
m_bresp_channel bresp_channel_Sum0 (
    .ARESETn(bresp_channel_Sum0_ARESETn),
    .BREADY(bresp_channel_Sum0_BREADY),
    .BVALID(bresp_channel_Sum0_BVALID),
    .clk(bresp_channel_Sum0_clk),
    .done(bresp_channel_Sum0_done),
    .go(bresp_channel_Sum0_go),
    .reset(bresp_channel_Sum0_reset)
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
    .WIDTH(1)
) signal_reg0 (
    .clk(signal_reg0_clk),
    .done(signal_reg0_done),
    .in(signal_reg0_in),
    .out(signal_reg0_out),
    .reset(signal_reg0_reset),
    .write_en(signal_reg0_write_en)
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
) pd0 (
    .clk(pd0_clk),
    .done(pd0_done),
    .in(pd0_in),
    .out(pd0_out),
    .reset(pd0_reset),
    .write_en(pd0_write_en)
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
) pd1 (
    .clk(pd1_clk),
    .done(pd1_done),
    .in(pd1_in),
    .out(pd1_out),
    .reset(pd1_reset),
    .write_en(pd1_write_en)
);
std_reg # (
    .WIDTH(2)
) fsm2 (
    .clk(fsm2_clk),
    .done(fsm2_done),
    .in(fsm2_in),
    .out(fsm2_out),
    .reset(fsm2_reset),
    .write_en(fsm2_write_en)
);
std_reg # (
    .WIDTH(1)
) pd2 (
    .clk(pd2_clk),
    .done(pd2_done),
    .in(pd2_in),
    .out(pd2_out),
    .reset(pd2_reset),
    .write_en(pd2_write_en)
);
std_reg # (
    .WIDTH(2)
) fsm3 (
    .clk(fsm3_clk),
    .done(fsm3_done),
    .in(fsm3_in),
    .out(fsm3_out),
    .reset(fsm3_reset),
    .write_en(fsm3_write_en)
);
std_reg # (
    .WIDTH(1)
) pd3 (
    .clk(pd3_clk),
    .done(pd3_done),
    .in(pd3_in),
    .out(pd3_out),
    .reset(pd3_reset),
    .write_en(pd3_write_en)
);
std_reg # (
    .WIDTH(2)
) fsm4 (
    .clk(fsm4_clk),
    .done(fsm4_done),
    .in(fsm4_in),
    .out(fsm4_out),
    .reset(fsm4_reset),
    .write_en(fsm4_write_en)
);
std_reg # (
    .WIDTH(1)
) pd4 (
    .clk(pd4_clk),
    .done(pd4_done),
    .in(pd4_in),
    .out(pd4_out),
    .reset(pd4_reset),
    .write_en(pd4_write_en)
);
std_reg # (
    .WIDTH(3)
) fsm5 (
    .clk(fsm5_clk),
    .done(fsm5_done),
    .in(fsm5_in),
    .out(fsm5_out),
    .reset(fsm5_reset),
    .write_en(fsm5_write_en)
);
std_wire # (
    .WIDTH(1)
) invoke6_go (
    .in(invoke6_go_in),
    .out(invoke6_go_out)
);
std_wire # (
    .WIDTH(1)
) invoke6_done (
    .in(invoke6_done_in),
    .out(invoke6_done_out)
);
std_wire # (
    .WIDTH(1)
) invoke7_go (
    .in(invoke7_go_in),
    .out(invoke7_go_out)
);
std_wire # (
    .WIDTH(1)
) invoke7_done (
    .in(invoke7_done_in),
    .out(invoke7_done_out)
);
std_wire # (
    .WIDTH(1)
) invoke8_go (
    .in(invoke8_go_in),
    .out(invoke8_go_out)
);
std_wire # (
    .WIDTH(1)
) invoke8_done (
    .in(invoke8_done_in),
    .out(invoke8_done_out)
);
std_wire # (
    .WIDTH(1)
) invoke9_go (
    .in(invoke9_go_in),
    .out(invoke9_go_out)
);
std_wire # (
    .WIDTH(1)
) invoke9_done (
    .in(invoke9_done_in),
    .out(invoke9_done_out)
);
std_wire # (
    .WIDTH(1)
) invoke10_go (
    .in(invoke10_go_in),
    .out(invoke10_go_out)
);
std_wire # (
    .WIDTH(1)
) invoke10_done (
    .in(invoke10_done_in),
    .out(invoke10_done_out)
);
std_wire # (
    .WIDTH(1)
) invoke11_go (
    .in(invoke11_go_in),
    .out(invoke11_go_out)
);
std_wire # (
    .WIDTH(1)
) invoke11_done (
    .in(invoke11_done_in),
    .out(invoke11_done_out)
);
std_wire # (
    .WIDTH(1)
) invoke12_go (
    .in(invoke12_go_in),
    .out(invoke12_go_out)
);
std_wire # (
    .WIDTH(1)
) invoke12_done (
    .in(invoke12_done_in),
    .out(invoke12_done_out)
);
std_wire # (
    .WIDTH(1)
) invoke16_go (
    .in(invoke16_go_in),
    .out(invoke16_go_out)
);
std_wire # (
    .WIDTH(1)
) invoke16_done (
    .in(invoke16_done_in),
    .out(invoke16_done_out)
);
std_wire # (
    .WIDTH(1)
) invoke17_go (
    .in(invoke17_go_in),
    .out(invoke17_go_out)
);
std_wire # (
    .WIDTH(1)
) invoke17_done (
    .in(invoke17_done_in),
    .out(invoke17_done_out)
);
std_wire # (
    .WIDTH(1)
) invoke18_go (
    .in(invoke18_go_in),
    .out(invoke18_go_out)
);
std_wire # (
    .WIDTH(1)
) invoke18_done (
    .in(invoke18_done_in),
    .out(invoke18_done_out)
);
std_wire # (
    .WIDTH(1)
) invoke19_go (
    .in(invoke19_go_in),
    .out(invoke19_go_out)
);
std_wire # (
    .WIDTH(1)
) invoke19_done (
    .in(invoke19_done_in),
    .out(invoke19_done_out)
);
std_wire # (
    .WIDTH(1)
) invoke20_go (
    .in(invoke20_go_in),
    .out(invoke20_go_out)
);
std_wire # (
    .WIDTH(1)
) invoke20_done (
    .in(invoke20_done_in),
    .out(invoke20_done_out)
);
std_wire # (
    .WIDTH(1)
) invoke21_go (
    .in(invoke21_go_in),
    .out(invoke21_go_out)
);
std_wire # (
    .WIDTH(1)
) invoke21_done (
    .in(invoke21_done_in),
    .out(invoke21_done_out)
);
std_wire # (
    .WIDTH(1)
) invoke22_go (
    .in(invoke22_go_in),
    .out(invoke22_go_out)
);
std_wire # (
    .WIDTH(1)
) invoke22_done (
    .in(invoke22_done_in),
    .out(invoke22_done_out)
);
std_wire # (
    .WIDTH(1)
) invoke23_go (
    .in(invoke23_go_in),
    .out(invoke23_go_out)
);
std_wire # (
    .WIDTH(1)
) invoke23_done (
    .in(invoke23_done_in),
    .out(invoke23_done_out)
);
std_wire # (
    .WIDTH(1)
) invoke24_go (
    .in(invoke24_go_in),
    .out(invoke24_go_out)
);
std_wire # (
    .WIDTH(1)
) invoke24_done (
    .in(invoke24_done_in),
    .out(invoke24_done_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_static_par_thread_go (
    .in(early_reset_static_par_thread_go_in),
    .out(early_reset_static_par_thread_go_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_static_par_thread_done (
    .in(early_reset_static_par_thread_done_in),
    .out(early_reset_static_par_thread_done_out)
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
) wrapper_early_reset_static_par_thread_go (
    .in(wrapper_early_reset_static_par_thread_go_in),
    .out(wrapper_early_reset_static_par_thread_go_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_static_par_thread_done (
    .in(wrapper_early_reset_static_par_thread_done_in),
    .out(wrapper_early_reset_static_par_thread_done_out)
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
std_wire # (
    .WIDTH(1)
) par1_go (
    .in(par1_go_in),
    .out(par1_go_out)
);
std_wire # (
    .WIDTH(1)
) par1_done (
    .in(par1_done_in),
    .out(par1_done_out)
);
std_wire # (
    .WIDTH(1)
) tdcc2_go (
    .in(tdcc2_go_in),
    .out(tdcc2_go_out)
);
std_wire # (
    .WIDTH(1)
) tdcc2_done (
    .in(tdcc2_done_in),
    .out(tdcc2_done_out)
);
std_wire # (
    .WIDTH(1)
) tdcc3_go (
    .in(tdcc3_go_in),
    .out(tdcc3_go_out)
);
std_wire # (
    .WIDTH(1)
) tdcc3_done (
    .in(tdcc3_done_in),
    .out(tdcc3_done_out)
);
std_wire # (
    .WIDTH(1)
) tdcc4_go (
    .in(tdcc4_go_in),
    .out(tdcc4_go_out)
);
std_wire # (
    .WIDTH(1)
) tdcc4_done (
    .in(tdcc4_done_in),
    .out(tdcc4_done_out)
);
std_wire # (
    .WIDTH(1)
) tdcc5_go (
    .in(tdcc5_go_in),
    .out(tdcc5_go_out)
);
std_wire # (
    .WIDTH(1)
) tdcc5_done (
    .in(tdcc5_done_in),
    .out(tdcc5_done_out)
);
wire _guard0 = 1;
wire _guard1 = early_reset_static_par_thread_go_out;
wire _guard2 = invoke7_go_out;
wire _guard3 = invoke17_go_out;
wire _guard4 = invoke7_go_out;
wire _guard5 = invoke17_go_out;
wire _guard6 = early_reset_static_par_thread_go_out;
wire _guard7 = invoke9_done_out;
wire _guard8 = ~_guard7;
wire _guard9 = fsm0_out == 2'd1;
wire _guard10 = _guard8 & _guard9;
wire _guard11 = tdcc0_go_out;
wire _guard12 = _guard10 & _guard11;
wire _guard13 = fsm2_out == 2'd3;
wire _guard14 = invoke11_go_out;
wire _guard15 = invoke11_go_out;
wire _guard16 = invoke11_go_out;
wire _guard17 = invoke11_go_out;
wire _guard18 = invoke11_go_out;
wire _guard19 = invoke11_go_out;
wire _guard20 = invoke11_go_out;
wire _guard21 = invoke11_go_out;
wire _guard22 = invoke11_go_out;
wire _guard23 = invoke11_go_out;
wire _guard24 = invoke11_go_out;
wire _guard25 = tdcc5_done_out;
wire _guard26 = invoke20_go_out;
wire _guard27 = invoke10_go_out;
wire _guard28 = invoke10_go_out;
wire _guard29 = invoke22_go_out;
wire _guard30 = invoke22_go_out;
wire _guard31 = invoke6_go_out;
wire _guard32 = invoke16_go_out;
wire _guard33 = invoke19_go_out;
wire _guard34 = invoke23_go_out;
wire _guard35 = invoke18_go_out;
wire _guard36 = invoke19_go_out;
wire _guard37 = invoke11_go_out;
wire _guard38 = invoke8_go_out;
wire _guard39 = invoke19_go_out;
wire _guard40 = invoke20_go_out;
wire _guard41 = invoke22_go_out;
wire _guard42 = invoke16_go_out;
wire _guard43 = invoke8_go_out;
wire _guard44 = invoke17_go_out;
wire _guard45 = invoke8_go_out;
wire _guard46 = invoke19_go_out;
wire _guard47 = invoke22_go_out;
wire _guard48 = invoke6_go_out;
wire _guard49 = invoke16_go_out;
wire _guard50 = invoke23_go_out;
wire _guard51 = invoke24_go_out;
wire _guard52 = invoke17_go_out;
wire _guard53 = invoke6_go_out;
wire _guard54 = invoke17_go_out;
wire _guard55 = invoke10_go_out;
wire _guard56 = invoke22_go_out;
wire _guard57 = invoke23_go_out;
wire _guard58 = invoke7_go_out;
wire _guard59 = invoke9_go_out;
wire _guard60 = invoke20_go_out;
wire _guard61 = invoke21_go_out;
wire _guard62 = invoke16_go_out;
wire _guard63 = invoke8_go_out;
wire _guard64 = invoke16_go_out;
wire _guard65 = invoke8_go_out;
wire _guard66 = invoke10_go_out;
wire _guard67 = invoke10_go_out;
wire _guard68 = invoke22_go_out;
wire _guard69 = invoke6_go_out;
wire _guard70 = invoke16_go_out;
wire _guard71 = invoke19_go_out;
wire _guard72 = invoke19_go_out;
wire _guard73 = invoke6_go_out;
wire _guard74 = fsm_out == 2'd2;
wire _guard75 = fsm_out == 2'd0;
wire _guard76 = invoke6_done_out;
wire _guard77 = _guard75 & _guard76;
wire _guard78 = tdcc_go_out;
wire _guard79 = _guard77 & _guard78;
wire _guard80 = _guard74 | _guard79;
wire _guard81 = fsm_out == 2'd1;
wire _guard82 = invoke7_done_out;
wire _guard83 = _guard81 & _guard82;
wire _guard84 = tdcc_go_out;
wire _guard85 = _guard83 & _guard84;
wire _guard86 = _guard80 | _guard85;
wire _guard87 = fsm_out == 2'd0;
wire _guard88 = invoke6_done_out;
wire _guard89 = _guard87 & _guard88;
wire _guard90 = tdcc_go_out;
wire _guard91 = _guard89 & _guard90;
wire _guard92 = fsm_out == 2'd2;
wire _guard93 = fsm_out == 2'd1;
wire _guard94 = invoke7_done_out;
wire _guard95 = _guard93 & _guard94;
wire _guard96 = tdcc_go_out;
wire _guard97 = _guard95 & _guard96;
wire _guard98 = invoke18_done_out;
wire _guard99 = ~_guard98;
wire _guard100 = fsm2_out == 2'd2;
wire _guard101 = _guard99 & _guard100;
wire _guard102 = tdcc2_go_out;
wire _guard103 = _guard101 & _guard102;
wire _guard104 = pd2_out;
wire _guard105 = tdcc2_done_out;
wire _guard106 = _guard104 | _guard105;
wire _guard107 = ~_guard106;
wire _guard108 = par1_go_out;
wire _guard109 = _guard107 & _guard108;
wire _guard110 = invoke9_go_out;
wire _guard111 = early_reset_static_par_thread_go_out;
wire _guard112 = invoke20_go_out;
wire _guard113 = invoke9_go_out;
wire _guard114 = invoke20_go_out;
wire _guard115 = early_reset_static_par_thread_go_out;
wire _guard116 = invoke9_go_out;
wire _guard117 = invoke9_go_out;
wire _guard118 = invoke9_go_out;
wire _guard119 = invoke9_go_out;
wire _guard120 = invoke9_go_out;
wire _guard121 = invoke9_go_out;
wire _guard122 = invoke9_go_out;
wire _guard123 = invoke9_go_out;
wire _guard124 = invoke9_go_out;
wire _guard125 = invoke9_go_out;
wire _guard126 = invoke9_go_out;
wire _guard127 = invoke9_go_out;
wire _guard128 = invoke12_go_out;
wire _guard129 = invoke20_go_out;
wire _guard130 = invoke9_go_out;
wire _guard131 = invoke12_go_out;
wire _guard132 = invoke20_go_out;
wire _guard133 = invoke9_go_out;
wire _guard134 = invoke12_go_out;
wire _guard135 = invoke20_go_out;
wire _guard136 = invoke9_go_out;
wire _guard137 = invoke24_go_out;
wire _guard138 = invoke24_go_out;
wire _guard139 = signal_reg0_out;
wire _guard140 = _guard0 & _guard0;
wire _guard141 = signal_reg0_out;
wire _guard142 = ~_guard141;
wire _guard143 = _guard140 & _guard142;
wire _guard144 = wrapper_early_reset_static_par_thread0_go_out;
wire _guard145 = _guard143 & _guard144;
wire _guard146 = _guard139 | _guard145;
wire _guard147 = _guard0 & _guard0;
wire _guard148 = signal_reg0_out;
wire _guard149 = ~_guard148;
wire _guard150 = _guard147 & _guard149;
wire _guard151 = wrapper_early_reset_static_par_thread0_go_out;
wire _guard152 = _guard150 & _guard151;
wire _guard153 = signal_reg0_out;
wire _guard154 = fsm3_out == 2'd3;
wire _guard155 = fsm3_out == 2'd0;
wire _guard156 = invoke19_done_out;
wire _guard157 = _guard155 & _guard156;
wire _guard158 = tdcc3_go_out;
wire _guard159 = _guard157 & _guard158;
wire _guard160 = _guard154 | _guard159;
wire _guard161 = fsm3_out == 2'd1;
wire _guard162 = invoke20_done_out;
wire _guard163 = _guard161 & _guard162;
wire _guard164 = tdcc3_go_out;
wire _guard165 = _guard163 & _guard164;
wire _guard166 = _guard160 | _guard165;
wire _guard167 = fsm3_out == 2'd2;
wire _guard168 = invoke21_done_out;
wire _guard169 = _guard167 & _guard168;
wire _guard170 = tdcc3_go_out;
wire _guard171 = _guard169 & _guard170;
wire _guard172 = _guard166 | _guard171;
wire _guard173 = fsm3_out == 2'd0;
wire _guard174 = invoke19_done_out;
wire _guard175 = _guard173 & _guard174;
wire _guard176 = tdcc3_go_out;
wire _guard177 = _guard175 & _guard176;
wire _guard178 = fsm3_out == 2'd3;
wire _guard179 = fsm3_out == 2'd2;
wire _guard180 = invoke21_done_out;
wire _guard181 = _guard179 & _guard180;
wire _guard182 = tdcc3_go_out;
wire _guard183 = _guard181 & _guard182;
wire _guard184 = fsm3_out == 2'd1;
wire _guard185 = invoke20_done_out;
wire _guard186 = _guard184 & _guard185;
wire _guard187 = tdcc3_go_out;
wire _guard188 = _guard186 & _guard187;
wire _guard189 = fsm5_out == 3'd5;
wire _guard190 = fsm5_out == 3'd0;
wire _guard191 = wrapper_early_reset_static_par_thread_done_out;
wire _guard192 = _guard190 & _guard191;
wire _guard193 = tdcc5_go_out;
wire _guard194 = _guard192 & _guard193;
wire _guard195 = _guard189 | _guard194;
wire _guard196 = fsm5_out == 3'd1;
wire _guard197 = par0_done_out;
wire _guard198 = _guard196 & _guard197;
wire _guard199 = tdcc5_go_out;
wire _guard200 = _guard198 & _guard199;
wire _guard201 = _guard195 | _guard200;
wire _guard202 = fsm5_out == 3'd2;
wire _guard203 = invoke12_done_out;
wire _guard204 = _guard202 & _guard203;
wire _guard205 = tdcc5_go_out;
wire _guard206 = _guard204 & _guard205;
wire _guard207 = _guard201 | _guard206;
wire _guard208 = fsm5_out == 3'd3;
wire _guard209 = wrapper_early_reset_static_par_thread0_done_out;
wire _guard210 = _guard208 & _guard209;
wire _guard211 = tdcc5_go_out;
wire _guard212 = _guard210 & _guard211;
wire _guard213 = _guard207 | _guard212;
wire _guard214 = fsm5_out == 3'd4;
wire _guard215 = par1_done_out;
wire _guard216 = _guard214 & _guard215;
wire _guard217 = tdcc5_go_out;
wire _guard218 = _guard216 & _guard217;
wire _guard219 = _guard213 | _guard218;
wire _guard220 = fsm5_out == 3'd4;
wire _guard221 = par1_done_out;
wire _guard222 = _guard220 & _guard221;
wire _guard223 = tdcc5_go_out;
wire _guard224 = _guard222 & _guard223;
wire _guard225 = fsm5_out == 3'd1;
wire _guard226 = par0_done_out;
wire _guard227 = _guard225 & _guard226;
wire _guard228 = tdcc5_go_out;
wire _guard229 = _guard227 & _guard228;
wire _guard230 = fsm5_out == 3'd3;
wire _guard231 = wrapper_early_reset_static_par_thread0_done_out;
wire _guard232 = _guard230 & _guard231;
wire _guard233 = tdcc5_go_out;
wire _guard234 = _guard232 & _guard233;
wire _guard235 = fsm5_out == 3'd0;
wire _guard236 = wrapper_early_reset_static_par_thread_done_out;
wire _guard237 = _guard235 & _guard236;
wire _guard238 = tdcc5_go_out;
wire _guard239 = _guard237 & _guard238;
wire _guard240 = fsm5_out == 3'd5;
wire _guard241 = fsm5_out == 3'd2;
wire _guard242 = invoke12_done_out;
wire _guard243 = _guard241 & _guard242;
wire _guard244 = tdcc5_go_out;
wire _guard245 = _guard243 & _guard244;
wire _guard246 = wrapper_early_reset_static_par_thread_done_out;
wire _guard247 = ~_guard246;
wire _guard248 = fsm5_out == 3'd0;
wire _guard249 = _guard247 & _guard248;
wire _guard250 = tdcc5_go_out;
wire _guard251 = _guard249 & _guard250;
wire _guard252 = fsm0_out == 2'd2;
wire _guard253 = invoke9_go_out;
wire _guard254 = early_reset_static_par_thread_go_out;
wire _guard255 = early_reset_static_par_thread0_go_out;
wire _guard256 = _guard254 | _guard255;
wire _guard257 = invoke20_go_out;
wire _guard258 = invoke9_go_out;
wire _guard259 = invoke20_go_out;
wire _guard260 = early_reset_static_par_thread_go_out;
wire _guard261 = early_reset_static_par_thread0_go_out;
wire _guard262 = _guard260 | _guard261;
wire _guard263 = invoke22_go_out;
wire _guard264 = invoke22_go_out;
wire _guard265 = invoke12_go_out;
wire _guard266 = invoke12_go_out;
wire _guard267 = invoke12_go_out;
wire _guard268 = invoke12_go_out;
wire _guard269 = invoke12_go_out;
wire _guard270 = invoke12_go_out;
wire _guard271 = fsm1_out == 2'd2;
wire _guard272 = fsm1_out == 2'd0;
wire _guard273 = invoke10_done_out;
wire _guard274 = _guard272 & _guard273;
wire _guard275 = tdcc1_go_out;
wire _guard276 = _guard274 & _guard275;
wire _guard277 = _guard271 | _guard276;
wire _guard278 = fsm1_out == 2'd1;
wire _guard279 = invoke11_done_out;
wire _guard280 = _guard278 & _guard279;
wire _guard281 = tdcc1_go_out;
wire _guard282 = _guard280 & _guard281;
wire _guard283 = _guard277 | _guard282;
wire _guard284 = fsm1_out == 2'd0;
wire _guard285 = invoke10_done_out;
wire _guard286 = _guard284 & _guard285;
wire _guard287 = tdcc1_go_out;
wire _guard288 = _guard286 & _guard287;
wire _guard289 = fsm1_out == 2'd2;
wire _guard290 = fsm1_out == 2'd1;
wire _guard291 = invoke11_done_out;
wire _guard292 = _guard290 & _guard291;
wire _guard293 = tdcc1_go_out;
wire _guard294 = _guard292 & _guard293;
wire _guard295 = fsm4_out == 2'd3;
wire _guard296 = fsm4_out == 2'd0;
wire _guard297 = invoke22_done_out;
wire _guard298 = _guard296 & _guard297;
wire _guard299 = tdcc4_go_out;
wire _guard300 = _guard298 & _guard299;
wire _guard301 = _guard295 | _guard300;
wire _guard302 = fsm4_out == 2'd1;
wire _guard303 = invoke23_done_out;
wire _guard304 = _guard302 & _guard303;
wire _guard305 = tdcc4_go_out;
wire _guard306 = _guard304 & _guard305;
wire _guard307 = _guard301 | _guard306;
wire _guard308 = fsm4_out == 2'd2;
wire _guard309 = invoke24_done_out;
wire _guard310 = _guard308 & _guard309;
wire _guard311 = tdcc4_go_out;
wire _guard312 = _guard310 & _guard311;
wire _guard313 = _guard307 | _guard312;
wire _guard314 = fsm4_out == 2'd0;
wire _guard315 = invoke22_done_out;
wire _guard316 = _guard314 & _guard315;
wire _guard317 = tdcc4_go_out;
wire _guard318 = _guard316 & _guard317;
wire _guard319 = fsm4_out == 2'd3;
wire _guard320 = fsm4_out == 2'd2;
wire _guard321 = invoke24_done_out;
wire _guard322 = _guard320 & _guard321;
wire _guard323 = tdcc4_go_out;
wire _guard324 = _guard322 & _guard323;
wire _guard325 = fsm4_out == 2'd1;
wire _guard326 = invoke23_done_out;
wire _guard327 = _guard325 & _guard326;
wire _guard328 = tdcc4_go_out;
wire _guard329 = _guard327 & _guard328;
wire _guard330 = invoke11_done_out;
wire _guard331 = ~_guard330;
wire _guard332 = fsm1_out == 2'd1;
wire _guard333 = _guard331 & _guard332;
wire _guard334 = tdcc1_go_out;
wire _guard335 = _guard333 & _guard334;
wire _guard336 = invoke23_done_out;
wire _guard337 = ~_guard336;
wire _guard338 = fsm4_out == 2'd1;
wire _guard339 = _guard337 & _guard338;
wire _guard340 = tdcc4_go_out;
wire _guard341 = _guard339 & _guard340;
wire _guard342 = par1_done_out;
wire _guard343 = ~_guard342;
wire _guard344 = fsm5_out == 3'd4;
wire _guard345 = _guard343 & _guard344;
wire _guard346 = tdcc5_go_out;
wire _guard347 = _guard345 & _guard346;
wire _guard348 = early_reset_static_par_thread_go_out;
wire _guard349 = early_reset_static_par_thread0_go_out;
wire _guard350 = _guard348 | _guard349;
wire _guard351 = invoke7_go_out;
wire _guard352 = invoke17_go_out;
wire _guard353 = invoke7_go_out;
wire _guard354 = early_reset_static_par_thread_go_out;
wire _guard355 = early_reset_static_par_thread0_go_out;
wire _guard356 = _guard354 | _guard355;
wire _guard357 = invoke17_go_out;
wire _guard358 = invoke7_go_out;
wire _guard359 = invoke7_go_out;
wire _guard360 = invoke7_go_out;
wire _guard361 = invoke7_go_out;
wire _guard362 = invoke7_go_out;
wire _guard363 = invoke7_go_out;
wire _guard364 = invoke7_go_out;
wire _guard365 = invoke7_go_out;
wire _guard366 = invoke7_go_out;
wire _guard367 = invoke7_go_out;
wire _guard368 = invoke7_go_out;
wire _guard369 = invoke12_go_out;
wire _guard370 = invoke7_go_out;
wire _guard371 = invoke17_go_out;
wire _guard372 = invoke12_go_out;
wire _guard373 = invoke7_go_out;
wire _guard374 = invoke17_go_out;
wire _guard375 = invoke12_go_out;
wire _guard376 = invoke7_go_out;
wire _guard377 = invoke17_go_out;
wire _guard378 = invoke7_go_out;
wire _guard379 = invoke20_go_out;
wire _guard380 = invoke20_go_out;
wire _guard381 = invoke20_go_out;
wire _guard382 = invoke20_go_out;
wire _guard383 = invoke20_go_out;
wire _guard384 = invoke20_go_out;
wire _guard385 = invoke20_go_out;
wire _guard386 = invoke20_go_out;
wire _guard387 = invoke20_go_out;
wire _guard388 = invoke11_go_out;
wire _guard389 = early_reset_static_par_thread_go_out;
wire _guard390 = early_reset_static_par_thread0_go_out;
wire _guard391 = _guard389 | _guard390;
wire _guard392 = invoke23_go_out;
wire _guard393 = invoke11_go_out;
wire _guard394 = early_reset_static_par_thread_go_out;
wire _guard395 = early_reset_static_par_thread0_go_out;
wire _guard396 = _guard394 | _guard395;
wire _guard397 = invoke23_go_out;
wire _guard398 = invoke10_go_out;
wire _guard399 = invoke10_go_out;
wire _guard400 = invoke10_go_out;
wire _guard401 = invoke10_go_out;
wire _guard402 = pd_out;
wire _guard403 = pd0_out;
wire _guard404 = _guard402 & _guard403;
wire _guard405 = pd1_out;
wire _guard406 = _guard404 & _guard405;
wire _guard407 = tdcc1_done_out;
wire _guard408 = par0_go_out;
wire _guard409 = _guard407 & _guard408;
wire _guard410 = _guard406 | _guard409;
wire _guard411 = tdcc1_done_out;
wire _guard412 = par0_go_out;
wire _guard413 = _guard411 & _guard412;
wire _guard414 = pd_out;
wire _guard415 = pd0_out;
wire _guard416 = _guard414 & _guard415;
wire _guard417 = pd1_out;
wire _guard418 = _guard416 & _guard417;
wire _guard419 = pd_out;
wire _guard420 = tdcc_done_out;
wire _guard421 = _guard419 | _guard420;
wire _guard422 = ~_guard421;
wire _guard423 = par0_go_out;
wire _guard424 = _guard422 & _guard423;
wire _guard425 = invoke12_done_out;
wire _guard426 = ~_guard425;
wire _guard427 = fsm5_out == 3'd2;
wire _guard428 = _guard426 & _guard427;
wire _guard429 = tdcc5_go_out;
wire _guard430 = _guard428 & _guard429;
wire _guard431 = pd3_out;
wire _guard432 = tdcc3_done_out;
wire _guard433 = _guard431 | _guard432;
wire _guard434 = ~_guard433;
wire _guard435 = par1_go_out;
wire _guard436 = _guard434 & _guard435;
wire _guard437 = fsm3_out == 2'd3;
wire _guard438 = invoke19_go_out;
wire _guard439 = invoke19_go_out;
wire _guard440 = invoke19_go_out;
wire _guard441 = invoke19_go_out;
wire _guard442 = fsm0_out == 2'd2;
wire _guard443 = fsm0_out == 2'd0;
wire _guard444 = invoke8_done_out;
wire _guard445 = _guard443 & _guard444;
wire _guard446 = tdcc0_go_out;
wire _guard447 = _guard445 & _guard446;
wire _guard448 = _guard442 | _guard447;
wire _guard449 = fsm0_out == 2'd1;
wire _guard450 = invoke9_done_out;
wire _guard451 = _guard449 & _guard450;
wire _guard452 = tdcc0_go_out;
wire _guard453 = _guard451 & _guard452;
wire _guard454 = _guard448 | _guard453;
wire _guard455 = fsm0_out == 2'd0;
wire _guard456 = invoke8_done_out;
wire _guard457 = _guard455 & _guard456;
wire _guard458 = tdcc0_go_out;
wire _guard459 = _guard457 & _guard458;
wire _guard460 = fsm0_out == 2'd2;
wire _guard461 = fsm0_out == 2'd1;
wire _guard462 = invoke9_done_out;
wire _guard463 = _guard461 & _guard462;
wire _guard464 = tdcc0_go_out;
wire _guard465 = _guard463 & _guard464;
wire _guard466 = fsm2_out == 2'd3;
wire _guard467 = fsm2_out == 2'd0;
wire _guard468 = invoke16_done_out;
wire _guard469 = _guard467 & _guard468;
wire _guard470 = tdcc2_go_out;
wire _guard471 = _guard469 & _guard470;
wire _guard472 = _guard466 | _guard471;
wire _guard473 = fsm2_out == 2'd1;
wire _guard474 = invoke17_done_out;
wire _guard475 = _guard473 & _guard474;
wire _guard476 = tdcc2_go_out;
wire _guard477 = _guard475 & _guard476;
wire _guard478 = _guard472 | _guard477;
wire _guard479 = fsm2_out == 2'd2;
wire _guard480 = invoke18_done_out;
wire _guard481 = _guard479 & _guard480;
wire _guard482 = tdcc2_go_out;
wire _guard483 = _guard481 & _guard482;
wire _guard484 = _guard478 | _guard483;
wire _guard485 = fsm2_out == 2'd0;
wire _guard486 = invoke16_done_out;
wire _guard487 = _guard485 & _guard486;
wire _guard488 = tdcc2_go_out;
wire _guard489 = _guard487 & _guard488;
wire _guard490 = fsm2_out == 2'd3;
wire _guard491 = fsm2_out == 2'd2;
wire _guard492 = invoke18_done_out;
wire _guard493 = _guard491 & _guard492;
wire _guard494 = tdcc2_go_out;
wire _guard495 = _guard493 & _guard494;
wire _guard496 = fsm2_out == 2'd1;
wire _guard497 = invoke17_done_out;
wire _guard498 = _guard496 & _guard497;
wire _guard499 = tdcc2_go_out;
wire _guard500 = _guard498 & _guard499;
wire _guard501 = invoke8_done_out;
wire _guard502 = ~_guard501;
wire _guard503 = fsm0_out == 2'd0;
wire _guard504 = _guard502 & _guard503;
wire _guard505 = tdcc0_go_out;
wire _guard506 = _guard504 & _guard505;
wire _guard507 = pd0_out;
wire _guard508 = tdcc0_done_out;
wire _guard509 = _guard507 | _guard508;
wire _guard510 = ~_guard509;
wire _guard511 = par0_go_out;
wire _guard512 = _guard510 & _guard511;
wire _guard513 = invoke6_go_out;
wire _guard514 = invoke6_go_out;
wire _guard515 = invoke6_go_out;
wire _guard516 = invoke6_go_out;
wire _guard517 = invoke16_go_out;
wire _guard518 = invoke16_go_out;
wire _guard519 = invoke16_go_out;
wire _guard520 = invoke16_go_out;
wire _guard521 = pd_out;
wire _guard522 = pd0_out;
wire _guard523 = _guard521 & _guard522;
wire _guard524 = pd1_out;
wire _guard525 = _guard523 & _guard524;
wire _guard526 = invoke17_done_out;
wire _guard527 = ~_guard526;
wire _guard528 = fsm2_out == 2'd1;
wire _guard529 = _guard527 & _guard528;
wire _guard530 = tdcc2_go_out;
wire _guard531 = _guard529 & _guard530;
wire _guard532 = invoke21_done_out;
wire _guard533 = ~_guard532;
wire _guard534 = fsm3_out == 2'd2;
wire _guard535 = _guard533 & _guard534;
wire _guard536 = tdcc3_go_out;
wire _guard537 = _guard535 & _guard536;
wire _guard538 = pd2_out;
wire _guard539 = pd3_out;
wire _guard540 = _guard538 & _guard539;
wire _guard541 = pd4_out;
wire _guard542 = _guard540 & _guard541;
wire _guard543 = tdcc2_done_out;
wire _guard544 = par1_go_out;
wire _guard545 = _guard543 & _guard544;
wire _guard546 = _guard542 | _guard545;
wire _guard547 = tdcc2_done_out;
wire _guard548 = par1_go_out;
wire _guard549 = _guard547 & _guard548;
wire _guard550 = pd2_out;
wire _guard551 = pd3_out;
wire _guard552 = _guard550 & _guard551;
wire _guard553 = pd4_out;
wire _guard554 = _guard552 & _guard553;
wire _guard555 = wrapper_early_reset_static_par_thread0_done_out;
wire _guard556 = ~_guard555;
wire _guard557 = fsm5_out == 3'd3;
wire _guard558 = _guard556 & _guard557;
wire _guard559 = tdcc5_go_out;
wire _guard560 = _guard558 & _guard559;
wire _guard561 = invoke16_done_out;
wire _guard562 = ~_guard561;
wire _guard563 = fsm2_out == 2'd0;
wire _guard564 = _guard562 & _guard563;
wire _guard565 = tdcc2_go_out;
wire _guard566 = _guard564 & _guard565;
wire _guard567 = invoke17_go_out;
wire _guard568 = invoke17_go_out;
wire _guard569 = invoke17_go_out;
wire _guard570 = invoke17_go_out;
wire _guard571 = invoke17_go_out;
wire _guard572 = invoke17_go_out;
wire _guard573 = invoke17_go_out;
wire _guard574 = invoke17_go_out;
wire _guard575 = invoke17_go_out;
wire _guard576 = invoke8_go_out;
wire _guard577 = invoke8_go_out;
wire _guard578 = invoke8_go_out;
wire _guard579 = invoke8_go_out;
wire _guard580 = signal_reg_out;
wire _guard581 = _guard0 & _guard0;
wire _guard582 = signal_reg_out;
wire _guard583 = ~_guard582;
wire _guard584 = _guard581 & _guard583;
wire _guard585 = wrapper_early_reset_static_par_thread_go_out;
wire _guard586 = _guard584 & _guard585;
wire _guard587 = _guard580 | _guard586;
wire _guard588 = _guard0 & _guard0;
wire _guard589 = signal_reg_out;
wire _guard590 = ~_guard589;
wire _guard591 = _guard588 & _guard590;
wire _guard592 = wrapper_early_reset_static_par_thread_go_out;
wire _guard593 = _guard591 & _guard592;
wire _guard594 = signal_reg_out;
wire _guard595 = wrapper_early_reset_static_par_thread_go_out;
wire _guard596 = signal_reg0_out;
wire _guard597 = fsm1_out == 2'd2;
wire _guard598 = pd2_out;
wire _guard599 = pd3_out;
wire _guard600 = _guard598 & _guard599;
wire _guard601 = pd4_out;
wire _guard602 = _guard600 & _guard601;
wire _guard603 = invoke21_go_out;
wire _guard604 = invoke21_go_out;
wire _guard605 = invoke11_go_out;
wire _guard606 = invoke12_go_out;
wire _guard607 = invoke23_go_out;
wire _guard608 = invoke11_go_out;
wire _guard609 = invoke12_go_out;
wire _guard610 = invoke23_go_out;
wire _guard611 = invoke11_go_out;
wire _guard612 = invoke12_go_out;
wire _guard613 = invoke23_go_out;
wire _guard614 = invoke11_go_out;
wire _guard615 = invoke12_go_out;
wire _guard616 = pd_out;
wire _guard617 = pd0_out;
wire _guard618 = _guard616 & _guard617;
wire _guard619 = pd1_out;
wire _guard620 = _guard618 & _guard619;
wire _guard621 = tdcc_done_out;
wire _guard622 = par0_go_out;
wire _guard623 = _guard621 & _guard622;
wire _guard624 = _guard620 | _guard623;
wire _guard625 = tdcc_done_out;
wire _guard626 = par0_go_out;
wire _guard627 = _guard625 & _guard626;
wire _guard628 = pd_out;
wire _guard629 = pd0_out;
wire _guard630 = _guard628 & _guard629;
wire _guard631 = pd1_out;
wire _guard632 = _guard630 & _guard631;
wire _guard633 = pd_out;
wire _guard634 = pd0_out;
wire _guard635 = _guard633 & _guard634;
wire _guard636 = pd1_out;
wire _guard637 = _guard635 & _guard636;
wire _guard638 = tdcc0_done_out;
wire _guard639 = par0_go_out;
wire _guard640 = _guard638 & _guard639;
wire _guard641 = _guard637 | _guard640;
wire _guard642 = tdcc0_done_out;
wire _guard643 = par0_go_out;
wire _guard644 = _guard642 & _guard643;
wire _guard645 = pd_out;
wire _guard646 = pd0_out;
wire _guard647 = _guard645 & _guard646;
wire _guard648 = pd1_out;
wire _guard649 = _guard647 & _guard648;
wire _guard650 = pd2_out;
wire _guard651 = pd3_out;
wire _guard652 = _guard650 & _guard651;
wire _guard653 = pd4_out;
wire _guard654 = _guard652 & _guard653;
wire _guard655 = tdcc4_done_out;
wire _guard656 = par1_go_out;
wire _guard657 = _guard655 & _guard656;
wire _guard658 = _guard654 | _guard657;
wire _guard659 = tdcc4_done_out;
wire _guard660 = par1_go_out;
wire _guard661 = _guard659 & _guard660;
wire _guard662 = pd2_out;
wire _guard663 = pd3_out;
wire _guard664 = _guard662 & _guard663;
wire _guard665 = pd4_out;
wire _guard666 = _guard664 & _guard665;
wire _guard667 = wrapper_early_reset_static_par_thread0_go_out;
wire _guard668 = signal_reg_out;
wire _guard669 = pd4_out;
wire _guard670 = tdcc4_done_out;
wire _guard671 = _guard669 | _guard670;
wire _guard672 = ~_guard671;
wire _guard673 = par1_go_out;
wire _guard674 = _guard672 & _guard673;
wire _guard675 = invoke18_go_out;
wire _guard676 = invoke18_go_out;
wire _guard677 = fsm_out == 2'd2;
wire _guard678 = invoke19_done_out;
wire _guard679 = ~_guard678;
wire _guard680 = fsm3_out == 2'd0;
wire _guard681 = _guard679 & _guard680;
wire _guard682 = tdcc3_go_out;
wire _guard683 = _guard681 & _guard682;
wire _guard684 = invoke20_done_out;
wire _guard685 = ~_guard684;
wire _guard686 = fsm3_out == 2'd1;
wire _guard687 = _guard685 & _guard686;
wire _guard688 = tdcc3_go_out;
wire _guard689 = _guard687 & _guard688;
wire _guard690 = invoke16_go_out;
wire _guard691 = invoke16_go_out;
wire _guard692 = invoke22_go_out;
wire _guard693 = invoke22_go_out;
wire _guard694 = invoke22_go_out;
wire _guard695 = invoke22_go_out;
wire _guard696 = invoke23_go_out;
wire _guard697 = invoke23_go_out;
wire _guard698 = invoke23_go_out;
wire _guard699 = invoke23_go_out;
wire _guard700 = invoke23_go_out;
wire _guard701 = invoke23_go_out;
wire _guard702 = invoke23_go_out;
wire _guard703 = invoke23_go_out;
wire _guard704 = invoke23_go_out;
wire _guard705 = pd2_out;
wire _guard706 = pd3_out;
wire _guard707 = _guard705 & _guard706;
wire _guard708 = pd4_out;
wire _guard709 = _guard707 & _guard708;
wire _guard710 = tdcc3_done_out;
wire _guard711 = par1_go_out;
wire _guard712 = _guard710 & _guard711;
wire _guard713 = _guard709 | _guard712;
wire _guard714 = tdcc3_done_out;
wire _guard715 = par1_go_out;
wire _guard716 = _guard714 & _guard715;
wire _guard717 = pd2_out;
wire _guard718 = pd3_out;
wire _guard719 = _guard717 & _guard718;
wire _guard720 = pd4_out;
wire _guard721 = _guard719 & _guard720;
wire _guard722 = fsm4_out == 2'd3;
wire _guard723 = invoke19_go_out;
wire _guard724 = invoke19_go_out;
wire _guard725 = invoke11_go_out;
wire _guard726 = early_reset_static_par_thread_go_out;
wire _guard727 = invoke23_go_out;
wire _guard728 = invoke11_go_out;
wire _guard729 = invoke23_go_out;
wire _guard730 = early_reset_static_par_thread_go_out;
wire _guard731 = invoke6_done_out;
wire _guard732 = ~_guard731;
wire _guard733 = fsm_out == 2'd0;
wire _guard734 = _guard732 & _guard733;
wire _guard735 = tdcc_go_out;
wire _guard736 = _guard734 & _guard735;
wire _guard737 = invoke24_done_out;
wire _guard738 = ~_guard737;
wire _guard739 = fsm4_out == 2'd2;
wire _guard740 = _guard738 & _guard739;
wire _guard741 = tdcc4_go_out;
wire _guard742 = _guard740 & _guard741;
wire _guard743 = pd1_out;
wire _guard744 = tdcc1_done_out;
wire _guard745 = _guard743 | _guard744;
wire _guard746 = ~_guard745;
wire _guard747 = par0_go_out;
wire _guard748 = _guard746 & _guard747;
wire _guard749 = par0_done_out;
wire _guard750 = ~_guard749;
wire _guard751 = fsm5_out == 3'd1;
wire _guard752 = _guard750 & _guard751;
wire _guard753 = tdcc5_go_out;
wire _guard754 = _guard752 & _guard753;
wire _guard755 = invoke7_done_out;
wire _guard756 = ~_guard755;
wire _guard757 = fsm_out == 2'd1;
wire _guard758 = _guard756 & _guard757;
wire _guard759 = tdcc_go_out;
wire _guard760 = _guard758 & _guard759;
wire _guard761 = invoke10_done_out;
wire _guard762 = ~_guard761;
wire _guard763 = fsm1_out == 2'd0;
wire _guard764 = _guard762 & _guard763;
wire _guard765 = tdcc1_go_out;
wire _guard766 = _guard764 & _guard765;
wire _guard767 = invoke22_done_out;
wire _guard768 = ~_guard767;
wire _guard769 = fsm4_out == 2'd0;
wire _guard770 = _guard768 & _guard769;
wire _guard771 = tdcc4_go_out;
wire _guard772 = _guard770 & _guard771;
wire _guard773 = fsm5_out == 3'd5;
assign curr_addr_internal_mem_A0_write_en =
  _guard1 ? 1'd1 :
  _guard2 ? read_channel_A0_curr_addr_internal_mem_write_en :
  _guard3 ? write_channel_A0_curr_addr_internal_mem_write_en :
  1'd0;
assign curr_addr_internal_mem_A0_clk = ap_clk;
assign curr_addr_internal_mem_A0_reset = reset;
assign curr_addr_internal_mem_A0_in =
  _guard4 ? read_channel_A0_curr_addr_internal_mem_in :
  _guard5 ? write_channel_A0_curr_addr_internal_mem_in :
  _guard6 ? 3'd0 :
  'x;
assign invoke7_done_in = read_channel_A0_done;
assign invoke9_go_in = _guard12;
assign invoke9_done_in = read_channel_B0_done;
assign tdcc2_done_in = _guard13;
assign read_channel_Sum0_curr_addr_internal_mem_out =
  _guard14 ? curr_addr_internal_mem_Sum0_out :
  3'd0;
assign read_channel_Sum0_curr_addr_axi_out =
  _guard15 ? curr_addr_axi_Sum0_out :
  64'd0;
assign read_channel_Sum0_RVALID =
  _guard16 ? Sum0_RVALID :
  1'd0;
assign read_channel_Sum0_RLAST =
  _guard17 ? Sum0_RLAST :
  1'd0;
assign read_channel_Sum0_RDATA =
  _guard18 ? Sum0_RDATA :
  32'd0;
assign read_channel_Sum0_clk = ap_clk;
assign read_channel_Sum0_go = _guard19;
assign read_channel_Sum0_reset = reset;
assign read_channel_Sum0_RRESP =
  _guard20 ? Sum0_RRESP :
  2'd0;
assign read_channel_Sum0_mem_ref_done =
  _guard21 ? internal_mem_Sum0_done :
  1'd0;
assign read_channel_Sum0_ARESETn =
  _guard22 ? Sum0_ARESETn :
  1'd0;
assign read_channel_Sum0_curr_addr_internal_mem_done =
  _guard23 ? curr_addr_internal_mem_Sum0_done :
  1'd0;
assign read_channel_Sum0_curr_addr_axi_done =
  _guard24 ? curr_addr_axi_Sum0_done :
  1'd0;
assign done = _guard25;
assign B0_WLAST =
  _guard26 ? write_channel_B0_WLAST :
  1'd0;
assign Sum0_ARVALID =
  _guard27 ? ar_channel_Sum0_ARVALID :
  1'd0;
assign Sum0_ARBURST =
  _guard28 ? ar_channel_Sum0_ARBURST :
  2'd0;
assign Sum0_AWADDR =
  _guard29 ? aw_channel_Sum0_AWADDR :
  64'd0;
assign Sum0_AWSIZE =
  _guard30 ? aw_channel_Sum0_AWSIZE :
  3'd0;
assign Sum0_ARID = 1'd0;
assign A0_ARSIZE =
  _guard31 ? ar_channel_A0_ARSIZE :
  3'd0;
assign A0_AWBURST =
  _guard32 ? aw_channel_A0_AWBURST :
  2'd0;
assign B0_AWBURST =
  _guard33 ? aw_channel_B0_AWBURST :
  2'd0;
assign Sum0_WDATA =
  _guard34 ? write_channel_Sum0_WDATA :
  32'd0;
assign A0_BREADY =
  _guard35 ? bresp_channel_A0_BREADY :
  1'd0;
assign B0_AWLEN =
  _guard36 ? aw_channel_B0_AWLEN :
  8'd0;
assign Sum0_RREADY =
  _guard37 ? read_channel_Sum0_RREADY :
  1'd0;
assign B0_ARID = 1'd0;
assign B0_ARBURST =
  _guard38 ? ar_channel_B0_ARBURST :
  2'd0;
assign B0_AWVALID =
  _guard39 ? aw_channel_B0_AWVALID :
  1'd0;
assign B0_WVALID =
  _guard40 ? write_channel_B0_WVALID :
  1'd0;
assign Sum0_AWLEN =
  _guard41 ? aw_channel_Sum0_AWLEN :
  8'd0;
assign Sum0_BID = 1'd0;
assign A0_AWSIZE =
  _guard42 ? aw_channel_A0_AWSIZE :
  3'd0;
assign B0_ARLEN =
  _guard43 ? ar_channel_B0_ARLEN :
  8'd0;
assign B0_WID = 1'd0;
assign B0_BID = 1'd0;
assign A0_WLAST =
  _guard44 ? write_channel_A0_WLAST :
  1'd0;
assign B0_ARVALID =
  _guard45 ? ar_channel_B0_ARVALID :
  1'd0;
assign B0_AWPROT =
  _guard46 ? aw_channel_B0_AWPROT :
  3'd0;
assign Sum0_AWPROT =
  _guard47 ? aw_channel_Sum0_AWPROT :
  3'd0;
assign A0_ARBURST =
  _guard48 ? ar_channel_A0_ARBURST :
  2'd0;
assign A0_AWPROT =
  _guard49 ? aw_channel_A0_AWPROT :
  3'd0;
assign Sum0_WLAST =
  _guard50 ? write_channel_Sum0_WLAST :
  1'd0;
assign Sum0_BREADY =
  _guard51 ? bresp_channel_Sum0_BREADY :
  1'd0;
assign Sum0_AWID = 1'd0;
assign A0_WVALID =
  _guard52 ? write_channel_A0_WVALID :
  1'd0;
assign A0_WID = 1'd0;
assign B0_AWID = 1'd0;
assign A0_ARADDR =
  _guard53 ? ar_channel_A0_ARADDR :
  64'd0;
assign A0_WDATA =
  _guard54 ? write_channel_A0_WDATA :
  32'd0;
assign Sum0_ARADDR =
  _guard55 ? ar_channel_Sum0_ARADDR :
  64'd0;
assign Sum0_AWBURST =
  _guard56 ? aw_channel_Sum0_AWBURST :
  2'd0;
assign Sum0_WVALID =
  _guard57 ? write_channel_Sum0_WVALID :
  1'd0;
assign A0_RREADY =
  _guard58 ? read_channel_A0_RREADY :
  1'd0;
assign A0_BID = 1'd0;
assign B0_RREADY =
  _guard59 ? read_channel_B0_RREADY :
  1'd0;
assign B0_WDATA =
  _guard60 ? write_channel_B0_WDATA :
  32'd0;
assign B0_BREADY =
  _guard61 ? bresp_channel_B0_BREADY :
  1'd0;
assign A0_AWLEN =
  _guard62 ? aw_channel_A0_AWLEN :
  8'd0;
assign B0_ARSIZE =
  _guard63 ? ar_channel_B0_ARSIZE :
  3'd0;
assign A0_AWADDR =
  _guard64 ? aw_channel_A0_AWADDR :
  64'd0;
assign B0_ARADDR =
  _guard65 ? ar_channel_B0_ARADDR :
  64'd0;
assign Sum0_ARSIZE =
  _guard66 ? ar_channel_Sum0_ARSIZE :
  3'd0;
assign Sum0_ARLEN =
  _guard67 ? ar_channel_Sum0_ARLEN :
  8'd0;
assign Sum0_AWVALID =
  _guard68 ? aw_channel_Sum0_AWVALID :
  1'd0;
assign A0_ARVALID =
  _guard69 ? ar_channel_A0_ARVALID :
  1'd0;
assign A0_AWVALID =
  _guard70 ? aw_channel_A0_AWVALID :
  1'd0;
assign A0_ARID = 1'd0;
assign B0_AWADDR =
  _guard71 ? aw_channel_B0_AWADDR :
  64'd0;
assign B0_AWSIZE =
  _guard72 ? aw_channel_B0_AWSIZE :
  3'd0;
assign Sum0_WID = 1'd0;
assign A0_ARLEN =
  _guard73 ? ar_channel_A0_ARLEN :
  8'd0;
assign A0_AWID = 1'd0;
assign fsm_write_en = _guard86;
assign fsm_clk = ap_clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard91 ? 2'd1 :
  _guard92 ? 2'd0 :
  _guard97 ? 2'd2 :
  2'd0;
assign invoke11_done_in = read_channel_Sum0_done;
assign invoke18_go_in = _guard103;
assign tdcc2_go_in = _guard109;
assign curr_addr_internal_mem_B0_write_en =
  _guard110 ? read_channel_B0_curr_addr_internal_mem_write_en :
  _guard111 ? 1'd1 :
  _guard112 ? write_channel_B0_curr_addr_internal_mem_write_en :
  1'd0;
assign curr_addr_internal_mem_B0_clk = ap_clk;
assign curr_addr_internal_mem_B0_reset = reset;
assign curr_addr_internal_mem_B0_in =
  _guard113 ? read_channel_B0_curr_addr_internal_mem_in :
  _guard114 ? write_channel_B0_curr_addr_internal_mem_in :
  _guard115 ? 3'd0 :
  'x;
assign read_channel_B0_curr_addr_internal_mem_out =
  _guard116 ? curr_addr_internal_mem_B0_out :
  3'd0;
assign read_channel_B0_curr_addr_axi_out =
  _guard117 ? curr_addr_axi_B0_out :
  64'd0;
assign read_channel_B0_RVALID =
  _guard118 ? B0_RVALID :
  1'd0;
assign read_channel_B0_RLAST =
  _guard119 ? B0_RLAST :
  1'd0;
assign read_channel_B0_RDATA =
  _guard120 ? B0_RDATA :
  32'd0;
assign read_channel_B0_clk = ap_clk;
assign read_channel_B0_go = _guard121;
assign read_channel_B0_reset = reset;
assign read_channel_B0_RRESP =
  _guard122 ? B0_RRESP :
  2'd0;
assign read_channel_B0_mem_ref_done =
  _guard123 ? internal_mem_B0_done :
  1'd0;
assign read_channel_B0_ARESETn =
  _guard124 ? B0_ARESETn :
  1'd0;
assign read_channel_B0_curr_addr_internal_mem_done =
  _guard125 ? curr_addr_internal_mem_B0_done :
  1'd0;
assign read_channel_B0_curr_addr_axi_done =
  _guard126 ? curr_addr_axi_B0_done :
  1'd0;
assign internal_mem_B0_write_en =
  _guard127 ? read_channel_B0_mem_ref_write_en :
  _guard128 ? main_compute_B0_write_en :
  _guard129 ? write_channel_B0_mem_ref_write_en :
  1'd0;
assign internal_mem_B0_clk = ap_clk;
assign internal_mem_B0_addr0 =
  _guard130 ? read_channel_B0_mem_ref_addr0 :
  _guard131 ? main_compute_B0_addr0 :
  _guard132 ? write_channel_B0_mem_ref_addr0 :
  'x;
assign internal_mem_B0_content_en =
  _guard133 ? read_channel_B0_mem_ref_content_en :
  _guard134 ? main_compute_B0_content_en :
  _guard135 ? write_channel_B0_mem_ref_content_en :
  1'd0;
assign internal_mem_B0_reset = reset;
assign internal_mem_B0_write_data = read_channel_B0_mem_ref_write_data;
assign bresp_channel_Sum0_clk = ap_clk;
assign bresp_channel_Sum0_go = _guard137;
assign bresp_channel_Sum0_reset = reset;
assign bresp_channel_Sum0_BVALID =
  _guard138 ? Sum0_BVALID :
  1'd0;
assign signal_reg0_write_en = _guard146;
assign signal_reg0_clk = ap_clk;
assign signal_reg0_reset = reset;
assign signal_reg0_in =
  _guard152 ? 1'd1 :
  _guard153 ? 1'd0 :
  1'd0;
assign fsm3_write_en = _guard172;
assign fsm3_clk = ap_clk;
assign fsm3_reset = reset;
assign fsm3_in =
  _guard177 ? 2'd1 :
  _guard178 ? 2'd0 :
  _guard183 ? 2'd3 :
  _guard188 ? 2'd2 :
  2'd0;
assign fsm5_write_en = _guard219;
assign fsm5_clk = ap_clk;
assign fsm5_reset = reset;
assign fsm5_in =
  _guard224 ? 3'd5 :
  _guard229 ? 3'd2 :
  _guard234 ? 3'd4 :
  _guard239 ? 3'd1 :
  _guard240 ? 3'd0 :
  _guard245 ? 3'd3 :
  3'd0;
assign wrapper_early_reset_static_par_thread_go_in = _guard251;
assign tdcc0_done_in = _guard252;
assign early_reset_static_par_thread0_done_in = ud0_out;
assign curr_addr_axi_B0_write_en =
  _guard253 ? read_channel_B0_curr_addr_axi_write_en :
  _guard256 ? 1'd1 :
  _guard257 ? write_channel_B0_curr_addr_axi_write_en :
  1'd0;
assign curr_addr_axi_B0_clk = ap_clk;
assign curr_addr_axi_B0_reset = reset;
assign curr_addr_axi_B0_in =
  _guard258 ? read_channel_B0_curr_addr_axi_in :
  _guard259 ? write_channel_B0_curr_addr_axi_in :
  _guard262 ? 64'd4096 :
  'x;
assign max_transfers_Sum0_write_en =
  _guard263 ? aw_channel_Sum0_max_transfers_write_en :
  1'd0;
assign max_transfers_Sum0_clk = ap_clk;
assign max_transfers_Sum0_reset = reset;
assign max_transfers_Sum0_in = aw_channel_Sum0_max_transfers_in;
assign main_compute_A0_read_data =
  _guard265 ? internal_mem_A0_read_data :
  32'd0;
assign main_compute_B0_read_data =
  _guard266 ? internal_mem_B0_read_data :
  32'd0;
assign main_compute_Sum0_done =
  _guard267 ? internal_mem_Sum0_done :
  1'd0;
assign main_compute_clk = ap_clk;
assign main_compute_B0_done =
  _guard268 ? internal_mem_B0_done :
  1'd0;
assign main_compute_go = _guard269;
assign main_compute_reset = reset;
assign main_compute_A0_done =
  _guard270 ? internal_mem_A0_done :
  1'd0;
assign fsm1_write_en = _guard283;
assign fsm1_clk = ap_clk;
assign fsm1_reset = reset;
assign fsm1_in =
  _guard288 ? 2'd1 :
  _guard289 ? 2'd0 :
  _guard294 ? 2'd2 :
  2'd0;
assign fsm4_write_en = _guard313;
assign fsm4_clk = ap_clk;
assign fsm4_reset = reset;
assign fsm4_in =
  _guard318 ? 2'd1 :
  _guard319 ? 2'd0 :
  _guard324 ? 2'd3 :
  _guard329 ? 2'd2 :
  2'd0;
assign invoke11_go_in = _guard335;
assign invoke20_done_in = write_channel_B0_done;
assign invoke23_go_in = _guard341;
assign par1_go_in = _guard347;
assign curr_addr_axi_A0_write_en =
  _guard350 ? 1'd1 :
  _guard351 ? read_channel_A0_curr_addr_axi_write_en :
  _guard352 ? write_channel_A0_curr_addr_axi_write_en :
  1'd0;
assign curr_addr_axi_A0_clk = ap_clk;
assign curr_addr_axi_A0_reset = reset;
assign curr_addr_axi_A0_in =
  _guard353 ? read_channel_A0_curr_addr_axi_in :
  _guard356 ? 64'd4096 :
  _guard357 ? write_channel_A0_curr_addr_axi_in :
  'x;
assign read_channel_A0_curr_addr_internal_mem_out =
  _guard358 ? curr_addr_internal_mem_A0_out :
  3'd0;
assign read_channel_A0_curr_addr_axi_out =
  _guard359 ? curr_addr_axi_A0_out :
  64'd0;
assign read_channel_A0_RVALID =
  _guard360 ? A0_RVALID :
  1'd0;
assign read_channel_A0_RLAST =
  _guard361 ? A0_RLAST :
  1'd0;
assign read_channel_A0_RDATA =
  _guard362 ? A0_RDATA :
  32'd0;
assign read_channel_A0_clk = ap_clk;
assign read_channel_A0_go = _guard363;
assign read_channel_A0_reset = reset;
assign read_channel_A0_RRESP =
  _guard364 ? A0_RRESP :
  2'd0;
assign read_channel_A0_mem_ref_done =
  _guard365 ? internal_mem_A0_done :
  1'd0;
assign read_channel_A0_ARESETn =
  _guard366 ? A0_ARESETn :
  1'd0;
assign read_channel_A0_curr_addr_internal_mem_done =
  _guard367 ? curr_addr_internal_mem_A0_done :
  1'd0;
assign read_channel_A0_curr_addr_axi_done =
  _guard368 ? curr_addr_axi_A0_done :
  1'd0;
assign internal_mem_A0_write_en =
  _guard369 ? main_compute_A0_write_en :
  _guard370 ? read_channel_A0_mem_ref_write_en :
  _guard371 ? write_channel_A0_mem_ref_write_en :
  1'd0;
assign internal_mem_A0_clk = ap_clk;
assign internal_mem_A0_addr0 =
  _guard372 ? main_compute_A0_addr0 :
  _guard373 ? read_channel_A0_mem_ref_addr0 :
  _guard374 ? write_channel_A0_mem_ref_addr0 :
  'x;
assign internal_mem_A0_content_en =
  _guard375 ? main_compute_A0_content_en :
  _guard376 ? read_channel_A0_mem_ref_content_en :
  _guard377 ? write_channel_A0_mem_ref_content_en :
  1'd0;
assign internal_mem_A0_reset = reset;
assign internal_mem_A0_write_data = read_channel_A0_mem_ref_write_data;
assign write_channel_B0_WREADY =
  _guard379 ? B0_WREADY :
  1'd0;
assign write_channel_B0_curr_addr_internal_mem_out =
  _guard380 ? curr_addr_internal_mem_B0_out :
  3'd0;
assign write_channel_B0_curr_addr_axi_out =
  _guard381 ? curr_addr_axi_B0_out :
  64'd0;
assign write_channel_B0_max_transfers_out =
  _guard382 ? max_transfers_B0_out :
  8'd0;
assign write_channel_B0_clk = ap_clk;
assign write_channel_B0_mem_ref_read_data =
  _guard383 ? internal_mem_B0_read_data :
  32'd0;
assign write_channel_B0_go = _guard384;
assign write_channel_B0_reset = reset;
assign write_channel_B0_ARESETn =
  _guard385 ? B0_ARESETn :
  1'd0;
assign write_channel_B0_curr_addr_internal_mem_done =
  _guard386 ? curr_addr_internal_mem_B0_done :
  1'd0;
assign write_channel_B0_curr_addr_axi_done =
  _guard387 ? curr_addr_axi_B0_done :
  1'd0;
assign curr_addr_axi_Sum0_write_en =
  _guard388 ? read_channel_Sum0_curr_addr_axi_write_en :
  _guard391 ? 1'd1 :
  _guard392 ? write_channel_Sum0_curr_addr_axi_write_en :
  1'd0;
assign curr_addr_axi_Sum0_clk = ap_clk;
assign curr_addr_axi_Sum0_reset = reset;
assign curr_addr_axi_Sum0_in =
  _guard393 ? read_channel_Sum0_curr_addr_axi_in :
  _guard396 ? 64'd4096 :
  _guard397 ? write_channel_Sum0_curr_addr_axi_in :
  'x;
assign ar_channel_Sum0_curr_addr_axi_out =
  _guard398 ? curr_addr_axi_Sum0_out :
  64'd0;
assign ar_channel_Sum0_clk = ap_clk;
assign ar_channel_Sum0_go = _guard399;
assign ar_channel_Sum0_reset = reset;
assign ar_channel_Sum0_ARREADY =
  _guard400 ? Sum0_ARREADY :
  1'd0;
assign ar_channel_Sum0_ARESETn =
  _guard401 ? Sum0_ARESETn :
  1'd0;
assign pd1_write_en = _guard410;
assign pd1_clk = ap_clk;
assign pd1_reset = reset;
assign pd1_in =
  _guard413 ? 1'd1 :
  _guard418 ? 1'd0 :
  1'd0;
assign tdcc_go_in = _guard424;
assign invoke12_go_in = _guard430;
assign invoke16_done_in = aw_channel_A0_done;
assign invoke18_done_in = bresp_channel_A0_done;
assign invoke23_done_in = write_channel_Sum0_done;
assign tdcc3_go_in = _guard436;
assign tdcc3_done_in = _guard437;
assign aw_channel_B0_curr_addr_axi_out =
  _guard438 ? curr_addr_axi_B0_out :
  64'd0;
assign aw_channel_B0_clk = ap_clk;
assign aw_channel_B0_AWREADY =
  _guard439 ? B0_AWREADY :
  1'd0;
assign aw_channel_B0_go = _guard440;
assign aw_channel_B0_reset = reset;
assign aw_channel_B0_ARESETn =
  _guard441 ? B0_ARESETn :
  1'd0;
assign fsm0_write_en = _guard454;
assign fsm0_clk = ap_clk;
assign fsm0_reset = reset;
assign fsm0_in =
  _guard459 ? 2'd1 :
  _guard460 ? 2'd0 :
  _guard465 ? 2'd2 :
  2'd0;
assign fsm2_write_en = _guard484;
assign fsm2_clk = ap_clk;
assign fsm2_reset = reset;
assign fsm2_in =
  _guard489 ? 2'd1 :
  _guard490 ? 2'd0 :
  _guard495 ? 2'd3 :
  _guard500 ? 2'd2 :
  2'd0;
assign invoke8_go_in = _guard506;
assign invoke10_done_in = ar_channel_Sum0_done;
assign tdcc0_go_in = _guard512;
assign ar_channel_A0_curr_addr_axi_out =
  _guard513 ? curr_addr_axi_A0_out :
  64'd0;
assign ar_channel_A0_clk = ap_clk;
assign ar_channel_A0_go = _guard514;
assign ar_channel_A0_reset = reset;
assign ar_channel_A0_ARREADY =
  _guard515 ? A0_ARREADY :
  1'd0;
assign ar_channel_A0_ARESETn =
  _guard516 ? A0_ARESETn :
  1'd0;
assign aw_channel_A0_curr_addr_axi_out =
  _guard517 ? curr_addr_axi_A0_out :
  64'd0;
assign aw_channel_A0_clk = ap_clk;
assign aw_channel_A0_AWREADY =
  _guard518 ? A0_AWREADY :
  1'd0;
assign aw_channel_A0_go = _guard519;
assign aw_channel_A0_reset = reset;
assign aw_channel_A0_ARESETn =
  _guard520 ? A0_ARESETn :
  1'd0;
assign par0_done_in = _guard525;
assign invoke8_done_in = ar_channel_B0_done;
assign invoke12_done_in = main_compute_done;
assign invoke17_go_in = _guard531;
assign invoke21_go_in = _guard537;
assign pd2_write_en = _guard546;
assign pd2_clk = ap_clk;
assign pd2_reset = reset;
assign pd2_in =
  _guard549 ? 1'd1 :
  _guard554 ? 1'd0 :
  1'd0;
assign wrapper_early_reset_static_par_thread0_go_in = _guard560;
assign invoke6_done_in = ar_channel_A0_done;
assign invoke16_go_in = _guard566;
assign invoke19_done_in = aw_channel_B0_done;
assign write_channel_A0_WREADY =
  _guard567 ? A0_WREADY :
  1'd0;
assign write_channel_A0_curr_addr_internal_mem_out =
  _guard568 ? curr_addr_internal_mem_A0_out :
  3'd0;
assign write_channel_A0_curr_addr_axi_out =
  _guard569 ? curr_addr_axi_A0_out :
  64'd0;
assign write_channel_A0_max_transfers_out =
  _guard570 ? max_transfers_A0_out :
  8'd0;
assign write_channel_A0_clk = ap_clk;
assign write_channel_A0_mem_ref_read_data =
  _guard571 ? internal_mem_A0_read_data :
  32'd0;
assign write_channel_A0_go = _guard572;
assign write_channel_A0_reset = reset;
assign write_channel_A0_ARESETn =
  _guard573 ? A0_ARESETn :
  1'd0;
assign write_channel_A0_curr_addr_internal_mem_done =
  _guard574 ? curr_addr_internal_mem_A0_done :
  1'd0;
assign write_channel_A0_curr_addr_axi_done =
  _guard575 ? curr_addr_axi_A0_done :
  1'd0;
assign ar_channel_B0_curr_addr_axi_out =
  _guard576 ? curr_addr_axi_B0_out :
  64'd0;
assign ar_channel_B0_clk = ap_clk;
assign ar_channel_B0_go = _guard577;
assign ar_channel_B0_reset = reset;
assign ar_channel_B0_ARREADY =
  _guard578 ? B0_ARREADY :
  1'd0;
assign ar_channel_B0_ARESETn =
  _guard579 ? B0_ARESETn :
  1'd0;
assign signal_reg_write_en = _guard587;
assign signal_reg_clk = ap_clk;
assign signal_reg_reset = reset;
assign signal_reg_in =
  _guard593 ? 1'd1 :
  _guard594 ? 1'd0 :
  1'd0;
assign early_reset_static_par_thread_go_in = _guard595;
assign wrapper_early_reset_static_par_thread0_done_in = _guard596;
assign invoke24_done_in = bresp_channel_Sum0_done;
assign tdcc1_done_in = _guard597;
assign par1_done_in = _guard602;
assign bresp_channel_B0_clk = ap_clk;
assign bresp_channel_B0_go = _guard603;
assign bresp_channel_B0_reset = reset;
assign bresp_channel_B0_BVALID =
  _guard604 ? B0_BVALID :
  1'd0;
assign internal_mem_Sum0_write_en =
  _guard605 ? read_channel_Sum0_mem_ref_write_en :
  _guard606 ? main_compute_Sum0_write_en :
  _guard607 ? write_channel_Sum0_mem_ref_write_en :
  1'd0;
assign internal_mem_Sum0_clk = ap_clk;
assign internal_mem_Sum0_addr0 =
  _guard608 ? read_channel_Sum0_mem_ref_addr0 :
  _guard609 ? main_compute_Sum0_addr0 :
  _guard610 ? write_channel_Sum0_mem_ref_addr0 :
  'x;
assign internal_mem_Sum0_content_en =
  _guard611 ? read_channel_Sum0_mem_ref_content_en :
  _guard612 ? main_compute_Sum0_content_en :
  _guard613 ? write_channel_Sum0_mem_ref_content_en :
  1'd0;
assign internal_mem_Sum0_reset = reset;
assign internal_mem_Sum0_write_data =
  _guard614 ? read_channel_Sum0_mem_ref_write_data :
  _guard615 ? main_compute_Sum0_write_data :
  'x;
assign pd_write_en = _guard624;
assign pd_clk = ap_clk;
assign pd_reset = reset;
assign pd_in =
  _guard627 ? 1'd1 :
  _guard632 ? 1'd0 :
  1'd0;
assign pd0_write_en = _guard641;
assign pd0_clk = ap_clk;
assign pd0_reset = reset;
assign pd0_in =
  _guard644 ? 1'd1 :
  _guard649 ? 1'd0 :
  1'd0;
assign pd4_write_en = _guard658;
assign pd4_clk = ap_clk;
assign pd4_reset = reset;
assign pd4_in =
  _guard661 ? 1'd1 :
  _guard666 ? 1'd0 :
  1'd0;
assign early_reset_static_par_thread0_go_in = _guard667;
assign wrapper_early_reset_static_par_thread_done_in = _guard668;
assign invoke22_done_in = aw_channel_Sum0_done;
assign tdcc4_go_in = _guard674;
assign bresp_channel_A0_clk = ap_clk;
assign bresp_channel_A0_go = _guard675;
assign bresp_channel_A0_reset = reset;
assign bresp_channel_A0_BVALID =
  _guard676 ? A0_BVALID :
  1'd0;
assign tdcc_done_in = _guard677;
assign invoke17_done_in = write_channel_A0_done;
assign invoke19_go_in = _guard683;
assign invoke20_go_in = _guard689;
assign invoke21_done_in = bresp_channel_B0_done;
assign max_transfers_A0_write_en =
  _guard690 ? aw_channel_A0_max_transfers_write_en :
  1'd0;
assign max_transfers_A0_clk = ap_clk;
assign max_transfers_A0_reset = reset;
assign max_transfers_A0_in = aw_channel_A0_max_transfers_in;
assign aw_channel_Sum0_curr_addr_axi_out =
  _guard692 ? curr_addr_axi_Sum0_out :
  64'd0;
assign aw_channel_Sum0_clk = ap_clk;
assign aw_channel_Sum0_AWREADY =
  _guard693 ? Sum0_AWREADY :
  1'd0;
assign aw_channel_Sum0_go = _guard694;
assign aw_channel_Sum0_reset = reset;
assign aw_channel_Sum0_ARESETn =
  _guard695 ? Sum0_ARESETn :
  1'd0;
assign write_channel_Sum0_WREADY =
  _guard696 ? Sum0_WREADY :
  1'd0;
assign write_channel_Sum0_curr_addr_internal_mem_out =
  _guard697 ? curr_addr_internal_mem_Sum0_out :
  3'd0;
assign write_channel_Sum0_curr_addr_axi_out =
  _guard698 ? curr_addr_axi_Sum0_out :
  64'd0;
assign write_channel_Sum0_max_transfers_out =
  _guard699 ? max_transfers_Sum0_out :
  8'd0;
assign write_channel_Sum0_clk = ap_clk;
assign write_channel_Sum0_mem_ref_read_data =
  _guard700 ? internal_mem_Sum0_read_data :
  32'd0;
assign write_channel_Sum0_go = _guard701;
assign write_channel_Sum0_reset = reset;
assign write_channel_Sum0_ARESETn =
  _guard702 ? Sum0_ARESETn :
  1'd0;
assign write_channel_Sum0_curr_addr_internal_mem_done =
  _guard703 ? curr_addr_internal_mem_Sum0_done :
  1'd0;
assign write_channel_Sum0_curr_addr_axi_done =
  _guard704 ? curr_addr_axi_Sum0_done :
  1'd0;
assign pd3_write_en = _guard713;
assign pd3_clk = ap_clk;
assign pd3_reset = reset;
assign pd3_in =
  _guard716 ? 1'd1 :
  _guard721 ? 1'd0 :
  1'd0;
assign tdcc4_done_in = _guard722;
assign max_transfers_B0_write_en =
  _guard723 ? aw_channel_B0_max_transfers_write_en :
  1'd0;
assign max_transfers_B0_clk = ap_clk;
assign max_transfers_B0_reset = reset;
assign max_transfers_B0_in = aw_channel_B0_max_transfers_in;
assign curr_addr_internal_mem_Sum0_write_en =
  _guard725 ? read_channel_Sum0_curr_addr_internal_mem_write_en :
  _guard726 ? 1'd1 :
  _guard727 ? write_channel_Sum0_curr_addr_internal_mem_write_en :
  1'd0;
assign curr_addr_internal_mem_Sum0_clk = ap_clk;
assign curr_addr_internal_mem_Sum0_reset = reset;
assign curr_addr_internal_mem_Sum0_in =
  _guard728 ? read_channel_Sum0_curr_addr_internal_mem_in :
  _guard729 ? write_channel_Sum0_curr_addr_internal_mem_in :
  _guard730 ? 3'd0 :
  'x;
assign early_reset_static_par_thread_done_in = ud_out;
assign invoke6_go_in = _guard736;
assign invoke24_go_in = _guard742;
assign tdcc1_go_in = _guard748;
assign par0_go_in = _guard754;
assign invoke7_go_in = _guard760;
assign invoke10_go_in = _guard766;
assign invoke22_go_in = _guard772;
assign tdcc5_go_in = go;
assign tdcc5_done_in = _guard773;
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
logic ud0_out;
logic signal_reg_in;
logic signal_reg_write_en;
logic signal_reg_clk;
logic signal_reg_reset;
logic signal_reg_out;
logic signal_reg_done;
logic [1:0] fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic [1:0] fsm_out;
logic fsm_done;
logic pd_in;
logic pd_write_en;
logic pd_clk;
logic pd_reset;
logic pd_out;
logic pd_done;
logic [1:0] fsm0_in;
logic fsm0_write_en;
logic fsm0_clk;
logic fsm0_reset;
logic [1:0] fsm0_out;
logic fsm0_done;
logic pd0_in;
logic pd0_write_en;
logic pd0_clk;
logic pd0_reset;
logic pd0_out;
logic pd0_done;
logic [2:0] fsm1_in;
logic fsm1_write_en;
logic fsm1_clk;
logic fsm1_reset;
logic [2:0] fsm1_out;
logic fsm1_done;
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
logic early_reset_cond000_go_in;
logic early_reset_cond000_go_out;
logic early_reset_cond000_done_in;
logic early_reset_cond000_done_out;
logic wrapper_early_reset_cond000_go_in;
logic wrapper_early_reset_cond000_go_out;
logic wrapper_early_reset_cond000_done_in;
logic wrapper_early_reset_cond000_done_out;
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
undef # (
    .WIDTH(1)
) ud0 (
    .out(ud0_out)
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
) fsm (
    .clk(fsm_clk),
    .done(fsm_done),
    .in(fsm_in),
    .out(fsm_out),
    .reset(fsm_reset),
    .write_en(fsm_write_en)
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
) fsm1 (
    .clk(fsm1_clk),
    .done(fsm1_done),
    .in(fsm1_in),
    .out(fsm1_out),
    .reset(fsm1_reset),
    .write_en(fsm1_write_en)
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
) early_reset_cond000_go (
    .in(early_reset_cond000_go_in),
    .out(early_reset_cond000_go_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_cond000_done (
    .in(early_reset_cond000_done_in),
    .out(early_reset_cond000_done_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_cond000_go (
    .in(wrapper_early_reset_cond000_go_in),
    .out(wrapper_early_reset_cond000_go_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_cond000_done (
    .in(wrapper_early_reset_cond000_done_in),
    .out(wrapper_early_reset_cond000_done_out)
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
wire _guard6 = invoke3_go_out;
wire _guard7 = invoke3_go_out;
wire _guard8 = tdcc1_done_out;
wire _guard9 = upd2_go_out;
wire _guard10 = beg_spl_upd1_go_out;
wire _guard11 = upd2_go_out;
wire _guard12 = _guard10 | _guard11;
wire _guard13 = beg_spl_upd1_go_out;
wire _guard14 = upd2_go_out;
wire _guard15 = _guard13 | _guard14;
wire _guard16 = beg_spl_upd0_go_out;
wire _guard17 = upd2_go_out;
wire _guard18 = _guard16 | _guard17;
wire _guard19 = upd2_go_out;
wire _guard20 = upd2_go_out;
wire _guard21 = upd2_go_out;
wire _guard22 = beg_spl_upd0_go_out;
wire _guard23 = upd2_go_out;
wire _guard24 = _guard22 | _guard23;
wire _guard25 = fsm_out == 2'd2;
wire _guard26 = fsm_out == 2'd0;
wire _guard27 = beg_spl_upd0_done_out;
wire _guard28 = _guard26 & _guard27;
wire _guard29 = tdcc_go_out;
wire _guard30 = _guard28 & _guard29;
wire _guard31 = _guard25 | _guard30;
wire _guard32 = fsm_out == 2'd1;
wire _guard33 = invoke1_done_out;
wire _guard34 = _guard32 & _guard33;
wire _guard35 = tdcc_go_out;
wire _guard36 = _guard34 & _guard35;
wire _guard37 = _guard31 | _guard36;
wire _guard38 = fsm_out == 2'd0;
wire _guard39 = beg_spl_upd0_done_out;
wire _guard40 = _guard38 & _guard39;
wire _guard41 = tdcc_go_out;
wire _guard42 = _guard40 & _guard41;
wire _guard43 = fsm_out == 2'd2;
wire _guard44 = fsm_out == 2'd1;
wire _guard45 = invoke1_done_out;
wire _guard46 = _guard44 & _guard45;
wire _guard47 = tdcc_go_out;
wire _guard48 = _guard46 & _guard47;
wire _guard49 = beg_spl_upd0_done_out;
wire _guard50 = ~_guard49;
wire _guard51 = fsm_out == 2'd0;
wire _guard52 = _guard50 & _guard51;
wire _guard53 = tdcc_go_out;
wire _guard54 = _guard52 & _guard53;
wire _guard55 = upd2_go_out;
wire _guard56 = upd2_go_out;
wire _guard57 = invoke2_done_out;
wire _guard58 = ~_guard57;
wire _guard59 = fsm0_out == 2'd1;
wire _guard60 = _guard58 & _guard59;
wire _guard61 = tdcc0_go_out;
wire _guard62 = _guard60 & _guard61;
wire _guard63 = fsm0_out == 2'd2;
wire _guard64 = early_reset_cond000_go_out;
wire _guard65 = early_reset_cond000_go_out;
wire _guard66 = wrapper_early_reset_cond000_done_out;
wire _guard67 = ~_guard66;
wire _guard68 = fsm1_out == 3'd1;
wire _guard69 = _guard67 & _guard68;
wire _guard70 = tdcc1_go_out;
wire _guard71 = _guard69 & _guard70;
wire _guard72 = wrapper_early_reset_cond000_done_out;
wire _guard73 = ~_guard72;
wire _guard74 = fsm1_out == 3'd5;
wire _guard75 = _guard73 & _guard74;
wire _guard76 = tdcc1_go_out;
wire _guard77 = _guard75 & _guard76;
wire _guard78 = _guard71 | _guard77;
wire _guard79 = fsm1_out == 3'd6;
wire _guard80 = fsm1_out == 3'd0;
wire _guard81 = invoke0_done_out;
wire _guard82 = _guard80 & _guard81;
wire _guard83 = tdcc1_go_out;
wire _guard84 = _guard82 & _guard83;
wire _guard85 = _guard79 | _guard84;
wire _guard86 = fsm1_out == 3'd1;
wire _guard87 = wrapper_early_reset_cond000_done_out;
wire _guard88 = comb_reg_out;
wire _guard89 = _guard87 & _guard88;
wire _guard90 = _guard86 & _guard89;
wire _guard91 = tdcc1_go_out;
wire _guard92 = _guard90 & _guard91;
wire _guard93 = _guard85 | _guard92;
wire _guard94 = fsm1_out == 3'd5;
wire _guard95 = wrapper_early_reset_cond000_done_out;
wire _guard96 = comb_reg_out;
wire _guard97 = _guard95 & _guard96;
wire _guard98 = _guard94 & _guard97;
wire _guard99 = tdcc1_go_out;
wire _guard100 = _guard98 & _guard99;
wire _guard101 = _guard93 | _guard100;
wire _guard102 = fsm1_out == 3'd2;
wire _guard103 = par0_done_out;
wire _guard104 = _guard102 & _guard103;
wire _guard105 = tdcc1_go_out;
wire _guard106 = _guard104 & _guard105;
wire _guard107 = _guard101 | _guard106;
wire _guard108 = fsm1_out == 3'd3;
wire _guard109 = upd2_done_out;
wire _guard110 = _guard108 & _guard109;
wire _guard111 = tdcc1_go_out;
wire _guard112 = _guard110 & _guard111;
wire _guard113 = _guard107 | _guard112;
wire _guard114 = fsm1_out == 3'd4;
wire _guard115 = invoke3_done_out;
wire _guard116 = _guard114 & _guard115;
wire _guard117 = tdcc1_go_out;
wire _guard118 = _guard116 & _guard117;
wire _guard119 = _guard113 | _guard118;
wire _guard120 = fsm1_out == 3'd1;
wire _guard121 = wrapper_early_reset_cond000_done_out;
wire _guard122 = comb_reg_out;
wire _guard123 = ~_guard122;
wire _guard124 = _guard121 & _guard123;
wire _guard125 = _guard120 & _guard124;
wire _guard126 = tdcc1_go_out;
wire _guard127 = _guard125 & _guard126;
wire _guard128 = _guard119 | _guard127;
wire _guard129 = fsm1_out == 3'd5;
wire _guard130 = wrapper_early_reset_cond000_done_out;
wire _guard131 = comb_reg_out;
wire _guard132 = ~_guard131;
wire _guard133 = _guard130 & _guard132;
wire _guard134 = _guard129 & _guard133;
wire _guard135 = tdcc1_go_out;
wire _guard136 = _guard134 & _guard135;
wire _guard137 = _guard128 | _guard136;
wire _guard138 = fsm1_out == 3'd1;
wire _guard139 = wrapper_early_reset_cond000_done_out;
wire _guard140 = comb_reg_out;
wire _guard141 = ~_guard140;
wire _guard142 = _guard139 & _guard141;
wire _guard143 = _guard138 & _guard142;
wire _guard144 = tdcc1_go_out;
wire _guard145 = _guard143 & _guard144;
wire _guard146 = fsm1_out == 3'd5;
wire _guard147 = wrapper_early_reset_cond000_done_out;
wire _guard148 = comb_reg_out;
wire _guard149 = ~_guard148;
wire _guard150 = _guard147 & _guard149;
wire _guard151 = _guard146 & _guard150;
wire _guard152 = tdcc1_go_out;
wire _guard153 = _guard151 & _guard152;
wire _guard154 = _guard145 | _guard153;
wire _guard155 = fsm1_out == 3'd4;
wire _guard156 = invoke3_done_out;
wire _guard157 = _guard155 & _guard156;
wire _guard158 = tdcc1_go_out;
wire _guard159 = _guard157 & _guard158;
wire _guard160 = fsm1_out == 3'd1;
wire _guard161 = wrapper_early_reset_cond000_done_out;
wire _guard162 = comb_reg_out;
wire _guard163 = _guard161 & _guard162;
wire _guard164 = _guard160 & _guard163;
wire _guard165 = tdcc1_go_out;
wire _guard166 = _guard164 & _guard165;
wire _guard167 = fsm1_out == 3'd5;
wire _guard168 = wrapper_early_reset_cond000_done_out;
wire _guard169 = comb_reg_out;
wire _guard170 = _guard168 & _guard169;
wire _guard171 = _guard167 & _guard170;
wire _guard172 = tdcc1_go_out;
wire _guard173 = _guard171 & _guard172;
wire _guard174 = _guard166 | _guard173;
wire _guard175 = fsm1_out == 3'd3;
wire _guard176 = upd2_done_out;
wire _guard177 = _guard175 & _guard176;
wire _guard178 = tdcc1_go_out;
wire _guard179 = _guard177 & _guard178;
wire _guard180 = fsm1_out == 3'd0;
wire _guard181 = invoke0_done_out;
wire _guard182 = _guard180 & _guard181;
wire _guard183 = tdcc1_go_out;
wire _guard184 = _guard182 & _guard183;
wire _guard185 = fsm1_out == 3'd6;
wire _guard186 = fsm1_out == 3'd2;
wire _guard187 = par0_done_out;
wire _guard188 = _guard186 & _guard187;
wire _guard189 = tdcc1_go_out;
wire _guard190 = _guard188 & _guard189;
wire _guard191 = pd_out;
wire _guard192 = tdcc_done_out;
wire _guard193 = _guard191 | _guard192;
wire _guard194 = ~_guard193;
wire _guard195 = par0_go_out;
wire _guard196 = _guard194 & _guard195;
wire _guard197 = invoke0_done_out;
wire _guard198 = ~_guard197;
wire _guard199 = fsm1_out == 3'd0;
wire _guard200 = _guard198 & _guard199;
wire _guard201 = tdcc1_go_out;
wire _guard202 = _guard200 & _guard201;
wire _guard203 = fsm0_out == 2'd2;
wire _guard204 = fsm0_out == 2'd0;
wire _guard205 = beg_spl_upd1_done_out;
wire _guard206 = _guard204 & _guard205;
wire _guard207 = tdcc0_go_out;
wire _guard208 = _guard206 & _guard207;
wire _guard209 = _guard203 | _guard208;
wire _guard210 = fsm0_out == 2'd1;
wire _guard211 = invoke2_done_out;
wire _guard212 = _guard210 & _guard211;
wire _guard213 = tdcc0_go_out;
wire _guard214 = _guard212 & _guard213;
wire _guard215 = _guard209 | _guard214;
wire _guard216 = fsm0_out == 2'd0;
wire _guard217 = beg_spl_upd1_done_out;
wire _guard218 = _guard216 & _guard217;
wire _guard219 = tdcc0_go_out;
wire _guard220 = _guard218 & _guard219;
wire _guard221 = fsm0_out == 2'd2;
wire _guard222 = fsm0_out == 2'd1;
wire _guard223 = invoke2_done_out;
wire _guard224 = _guard222 & _guard223;
wire _guard225 = tdcc0_go_out;
wire _guard226 = _guard224 & _guard225;
wire _guard227 = pd0_out;
wire _guard228 = tdcc0_done_out;
wire _guard229 = _guard227 | _guard228;
wire _guard230 = ~_guard229;
wire _guard231 = par0_go_out;
wire _guard232 = _guard230 & _guard231;
wire _guard233 = wrapper_early_reset_cond000_go_out;
wire _guard234 = pd_out;
wire _guard235 = pd0_out;
wire _guard236 = _guard234 & _guard235;
wire _guard237 = invoke1_done_out;
wire _guard238 = ~_guard237;
wire _guard239 = fsm_out == 2'd1;
wire _guard240 = _guard238 & _guard239;
wire _guard241 = tdcc_go_out;
wire _guard242 = _guard240 & _guard241;
wire _guard243 = beg_spl_upd1_done_out;
wire _guard244 = ~_guard243;
wire _guard245 = fsm0_out == 2'd0;
wire _guard246 = _guard244 & _guard245;
wire _guard247 = tdcc0_go_out;
wire _guard248 = _guard246 & _guard247;
wire _guard249 = signal_reg_out;
wire _guard250 = early_reset_cond000_go_out;
wire _guard251 = early_reset_cond000_go_out;
wire _guard252 = signal_reg_out;
wire _guard253 = _guard0 & _guard0;
wire _guard254 = signal_reg_out;
wire _guard255 = ~_guard254;
wire _guard256 = _guard253 & _guard255;
wire _guard257 = wrapper_early_reset_cond000_go_out;
wire _guard258 = _guard256 & _guard257;
wire _guard259 = _guard252 | _guard258;
wire _guard260 = _guard0 & _guard0;
wire _guard261 = signal_reg_out;
wire _guard262 = ~_guard261;
wire _guard263 = _guard260 & _guard262;
wire _guard264 = wrapper_early_reset_cond000_go_out;
wire _guard265 = _guard263 & _guard264;
wire _guard266 = signal_reg_out;
wire _guard267 = fsm1_out == 3'd6;
wire _guard268 = invoke2_go_out;
wire _guard269 = invoke2_go_out;
wire _guard270 = pd_out;
wire _guard271 = pd0_out;
wire _guard272 = _guard270 & _guard271;
wire _guard273 = tdcc_done_out;
wire _guard274 = par0_go_out;
wire _guard275 = _guard273 & _guard274;
wire _guard276 = _guard272 | _guard275;
wire _guard277 = tdcc_done_out;
wire _guard278 = par0_go_out;
wire _guard279 = _guard277 & _guard278;
wire _guard280 = pd_out;
wire _guard281 = pd0_out;
wire _guard282 = _guard280 & _guard281;
wire _guard283 = pd_out;
wire _guard284 = pd0_out;
wire _guard285 = _guard283 & _guard284;
wire _guard286 = tdcc0_done_out;
wire _guard287 = par0_go_out;
wire _guard288 = _guard286 & _guard287;
wire _guard289 = _guard285 | _guard288;
wire _guard290 = tdcc0_done_out;
wire _guard291 = par0_go_out;
wire _guard292 = _guard290 & _guard291;
wire _guard293 = pd_out;
wire _guard294 = pd0_out;
wire _guard295 = _guard293 & _guard294;
wire _guard296 = fsm_out == 2'd2;
wire _guard297 = upd2_done_out;
wire _guard298 = ~_guard297;
wire _guard299 = fsm1_out == 3'd3;
wire _guard300 = _guard298 & _guard299;
wire _guard301 = tdcc1_go_out;
wire _guard302 = _guard300 & _guard301;
wire _guard303 = invoke3_done_out;
wire _guard304 = ~_guard303;
wire _guard305 = fsm1_out == 3'd4;
wire _guard306 = _guard304 & _guard305;
wire _guard307 = tdcc1_go_out;
wire _guard308 = _guard306 & _guard307;
wire _guard309 = invoke1_go_out;
wire _guard310 = invoke1_go_out;
wire _guard311 = par0_done_out;
wire _guard312 = ~_guard311;
wire _guard313 = fsm1_out == 3'd2;
wire _guard314 = _guard312 & _guard313;
wire _guard315 = tdcc1_go_out;
wire _guard316 = _guard314 & _guard315;
assign i0_write_en = _guard3;
assign i0_clk = clk;
assign i0_reset = reset;
assign i0_in =
  _guard4 ? add1_out :
  _guard5 ? const0_out :
  'x;
assign upd2_done_in = Sum0_done;
assign add1_left = i0_out;
assign add1_right = const2_out;
assign done = _guard8;
assign B0_write_en = 1'd0;
assign Sum0_addr0 = bit_slice_out;
assign A0_write_en = 1'd0;
assign B0_addr0 = bit_slice_out;
assign B0_content_en = _guard15;
assign A0_addr0 = bit_slice_out;
assign Sum0_write_en = _guard19;
assign Sum0_content_en = _guard20;
assign Sum0_write_data = add0_out;
assign A0_content_en = _guard24;
assign fsm_write_en = _guard37;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard42 ? 2'd1 :
  _guard43 ? 2'd0 :
  _guard48 ? 2'd2 :
  2'd0;
assign beg_spl_upd0_go_in = _guard54;
assign early_reset_cond000_done_in = ud0_out;
assign add0_left = B_read0_0_out;
assign add0_right = A_read0_0_out;
assign invoke2_go_in = _guard62;
assign tdcc0_done_in = _guard63;
assign comb_reg_write_en = _guard64;
assign comb_reg_clk = clk;
assign comb_reg_reset = reset;
assign comb_reg_in =
  _guard65 ? le0_out :
  1'd0;
assign wrapper_early_reset_cond000_go_in = _guard78;
assign fsm1_write_en = _guard137;
assign fsm1_clk = clk;
assign fsm1_reset = reset;
assign fsm1_in =
  _guard154 ? 3'd6 :
  _guard159 ? 3'd5 :
  _guard174 ? 3'd2 :
  _guard179 ? 3'd4 :
  _guard184 ? 3'd1 :
  _guard185 ? 3'd0 :
  _guard190 ? 3'd3 :
  3'd0;
assign tdcc_go_in = _guard196;
assign invoke0_go_in = _guard202;
assign beg_spl_upd0_done_in = A0_done;
assign bit_slice_in = i0_out;
assign fsm0_write_en = _guard215;
assign fsm0_clk = clk;
assign fsm0_reset = reset;
assign fsm0_in =
  _guard220 ? 2'd1 :
  _guard221 ? 2'd0 :
  _guard226 ? 2'd2 :
  2'd0;
assign tdcc0_go_in = _guard232;
assign invoke3_done_in = i0_done;
assign early_reset_cond000_go_in = _guard233;
assign par0_done_in = _guard236;
assign invoke0_done_in = i0_done;
assign invoke1_go_in = _guard242;
assign beg_spl_upd1_go_in = _guard248;
assign wrapper_early_reset_cond000_done_in = _guard249;
assign le0_left =
  _guard250 ? i0_out :
  4'd0;
assign le0_right =
  _guard251 ? const1_out :
  4'd0;
assign signal_reg_write_en = _guard259;
assign signal_reg_clk = clk;
assign signal_reg_reset = reset;
assign signal_reg_in =
  _guard265 ? 1'd1 :
  _guard266 ? 1'd0 :
  1'd0;
assign invoke2_done_in = B_read0_0_done;
assign tdcc1_done_in = _guard267;
assign beg_spl_upd1_done_in = B0_done;
assign B_read0_0_write_en = _guard268;
assign B_read0_0_clk = clk;
assign B_read0_0_reset = reset;
assign B_read0_0_in = B0_read_data;
assign pd_write_en = _guard276;
assign pd_clk = clk;
assign pd_reset = reset;
assign pd_in =
  _guard279 ? 1'd1 :
  _guard282 ? 1'd0 :
  1'd0;
assign pd0_write_en = _guard289;
assign pd0_clk = clk;
assign pd0_reset = reset;
assign pd0_in =
  _guard292 ? 1'd1 :
  _guard295 ? 1'd0 :
  1'd0;
assign tdcc_done_in = _guard296;
assign upd2_go_in = _guard302;
assign invoke3_go_in = _guard308;
assign invoke1_done_in = A_read0_0_done;
assign tdcc1_go_in = go;
assign A_read0_0_write_en = _guard309;
assign A_read0_0_clk = clk;
assign A_read0_0_reset = reset;
assign A_read0_0_in = A0_read_data;
assign par0_go_in = _guard316;
// COMPONENT END: main
endmodule
