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

   // Read signal
   input wire logic read_en,
   output logic [ WIDTH-1:0] read_data,
   output logic read_done,

   // Write signals
   input wire logic [ WIDTH-1:0] write_data,
   input wire logic write_en,
   output logic write_done
);
  // Internal memory
  (* ram_style = "ultra" *)  logic [WIDTH-1:0] mem[SIZE-1:0];

  // Register for the read output
  logic [WIDTH-1:0] read_out;
  assign read_data = read_out;

  // Read value from the memory
  always_ff @(posedge clk) begin
    if (reset) begin
      read_out <= '0;
    end else if (read_en) begin
      /* verilator lint_off WIDTH */
      read_out <= mem[addr0];
    end else if (write_en) begin
      // Explicitly clobber the read output when a write is performed
      read_out <= 'x;
    end else begin
      read_out <= read_out;
    end
  end

  // Propagate the read_done signal
  always_ff @(posedge clk) begin
    if (reset) begin
      read_done <= '0;
    end else if (read_en) begin
      read_done <= '1;
    end else begin
      read_done <= '0;
    end
  end

  // Write value to the memory
  always_ff @(posedge clk) begin
    if (!reset && write_en)
      mem[addr0] <= write_data;
  end

  // Propagate the write_done signal
  always_ff @(posedge clk) begin
    if (reset) begin
      write_done <= '0;
    end else if (write_en) begin
      write_done <= 1'd1;
    end else begin
      write_done <= '0;
    end
  end

  // Check for out of bounds access
  `ifdef VERILATOR
    always_comb begin
      if (read_en)
        if (addr0 >= SIZE)
          $error(
            "std_mem_d1: Out of bounds access\n",
            "addr0: %0d\n", addr0,
            "SIZE: %0d", SIZE
          );
    end
    always_comb begin
      if (read_en && write_en)
        $error("Simultaneous read and write attempted\n");
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

   // Read signal
   input wire logic read_en,
   output logic [WIDTH-1:0] read_data,
   output logic read_done,

   // Write signals
   input wire logic write_en,
   input wire logic [ WIDTH-1:0] write_data,
   output logic write_done
);
  wire [D0_IDX_SIZE+D1_IDX_SIZE-1:0] addr;
  assign addr = addr0 * D1_SIZE + addr1;

  seq_mem_d1 #(.WIDTH(WIDTH), .SIZE(D0_SIZE * D1_SIZE), .IDX_SIZE(D0_IDX_SIZE+D1_IDX_SIZE)) mem
     (.clk(clk), .reset(reset), .addr0(addr),
    .read_en(read_en), .read_data(read_data), .read_done(read_done), .write_data(write_data), .write_en(write_en),
    .write_done(write_done));
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

   // Read signal
   input wire logic read_en,
   output logic [WIDTH-1:0] read_data,
   output logic read_done,

   // Write signals
   input wire logic write_en,
   input wire logic [ WIDTH-1:0] write_data,
   output logic write_done
);
  wire [D0_IDX_SIZE+D1_IDX_SIZE+D2_IDX_SIZE-1:0] addr;
  assign addr = addr0 * (D1_SIZE * D2_SIZE) + addr1 * (D2_SIZE) + addr2;

  seq_mem_d1 #(.WIDTH(WIDTH), .SIZE(D0_SIZE * D1_SIZE * D2_SIZE), .IDX_SIZE(D0_IDX_SIZE+D1_IDX_SIZE+D2_IDX_SIZE)) mem
     (.clk(clk), .reset(reset), .addr0(addr),
    .read_en(read_en), .read_data(read_data), .read_done(read_done), .write_data(write_data), .write_en(write_en),
    .write_done(write_done));
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

   // Read signal
   input wire logic read_en,
   output logic [WIDTH-1:0] read_data,
   output logic read_done,

   // Write signals
   input wire logic write_en,
   input wire logic [ WIDTH-1:0] write_data,
   output logic write_done
);
  wire [D0_IDX_SIZE+D1_IDX_SIZE+D2_IDX_SIZE+D3_IDX_SIZE-1:0] addr;
  assign addr = addr0 * (D1_SIZE * D2_SIZE * D3_SIZE) + addr1 * (D2_SIZE * D3_SIZE) + addr2 * (D3_SIZE) + addr3;

  seq_mem_d1 #(.WIDTH(WIDTH), .SIZE(D0_SIZE * D1_SIZE * D2_SIZE * D3_SIZE), .IDX_SIZE(D0_IDX_SIZE+D1_IDX_SIZE+D2_IDX_SIZE+D3_IDX_SIZE)) mem
     (.clk(clk), .reset(reset), .addr0(addr),
    .read_en(read_en), .read_data(read_data), .read_done(read_done), .write_data(write_data), .write_en(write_en),
    .write_done(write_done));
endmodule
module fp_sqrt #(
    parameter WIDTH = 32,
    parameter INT_WIDTH = 16,
    parameter FRAC_WIDTH = 16
) (
    input  logic             clk,
    input  logic             reset,
    input  logic             go,
    input  logic [WIDTH-1:0] in,
    output logic [WIDTH-1:0] out,
    output logic             done
);
    // The algorithm requires an even number of bits to the left of the binary
    // point. Thus, if INT_WIDTH is odd, we extend the input to include the
    // implicit leading 0.
    localparam EXT_WIDTH = WIDTH + (INT_WIDTH & 1);
    localparam ITERATIONS = EXT_WIDTH+FRAC_WIDTH >> 1;
    logic [$clog2(ITERATIONS)-1:0] idx;

    logic [EXT_WIDTH-1:0] x, x_next;
    logic [EXT_WIDTH-1:0] quotient, quotient_next;
    logic [EXT_WIDTH+1:0] acc, acc_next;
    logic [EXT_WIDTH+1:0] tmp;
    logic start, running, finished;

    assign start = go && !running;
    /* verilator lint_off WIDTH */
    assign finished = (ITERATIONS - 1) == idx && running;

    always_ff @(posedge clk) begin
      if (reset || finished)
        running <= 0;
      else if (start)
        running <= 1;
      else
        running <= running;
    end

    always_ff @(posedge clk) begin
      if (running)
        idx <= idx + 1;
      else
        idx <= 0;
    end

    always_comb begin
      tmp = acc - {quotient, 2'b01};
      if (tmp[EXT_WIDTH+1]) begin
        // tmp is negative.
        {acc_next, x_next} = {acc[EXT_WIDTH-1:0], x, 2'b0};
        // Append a 0 to the result.
        quotient_next = quotient << 1;
      end else begin
        // tmp is positive.
        {acc_next, x_next} = {tmp[EXT_WIDTH-1:0], x, 2'b0};
        // Append a 1 to the result.
        quotient_next = {quotient[EXT_WIDTH-2:0], 1'b1};
      end
    end

    always_ff @(posedge clk) begin
      if (start) begin
        quotient <= 0;
        {acc, x} <= {{EXT_WIDTH + (INT_WIDTH & 1){1'b0}}, in, 2'b0};
      end else begin
        x <= x_next;
        acc <= acc_next;
        quotient <= quotient_next;
      end
    end

    always_ff @(posedge clk) begin
      if (finished) begin
        done <= 1;
        out <= quotient_next;
      end else if (reset) begin
        done <= 0;
        out <= 0;
      end else begin
        done <= 0;
        out <= out;
      end
    end

endmodule

module sqrt #(
    parameter WIDTH = 32
) (
    input  logic             clk,
    input  logic             go,
    input  logic             reset,
    input  logic [WIDTH-1:0] in,
    output logic [WIDTH-1:0] out,
    output logic             done
);
  fp_sqrt #(
    .WIDTH(WIDTH),
    .INT_WIDTH(WIDTH),
    .FRAC_WIDTH(0)
  ) comp (
    .clk(clk),
    .done(done),
    .reset(reset),
    .go(go),
    .in(in),
    .out(out)
  );

  // Simulation self test against unsynthesizable implementation.
  `ifdef VERILATOR
    logic [WIDTH-1:0] radicand;
    always_ff @(posedge clk) begin
      if (go)
        radicand <= in;
      else
        radicand <= radicand;
    end

    always @(posedge clk) begin
      if (done && out != $floor($sqrt(radicand)))
        $error(
          "\nsqrt: Computed and golden outputs do not match!\n",
          "input: %0d\n", radicand,
          /* verilator lint_off REALCVT */
          "expected: %0d\n", $floor($sqrt(radicand)),
          "computed: %0d", out
        );
    end
  `endif
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

module m_arread_channel(
  input logic ARESET,
  input logic ARREADY,
  output logic ARVALID,
  output logic [63:0] ARADDR,
  output logic [2:0] ARSIZE,
  output logic [7:0] ARLEN,
  output logic [1:0] ARBURST,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done,
  output logic [63:0] base_addr_in,
  output logic base_addr_write_en,
  input logic [63:0] base_addr_out,
  input logic base_addr_done
);
// COMPONENT START: m_arread_channel
logic is_arvalid_in;
logic is_arvalid_write_en;
logic is_arvalid_clk;
logic is_arvalid_reset;
logic is_arvalid_out;
logic is_arvalid_done;
logic arvalid_was_high_in;
logic arvalid_was_high_write_en;
logic arvalid_was_high_clk;
logic arvalid_was_high_reset;
logic arvalid_was_high_out;
logic arvalid_was_high_done;
logic [7:0] txn_len_in;
logic txn_len_write_en;
logic txn_len_clk;
logic txn_len_reset;
logic [7:0] txn_len_out;
logic txn_len_done;
logic [31:0] txn_n_out;
logic [31:0] txn_count_in;
logic txn_count_write_en;
logic txn_count_clk;
logic txn_count_reset;
logic [31:0] txn_count_out;
logic txn_count_done;
logic [31:0] perform_reads_left;
logic [31:0] perform_reads_right;
logic perform_reads_out;
logic [31:0] txn_adder_left;
logic [31:0] txn_adder_right;
logic [31:0] txn_adder_out;
logic block_transfer_done_and_left;
logic block_transfer_done_and_right;
logic block_transfer_done_and_out;
logic bt_reg_in;
logic bt_reg_write_en;
logic bt_reg_clk;
logic bt_reg_reset;
logic bt_reg_out;
logic bt_reg_done;
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
logic ud_out;
logic adder_left;
logic adder_right;
logic adder_out;
logic ud0_out;
logic adder0_left;
logic adder0_right;
logic adder0_out;
logic ud1_out;
logic adder1_left;
logic adder1_right;
logic adder1_out;
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
logic do_ar_transfer_go_in;
logic do_ar_transfer_go_out;
logic do_ar_transfer_done_in;
logic do_ar_transfer_done_out;
logic invoke2_go_in;
logic invoke2_go_out;
logic invoke2_done_in;
logic invoke2_done_out;
logic early_reset_check_reads_done0_go_in;
logic early_reset_check_reads_done0_go_out;
logic early_reset_check_reads_done0_done_in;
logic early_reset_check_reads_done0_done_out;
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
logic wrapper_early_reset_check_reads_done0_go_in;
logic wrapper_early_reset_check_reads_done0_go_out;
logic wrapper_early_reset_check_reads_done0_done_in;
logic wrapper_early_reset_check_reads_done0_done_out;
logic wrapper_early_reset_static_par0_go_in;
logic wrapper_early_reset_static_par0_go_out;
logic wrapper_early_reset_static_par0_done_in;
logic wrapper_early_reset_static_par0_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
std_reg # (
    .WIDTH(1)
) is_arvalid (
    .clk(is_arvalid_clk),
    .done(is_arvalid_done),
    .in(is_arvalid_in),
    .out(is_arvalid_out),
    .reset(is_arvalid_reset),
    .write_en(is_arvalid_write_en)
);
std_reg # (
    .WIDTH(1)
) arvalid_was_high (
    .clk(arvalid_was_high_clk),
    .done(arvalid_was_high_done),
    .in(arvalid_was_high_in),
    .out(arvalid_was_high_out),
    .reset(arvalid_was_high_reset),
    .write_en(arvalid_was_high_write_en)
);
std_reg # (
    .WIDTH(8)
) txn_len (
    .clk(txn_len_clk),
    .done(txn_len_done),
    .in(txn_len_in),
    .out(txn_len_out),
    .reset(txn_len_reset),
    .write_en(txn_len_write_en)
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
std_neq # (
    .WIDTH(32)
) perform_reads (
    .left(perform_reads_left),
    .out(perform_reads_out),
    .right(perform_reads_right)
);
std_add # (
    .WIDTH(32)
) txn_adder (
    .left(txn_adder_left),
    .out(txn_adder_out),
    .right(txn_adder_right)
);
std_and # (
    .WIDTH(1)
) block_transfer_done_and (
    .left(block_transfer_done_and_left),
    .out(block_transfer_done_and_out),
    .right(block_transfer_done_and_right)
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
undef # (
    .WIDTH(1)
) ud1 (
    .out(ud1_out)
);
std_add # (
    .WIDTH(1)
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
) early_reset_check_reads_done0_go (
    .in(early_reset_check_reads_done0_go_in),
    .out(early_reset_check_reads_done0_go_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_check_reads_done0_done (
    .in(early_reset_check_reads_done0_done_in),
    .out(early_reset_check_reads_done0_done_out)
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
) wrapper_early_reset_check_reads_done0_go (
    .in(wrapper_early_reset_check_reads_done0_go_in),
    .out(wrapper_early_reset_check_reads_done0_go_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_check_reads_done0_done (
    .in(wrapper_early_reset_check_reads_done0_done_in),
    .out(wrapper_early_reset_check_reads_done0_done_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_static_par0_go (
    .in(wrapper_early_reset_static_par0_go_in),
    .out(wrapper_early_reset_static_par0_go_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_static_par0_done (
    .in(wrapper_early_reset_static_par0_done_in),
    .out(wrapper_early_reset_static_par0_done_out)
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
wire _guard1 = is_arvalid_out;
wire _guard2 = ARREADY;
wire _guard3 = _guard1 & _guard2;
wire _guard4 = ~_guard3;
wire _guard5 = arvalid_was_high_out;
wire _guard6 = ~_guard5;
wire _guard7 = _guard4 & _guard6;
wire _guard8 = do_ar_transfer_go_out;
wire _guard9 = _guard7 & _guard8;
wire _guard10 = is_arvalid_out;
wire _guard11 = ARREADY;
wire _guard12 = _guard10 & _guard11;
wire _guard13 = ~_guard12;
wire _guard14 = arvalid_was_high_out;
wire _guard15 = ~_guard14;
wire _guard16 = _guard13 & _guard15;
wire _guard17 = do_ar_transfer_go_out;
wire _guard18 = _guard16 & _guard17;
wire _guard19 = early_reset_static_par0_go_out;
wire _guard20 = early_reset_static_par0_go_out;
wire _guard21 = do_ar_transfer_done_out;
wire _guard22 = ~_guard21;
wire _guard23 = fsm0_out == 3'd3;
wire _guard24 = _guard22 & _guard23;
wire _guard25 = tdcc_go_out;
wire _guard26 = _guard24 & _guard25;
wire _guard27 = tdcc_done_out;
wire _guard28 = do_ar_transfer_go_out;
wire _guard29 = do_ar_transfer_go_out;
wire _guard30 = do_ar_transfer_go_out;
wire _guard31 = do_ar_transfer_go_out;
wire _guard32 = early_reset_check_reads_done0_go_out;
wire _guard33 = early_reset_static_par_go_out;
wire _guard34 = _guard32 | _guard33;
wire _guard35 = early_reset_static_par0_go_out;
wire _guard36 = _guard34 | _guard35;
wire _guard37 = fsm_out != 1'd0;
wire _guard38 = early_reset_static_par0_go_out;
wire _guard39 = _guard37 & _guard38;
wire _guard40 = fsm_out != 1'd0;
wire _guard41 = early_reset_check_reads_done0_go_out;
wire _guard42 = _guard40 & _guard41;
wire _guard43 = fsm_out == 1'd0;
wire _guard44 = early_reset_check_reads_done0_go_out;
wire _guard45 = _guard43 & _guard44;
wire _guard46 = fsm_out == 1'd0;
wire _guard47 = early_reset_static_par_go_out;
wire _guard48 = _guard46 & _guard47;
wire _guard49 = _guard45 | _guard48;
wire _guard50 = fsm_out == 1'd0;
wire _guard51 = early_reset_static_par0_go_out;
wire _guard52 = _guard50 & _guard51;
wire _guard53 = _guard49 | _guard52;
wire _guard54 = fsm_out != 1'd0;
wire _guard55 = early_reset_static_par_go_out;
wire _guard56 = _guard54 & _guard55;
wire _guard57 = early_reset_check_reads_done0_go_out;
wire _guard58 = early_reset_check_reads_done0_go_out;
wire _guard59 = wrapper_early_reset_static_par0_go_out;
wire _guard60 = invoke2_done_out;
wire _guard61 = ~_guard60;
wire _guard62 = fsm0_out == 3'd2;
wire _guard63 = _guard61 & _guard62;
wire _guard64 = tdcc_go_out;
wire _guard65 = _guard63 & _guard64;
wire _guard66 = early_reset_check_reads_done0_go_out;
wire _guard67 = early_reset_check_reads_done0_go_out;
wire _guard68 = fsm_out == 1'd0;
wire _guard69 = signal_reg_out;
wire _guard70 = _guard68 & _guard69;
wire _guard71 = early_reset_check_reads_done0_go_out;
wire _guard72 = early_reset_check_reads_done0_go_out;
wire _guard73 = do_ar_transfer_go_out;
wire _guard74 = do_ar_transfer_go_out;
wire _guard75 = wrapper_early_reset_static_par_done_out;
wire _guard76 = ~_guard75;
wire _guard77 = fsm0_out == 3'd0;
wire _guard78 = _guard76 & _guard77;
wire _guard79 = tdcc_go_out;
wire _guard80 = _guard78 & _guard79;
wire _guard81 = early_reset_static_par_go_out;
wire _guard82 = early_reset_static_par0_go_out;
wire _guard83 = _guard81 | _guard82;
wire _guard84 = early_reset_static_par_go_out;
wire _guard85 = early_reset_static_par0_go_out;
wire _guard86 = fsm_out == 1'd0;
wire _guard87 = signal_reg_out;
wire _guard88 = _guard86 & _guard87;
wire _guard89 = fsm0_out == 3'd6;
wire _guard90 = fsm0_out == 3'd0;
wire _guard91 = wrapper_early_reset_static_par_done_out;
wire _guard92 = _guard90 & _guard91;
wire _guard93 = tdcc_go_out;
wire _guard94 = _guard92 & _guard93;
wire _guard95 = _guard89 | _guard94;
wire _guard96 = fsm0_out == 3'd1;
wire _guard97 = wrapper_early_reset_check_reads_done0_done_out;
wire _guard98 = comb_reg_out;
wire _guard99 = _guard97 & _guard98;
wire _guard100 = _guard96 & _guard99;
wire _guard101 = tdcc_go_out;
wire _guard102 = _guard100 & _guard101;
wire _guard103 = _guard95 | _guard102;
wire _guard104 = fsm0_out == 3'd5;
wire _guard105 = wrapper_early_reset_check_reads_done0_done_out;
wire _guard106 = comb_reg_out;
wire _guard107 = _guard105 & _guard106;
wire _guard108 = _guard104 & _guard107;
wire _guard109 = tdcc_go_out;
wire _guard110 = _guard108 & _guard109;
wire _guard111 = _guard103 | _guard110;
wire _guard112 = fsm0_out == 3'd2;
wire _guard113 = invoke2_done_out;
wire _guard114 = _guard112 & _guard113;
wire _guard115 = tdcc_go_out;
wire _guard116 = _guard114 & _guard115;
wire _guard117 = _guard111 | _guard116;
wire _guard118 = fsm0_out == 3'd3;
wire _guard119 = do_ar_transfer_done_out;
wire _guard120 = _guard118 & _guard119;
wire _guard121 = tdcc_go_out;
wire _guard122 = _guard120 & _guard121;
wire _guard123 = _guard117 | _guard122;
wire _guard124 = fsm0_out == 3'd4;
wire _guard125 = wrapper_early_reset_static_par0_done_out;
wire _guard126 = _guard124 & _guard125;
wire _guard127 = tdcc_go_out;
wire _guard128 = _guard126 & _guard127;
wire _guard129 = _guard123 | _guard128;
wire _guard130 = fsm0_out == 3'd1;
wire _guard131 = wrapper_early_reset_check_reads_done0_done_out;
wire _guard132 = comb_reg_out;
wire _guard133 = ~_guard132;
wire _guard134 = _guard131 & _guard133;
wire _guard135 = _guard130 & _guard134;
wire _guard136 = tdcc_go_out;
wire _guard137 = _guard135 & _guard136;
wire _guard138 = _guard129 | _guard137;
wire _guard139 = fsm0_out == 3'd5;
wire _guard140 = wrapper_early_reset_check_reads_done0_done_out;
wire _guard141 = comb_reg_out;
wire _guard142 = ~_guard141;
wire _guard143 = _guard140 & _guard142;
wire _guard144 = _guard139 & _guard143;
wire _guard145 = tdcc_go_out;
wire _guard146 = _guard144 & _guard145;
wire _guard147 = _guard138 | _guard146;
wire _guard148 = fsm0_out == 3'd1;
wire _guard149 = wrapper_early_reset_check_reads_done0_done_out;
wire _guard150 = comb_reg_out;
wire _guard151 = ~_guard150;
wire _guard152 = _guard149 & _guard151;
wire _guard153 = _guard148 & _guard152;
wire _guard154 = tdcc_go_out;
wire _guard155 = _guard153 & _guard154;
wire _guard156 = fsm0_out == 3'd5;
wire _guard157 = wrapper_early_reset_check_reads_done0_done_out;
wire _guard158 = comb_reg_out;
wire _guard159 = ~_guard158;
wire _guard160 = _guard157 & _guard159;
wire _guard161 = _guard156 & _guard160;
wire _guard162 = tdcc_go_out;
wire _guard163 = _guard161 & _guard162;
wire _guard164 = _guard155 | _guard163;
wire _guard165 = fsm0_out == 3'd4;
wire _guard166 = wrapper_early_reset_static_par0_done_out;
wire _guard167 = _guard165 & _guard166;
wire _guard168 = tdcc_go_out;
wire _guard169 = _guard167 & _guard168;
wire _guard170 = fsm0_out == 3'd1;
wire _guard171 = wrapper_early_reset_check_reads_done0_done_out;
wire _guard172 = comb_reg_out;
wire _guard173 = _guard171 & _guard172;
wire _guard174 = _guard170 & _guard173;
wire _guard175 = tdcc_go_out;
wire _guard176 = _guard174 & _guard175;
wire _guard177 = fsm0_out == 3'd5;
wire _guard178 = wrapper_early_reset_check_reads_done0_done_out;
wire _guard179 = comb_reg_out;
wire _guard180 = _guard178 & _guard179;
wire _guard181 = _guard177 & _guard180;
wire _guard182 = tdcc_go_out;
wire _guard183 = _guard181 & _guard182;
wire _guard184 = _guard176 | _guard183;
wire _guard185 = fsm0_out == 3'd3;
wire _guard186 = do_ar_transfer_done_out;
wire _guard187 = _guard185 & _guard186;
wire _guard188 = tdcc_go_out;
wire _guard189 = _guard187 & _guard188;
wire _guard190 = fsm0_out == 3'd0;
wire _guard191 = wrapper_early_reset_static_par_done_out;
wire _guard192 = _guard190 & _guard191;
wire _guard193 = tdcc_go_out;
wire _guard194 = _guard192 & _guard193;
wire _guard195 = fsm0_out == 3'd6;
wire _guard196 = fsm0_out == 3'd2;
wire _guard197 = invoke2_done_out;
wire _guard198 = _guard196 & _guard197;
wire _guard199 = tdcc_go_out;
wire _guard200 = _guard198 & _guard199;
wire _guard201 = wrapper_early_reset_check_reads_done0_done_out;
wire _guard202 = ~_guard201;
wire _guard203 = fsm0_out == 3'd1;
wire _guard204 = _guard202 & _guard203;
wire _guard205 = tdcc_go_out;
wire _guard206 = _guard204 & _guard205;
wire _guard207 = wrapper_early_reset_check_reads_done0_done_out;
wire _guard208 = ~_guard207;
wire _guard209 = fsm0_out == 3'd5;
wire _guard210 = _guard208 & _guard209;
wire _guard211 = tdcc_go_out;
wire _guard212 = _guard210 & _guard211;
wire _guard213 = _guard206 | _guard212;
wire _guard214 = do_ar_transfer_go_out;
wire _guard215 = early_reset_static_par0_go_out;
wire _guard216 = _guard214 | _guard215;
wire _guard217 = is_arvalid_out;
wire _guard218 = ARREADY;
wire _guard219 = _guard217 & _guard218;
wire _guard220 = ~_guard219;
wire _guard221 = arvalid_was_high_out;
wire _guard222 = ~_guard221;
wire _guard223 = _guard220 & _guard222;
wire _guard224 = do_ar_transfer_go_out;
wire _guard225 = _guard223 & _guard224;
wire _guard226 = is_arvalid_out;
wire _guard227 = ARREADY;
wire _guard228 = _guard226 & _guard227;
wire _guard229 = arvalid_was_high_out;
wire _guard230 = _guard228 & _guard229;
wire _guard231 = do_ar_transfer_go_out;
wire _guard232 = _guard230 & _guard231;
wire _guard233 = early_reset_static_par0_go_out;
wire _guard234 = _guard232 | _guard233;
wire _guard235 = early_reset_static_par_go_out;
wire _guard236 = early_reset_static_par_go_out;
wire _guard237 = do_ar_transfer_go_out;
wire _guard238 = invoke2_go_out;
wire _guard239 = _guard237 | _guard238;
wire _guard240 = do_ar_transfer_go_out;
wire _guard241 = invoke2_go_out;
wire _guard242 = fsm_out == 1'd0;
wire _guard243 = signal_reg_out;
wire _guard244 = _guard242 & _guard243;
wire _guard245 = fsm_out == 1'd0;
wire _guard246 = signal_reg_out;
wire _guard247 = ~_guard246;
wire _guard248 = _guard245 & _guard247;
wire _guard249 = wrapper_early_reset_static_par_go_out;
wire _guard250 = _guard248 & _guard249;
wire _guard251 = _guard244 | _guard250;
wire _guard252 = fsm_out == 1'd0;
wire _guard253 = signal_reg_out;
wire _guard254 = ~_guard253;
wire _guard255 = _guard252 & _guard254;
wire _guard256 = wrapper_early_reset_check_reads_done0_go_out;
wire _guard257 = _guard255 & _guard256;
wire _guard258 = _guard251 | _guard257;
wire _guard259 = fsm_out == 1'd0;
wire _guard260 = signal_reg_out;
wire _guard261 = ~_guard260;
wire _guard262 = _guard259 & _guard261;
wire _guard263 = wrapper_early_reset_static_par0_go_out;
wire _guard264 = _guard262 & _guard263;
wire _guard265 = _guard258 | _guard264;
wire _guard266 = fsm_out == 1'd0;
wire _guard267 = signal_reg_out;
wire _guard268 = ~_guard267;
wire _guard269 = _guard266 & _guard268;
wire _guard270 = wrapper_early_reset_static_par_go_out;
wire _guard271 = _guard269 & _guard270;
wire _guard272 = fsm_out == 1'd0;
wire _guard273 = signal_reg_out;
wire _guard274 = ~_guard273;
wire _guard275 = _guard272 & _guard274;
wire _guard276 = wrapper_early_reset_check_reads_done0_go_out;
wire _guard277 = _guard275 & _guard276;
wire _guard278 = _guard271 | _guard277;
wire _guard279 = fsm_out == 1'd0;
wire _guard280 = signal_reg_out;
wire _guard281 = ~_guard280;
wire _guard282 = _guard279 & _guard281;
wire _guard283 = wrapper_early_reset_static_par0_go_out;
wire _guard284 = _guard282 & _guard283;
wire _guard285 = _guard278 | _guard284;
wire _guard286 = fsm_out == 1'd0;
wire _guard287 = signal_reg_out;
wire _guard288 = _guard286 & _guard287;
wire _guard289 = early_reset_static_par0_go_out;
wire _guard290 = early_reset_static_par0_go_out;
wire _guard291 = wrapper_early_reset_static_par0_done_out;
wire _guard292 = ~_guard291;
wire _guard293 = fsm0_out == 3'd4;
wire _guard294 = _guard292 & _guard293;
wire _guard295 = tdcc_go_out;
wire _guard296 = _guard294 & _guard295;
wire _guard297 = fsm_out == 1'd0;
wire _guard298 = signal_reg_out;
wire _guard299 = _guard297 & _guard298;
wire _guard300 = fsm0_out == 3'd6;
wire _guard301 = wrapper_early_reset_static_par_go_out;
wire _guard302 = wrapper_early_reset_check_reads_done0_go_out;
wire _guard303 = early_reset_static_par_go_out;
wire _guard304 = early_reset_static_par_go_out;
assign arvalid_was_high_write_en = _guard9;
assign arvalid_was_high_clk = clk;
assign arvalid_was_high_reset = reset;
assign arvalid_was_high_in = 1'd1;
assign adder1_left =
  _guard19 ? fsm_out :
  1'd0;
assign adder1_right = _guard20;
assign do_ar_transfer_go_in = _guard26;
assign done = _guard27;
assign ARSIZE =
  _guard28 ? 3'd2 :
  3'd0;
assign ARLEN =
  _guard29 ? txn_len_out :
  8'd0;
assign ARADDR =
  _guard30 ? base_addr_out :
  64'd0;
assign ARBURST =
  _guard31 ? 2'd1 :
  2'd0;
assign ARVALID = is_arvalid_out;
assign fsm_write_en = _guard36;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard39 ? adder1_out :
  _guard42 ? adder_out :
  _guard53 ? 1'd0 :
  _guard56 ? adder0_out :
  1'd0;
assign adder_left =
  _guard57 ? fsm_out :
  1'd0;
assign adder_right = _guard58;
assign early_reset_static_par0_go_in = _guard59;
assign invoke2_go_in = _guard65;
assign comb_reg_write_en = _guard66;
assign comb_reg_clk = clk;
assign comb_reg_reset = reset;
assign comb_reg_in =
  _guard67 ? perform_reads_out :
  1'd0;
assign wrapper_early_reset_check_reads_done0_done_in = _guard70;
assign perform_reads_left =
  _guard71 ? txn_count_out :
  32'd0;
assign perform_reads_right =
  _guard72 ? txn_n_out :
  32'd0;
assign block_transfer_done_and_left = ARREADY;
assign block_transfer_done_and_right = is_arvalid_out;
assign wrapper_early_reset_static_par_go_in = _guard80;
assign txn_count_write_en = _guard83;
assign txn_count_clk = clk;
assign txn_count_reset = reset;
assign txn_count_in =
  _guard84 ? 32'd0 :
  _guard85 ? txn_adder_out :
  'x;
assign early_reset_static_par0_done_in = ud1_out;
assign wrapper_early_reset_static_par_done_in = _guard88;
assign tdcc_go_in = go;
assign fsm0_write_en = _guard147;
assign fsm0_clk = clk;
assign fsm0_reset = reset;
assign fsm0_in =
  _guard164 ? 3'd6 :
  _guard169 ? 3'd5 :
  _guard184 ? 3'd2 :
  _guard189 ? 3'd4 :
  _guard194 ? 3'd1 :
  _guard195 ? 3'd0 :
  _guard200 ? 3'd3 :
  3'd0;
assign wrapper_early_reset_check_reads_done0_go_in = _guard213;
assign early_reset_check_reads_done0_done_in = ud_out;
assign is_arvalid_write_en = _guard216;
assign is_arvalid_clk = clk;
assign is_arvalid_reset = reset;
assign is_arvalid_in =
  _guard225 ? 1'd1 :
  _guard234 ? 1'd0 :
  'x;
assign adder0_left =
  _guard235 ? fsm_out :
  1'd0;
assign adder0_right = _guard236;
assign early_reset_static_par_done_in = ud0_out;
assign bt_reg_write_en = _guard239;
assign bt_reg_clk = clk;
assign bt_reg_reset = reset;
assign bt_reg_in =
  _guard240 ? block_transfer_done_and_out :
  _guard241 ? 1'd0 :
  'x;
assign signal_reg_write_en = _guard265;
assign signal_reg_clk = clk;
assign signal_reg_reset = reset;
assign signal_reg_in =
  _guard285 ? 1'd1 :
  _guard288 ? 1'd0 :
  1'd0;
assign invoke2_done_in = bt_reg_done;
assign txn_adder_left = txn_count_out;
assign txn_adder_right = 32'd1;
assign wrapper_early_reset_static_par0_go_in = _guard296;
assign wrapper_early_reset_static_par0_done_in = _guard299;
assign tdcc_done_in = _guard300;
assign early_reset_static_par_go_in = _guard301;
assign early_reset_check_reads_done0_go_in = _guard302;
assign txn_len_write_en = _guard303;
assign txn_len_clk = clk;
assign txn_len_reset = reset;
assign txn_len_in = 8'd15;
assign do_ar_transfer_done_in = bt_reg_out;
// COMPONENT END: m_arread_channel
endmodule
module m_read_channel(
  input logic ARESET,
  input logic RVALID,
  input logic RLAST,
  input logic [31:0] RDATA,
  input logic [1:0] RRESP,
  output logic RREADY,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done,
  output logic [63:0] data_received_addr0,
  output logic data_received_write_en,
  output logic [31:0] data_received_write_data,
  output logic data_received_read_en,
  input logic [31:0] data_received_read_data,
  input logic data_received_write_done,
  input logic data_received_read_done,
  output logic [63:0] curr_addr_in,
  output logic curr_addr_write_en,
  input logic [63:0] curr_addr_out,
  input logic curr_addr_done
);
// COMPONENT START: m_read_channel
logic is_rdy_in;
logic is_rdy_write_en;
logic is_rdy_clk;
logic is_rdy_reset;
logic is_rdy_out;
logic is_rdy_done;
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
logic [63:0] curr_addr_adder_left;
logic [63:0] curr_addr_adder_right;
logic [63:0] curr_addr_adder_out;
logic block_transfer_done_and_left;
logic block_transfer_done_and_right;
logic block_transfer_done_and_out;
logic bt_reg_in;
logic bt_reg_write_en;
logic bt_reg_clk;
logic bt_reg_reset;
logic bt_reg_out;
logic bt_reg_done;
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
logic receive_r_transfer_go_in;
logic receive_r_transfer_go_out;
logic receive_r_transfer_done_in;
logic receive_r_transfer_done_out;
logic incr_curr_addr_go_in;
logic incr_curr_addr_go_out;
logic incr_curr_addr_done_in;
logic incr_curr_addr_done_out;
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
std_reg # (
    .WIDTH(1)
) is_rdy (
    .clk(is_rdy_clk),
    .done(is_rdy_done),
    .in(is_rdy_in),
    .out(is_rdy_out),
    .reset(is_rdy_reset),
    .write_en(is_rdy_write_en)
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
std_add # (
    .WIDTH(64)
) curr_addr_adder (
    .left(curr_addr_adder_left),
    .out(curr_addr_adder_out),
    .right(curr_addr_adder_right)
);
std_and # (
    .WIDTH(1)
) block_transfer_done_and (
    .left(block_transfer_done_and_left),
    .out(block_transfer_done_and_out),
    .right(block_transfer_done_and_right)
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
) receive_r_transfer_go (
    .in(receive_r_transfer_go_in),
    .out(receive_r_transfer_go_out)
);
std_wire # (
    .WIDTH(1)
) receive_r_transfer_done (
    .in(receive_r_transfer_done_in),
    .out(receive_r_transfer_done_out)
);
std_wire # (
    .WIDTH(1)
) incr_curr_addr_go (
    .in(incr_curr_addr_go_in),
    .out(incr_curr_addr_go_out)
);
std_wire # (
    .WIDTH(1)
) incr_curr_addr_done (
    .in(incr_curr_addr_done_in),
    .out(incr_curr_addr_done_out)
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
wire _guard2 = incr_curr_addr_go_out;
wire _guard3 = incr_curr_addr_go_out;
wire _guard4 = receive_r_transfer_go_out;
wire _guard5 = receive_r_transfer_go_out;
wire _guard6 = receive_r_transfer_go_out;
wire _guard7 = fsm_out == 3'd5;
wire _guard8 = fsm_out == 3'd0;
wire _guard9 = invoke0_done_out;
wire _guard10 = n_RLAST_out;
wire _guard11 = _guard9 & _guard10;
wire _guard12 = _guard8 & _guard11;
wire _guard13 = tdcc_go_out;
wire _guard14 = _guard12 & _guard13;
wire _guard15 = _guard7 | _guard14;
wire _guard16 = fsm_out == 3'd4;
wire _guard17 = incr_curr_addr_done_out;
wire _guard18 = n_RLAST_out;
wire _guard19 = _guard17 & _guard18;
wire _guard20 = _guard16 & _guard19;
wire _guard21 = tdcc_go_out;
wire _guard22 = _guard20 & _guard21;
wire _guard23 = _guard15 | _guard22;
wire _guard24 = fsm_out == 3'd1;
wire _guard25 = invoke1_done_out;
wire _guard26 = _guard24 & _guard25;
wire _guard27 = tdcc_go_out;
wire _guard28 = _guard26 & _guard27;
wire _guard29 = _guard23 | _guard28;
wire _guard30 = fsm_out == 3'd2;
wire _guard31 = block_transfer_done_out;
wire _guard32 = _guard30 & _guard31;
wire _guard33 = tdcc_go_out;
wire _guard34 = _guard32 & _guard33;
wire _guard35 = _guard29 | _guard34;
wire _guard36 = fsm_out == 3'd3;
wire _guard37 = receive_r_transfer_done_out;
wire _guard38 = _guard36 & _guard37;
wire _guard39 = tdcc_go_out;
wire _guard40 = _guard38 & _guard39;
wire _guard41 = _guard35 | _guard40;
wire _guard42 = fsm_out == 3'd0;
wire _guard43 = invoke0_done_out;
wire _guard44 = n_RLAST_out;
wire _guard45 = ~_guard44;
wire _guard46 = _guard43 & _guard45;
wire _guard47 = _guard42 & _guard46;
wire _guard48 = tdcc_go_out;
wire _guard49 = _guard47 & _guard48;
wire _guard50 = _guard41 | _guard49;
wire _guard51 = fsm_out == 3'd4;
wire _guard52 = incr_curr_addr_done_out;
wire _guard53 = n_RLAST_out;
wire _guard54 = ~_guard53;
wire _guard55 = _guard52 & _guard54;
wire _guard56 = _guard51 & _guard55;
wire _guard57 = tdcc_go_out;
wire _guard58 = _guard56 & _guard57;
wire _guard59 = _guard50 | _guard58;
wire _guard60 = fsm_out == 3'd0;
wire _guard61 = invoke0_done_out;
wire _guard62 = n_RLAST_out;
wire _guard63 = ~_guard62;
wire _guard64 = _guard61 & _guard63;
wire _guard65 = _guard60 & _guard64;
wire _guard66 = tdcc_go_out;
wire _guard67 = _guard65 & _guard66;
wire _guard68 = fsm_out == 3'd4;
wire _guard69 = incr_curr_addr_done_out;
wire _guard70 = n_RLAST_out;
wire _guard71 = ~_guard70;
wire _guard72 = _guard69 & _guard71;
wire _guard73 = _guard68 & _guard72;
wire _guard74 = tdcc_go_out;
wire _guard75 = _guard73 & _guard74;
wire _guard76 = _guard67 | _guard75;
wire _guard77 = fsm_out == 3'd1;
wire _guard78 = invoke1_done_out;
wire _guard79 = _guard77 & _guard78;
wire _guard80 = tdcc_go_out;
wire _guard81 = _guard79 & _guard80;
wire _guard82 = fsm_out == 3'd3;
wire _guard83 = receive_r_transfer_done_out;
wire _guard84 = _guard82 & _guard83;
wire _guard85 = tdcc_go_out;
wire _guard86 = _guard84 & _guard85;
wire _guard87 = fsm_out == 3'd0;
wire _guard88 = invoke0_done_out;
wire _guard89 = n_RLAST_out;
wire _guard90 = _guard88 & _guard89;
wire _guard91 = _guard87 & _guard90;
wire _guard92 = tdcc_go_out;
wire _guard93 = _guard91 & _guard92;
wire _guard94 = fsm_out == 3'd4;
wire _guard95 = incr_curr_addr_done_out;
wire _guard96 = n_RLAST_out;
wire _guard97 = _guard95 & _guard96;
wire _guard98 = _guard94 & _guard97;
wire _guard99 = tdcc_go_out;
wire _guard100 = _guard98 & _guard99;
wire _guard101 = _guard93 | _guard100;
wire _guard102 = fsm_out == 3'd5;
wire _guard103 = fsm_out == 3'd2;
wire _guard104 = block_transfer_done_out;
wire _guard105 = _guard103 & _guard104;
wire _guard106 = tdcc_go_out;
wire _guard107 = _guard105 & _guard106;
wire _guard108 = block_transfer_go_out;
wire _guard109 = block_transfer_go_out;
wire _guard110 = block_transfer_go_out;
wire _guard111 = block_transfer_go_out;
wire _guard112 = invoke0_done_out;
wire _guard113 = ~_guard112;
wire _guard114 = fsm_out == 3'd0;
wire _guard115 = _guard113 & _guard114;
wire _guard116 = tdcc_go_out;
wire _guard117 = _guard115 & _guard116;
wire _guard118 = block_transfer_go_out;
wire _guard119 = receive_r_transfer_go_out;
wire _guard120 = _guard118 | _guard119;
wire _guard121 = RVALID;
wire _guard122 = is_rdy_out;
wire _guard123 = _guard121 & _guard122;
wire _guard124 = ~_guard123;
wire _guard125 = block_transfer_go_out;
wire _guard126 = _guard124 & _guard125;
wire _guard127 = RVALID;
wire _guard128 = is_rdy_out;
wire _guard129 = _guard127 & _guard128;
wire _guard130 = block_transfer_go_out;
wire _guard131 = _guard129 & _guard130;
wire _guard132 = receive_r_transfer_go_out;
wire _guard133 = _guard131 | _guard132;
wire _guard134 = block_transfer_go_out;
wire _guard135 = invoke0_go_out;
wire _guard136 = _guard134 | _guard135;
wire _guard137 = RLAST;
wire _guard138 = ~_guard137;
wire _guard139 = block_transfer_go_out;
wire _guard140 = _guard138 & _guard139;
wire _guard141 = invoke0_go_out;
wire _guard142 = _guard140 | _guard141;
wire _guard143 = RLAST;
wire _guard144 = block_transfer_go_out;
wire _guard145 = _guard143 & _guard144;
wire _guard146 = invoke1_done_out;
wire _guard147 = ~_guard146;
wire _guard148 = fsm_out == 3'd1;
wire _guard149 = _guard147 & _guard148;
wire _guard150 = tdcc_go_out;
wire _guard151 = _guard149 & _guard150;
wire _guard152 = block_transfer_go_out;
wire _guard153 = invoke1_go_out;
wire _guard154 = _guard152 | _guard153;
wire _guard155 = block_transfer_go_out;
wire _guard156 = invoke1_go_out;
wire _guard157 = fsm_out == 3'd5;
wire _guard158 = incr_curr_addr_done_out;
wire _guard159 = ~_guard158;
wire _guard160 = fsm_out == 3'd4;
wire _guard161 = _guard159 & _guard160;
wire _guard162 = tdcc_go_out;
wire _guard163 = _guard161 & _guard162;
wire _guard164 = receive_r_transfer_done_out;
wire _guard165 = ~_guard164;
wire _guard166 = fsm_out == 3'd3;
wire _guard167 = _guard165 & _guard166;
wire _guard168 = tdcc_go_out;
wire _guard169 = _guard167 & _guard168;
wire _guard170 = block_transfer_done_out;
wire _guard171 = ~_guard170;
wire _guard172 = fsm_out == 3'd2;
wire _guard173 = _guard171 & _guard172;
wire _guard174 = tdcc_go_out;
wire _guard175 = _guard173 & _guard174;
wire _guard176 = incr_curr_addr_go_out;
wire _guard177 = incr_curr_addr_go_out;
assign done = _guard1;
assign curr_addr_write_en = _guard2;
assign data_received_read_en = 1'd0;
assign curr_addr_in =
  _guard3 ? curr_addr_adder_out :
  64'd0;
assign RREADY = is_rdy_out;
assign data_received_write_en = _guard4;
assign data_received_write_data =
  _guard5 ? read_data_reg_out :
  32'd0;
assign data_received_addr0 =
  _guard6 ? curr_addr_out :
  64'd0;
assign fsm_write_en = _guard59;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard76 ? 3'd5 :
  _guard81 ? 3'd2 :
  _guard86 ? 3'd4 :
  _guard101 ? 3'd1 :
  _guard102 ? 3'd0 :
  _guard107 ? 3'd3 :
  3'd0;
assign block_transfer_done_in = bt_reg_out;
assign read_data_reg_write_en =
  _guard108 ? is_rdy_out :
  1'd0;
assign read_data_reg_clk = clk;
assign read_data_reg_reset = reset;
assign read_data_reg_in = RDATA;
assign block_transfer_done_and_left = is_rdy_out;
assign block_transfer_done_and_right = RVALID;
assign tdcc_go_in = go;
assign invoke0_go_in = _guard117;
assign is_rdy_write_en = _guard120;
assign is_rdy_clk = clk;
assign is_rdy_reset = reset;
assign is_rdy_in =
  _guard126 ? 1'd1 :
  _guard133 ? 1'd0 :
  'x;
assign n_RLAST_write_en = _guard136;
assign n_RLAST_clk = clk;
assign n_RLAST_reset = reset;
assign n_RLAST_in =
  _guard142 ? 1'd1 :
  _guard145 ? 1'd0 :
  'x;
assign invoke0_done_in = n_RLAST_done;
assign invoke1_go_in = _guard151;
assign bt_reg_write_en = _guard154;
assign bt_reg_clk = clk;
assign bt_reg_reset = reset;
assign bt_reg_in =
  _guard155 ? block_transfer_done_and_out :
  _guard156 ? 1'd0 :
  'x;
assign tdcc_done_in = _guard157;
assign incr_curr_addr_go_in = _guard163;
assign receive_r_transfer_go_in = _guard169;
assign incr_curr_addr_done_in = curr_addr_done;
assign block_transfer_go_in = _guard175;
assign invoke1_done_in = bt_reg_done;
assign curr_addr_adder_left = 64'd1;
assign curr_addr_adder_right = curr_addr_out;
assign receive_r_transfer_done_in = data_received_write_done;
// COMPONENT END: m_read_channel
endmodule
module main(
  input logic m_ARESET,
  input logic m_ARREADY,
  input logic m_RVALID,
  input logic m_RLAST,
  input logic [31:0] m_RDATA,
  input logic [1:0] m_RRESP,
  input logic m_RID,
  output logic m_ARVALID,
  output logic [63:0] m_ARADDR,
  output logic [2:0] m_ARSIZE,
  output logic [7:0] m_ARLEN,
  output logic [1:0] m_ARBURST,
  output logic m_RREADY,
  output logic m_ARID,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: main
logic vec1_data_clk;
logic vec1_data_reset;
logic [63:0] vec1_data_addr0;
logic vec1_data_write_en;
logic [31:0] vec1_data_write_data;
logic vec1_data_read_en;
logic [31:0] vec1_data_read_data;
logic vec1_data_write_done;
logic vec1_data_read_done;
logic [63:0] curr_addr_in;
logic curr_addr_write_en;
logic curr_addr_clk;
logic curr_addr_reset;
logic [63:0] curr_addr_out;
logic curr_addr_done;
logic [63:0] base_addr_in;
logic base_addr_write_en;
logic base_addr_clk;
logic base_addr_reset;
logic [63:0] base_addr_out;
logic base_addr_done;
logic read_channel_ARESET;
logic read_channel_RVALID;
logic read_channel_RLAST;
logic [31:0] read_channel_RDATA;
logic [1:0] read_channel_RRESP;
logic read_channel_RREADY;
logic read_channel_go;
logic read_channel_clk;
logic read_channel_reset;
logic read_channel_done;
logic read_channel_data_received_read_done;
logic read_channel_curr_addr_write_en;
logic read_channel_curr_addr_done;
logic [31:0] read_channel_data_received_write_data;
logic [63:0] read_channel_data_received_addr0;
logic [63:0] read_channel_curr_addr_in;
logic [63:0] read_channel_curr_addr_out;
logic [31:0] read_channel_data_received_read_data;
logic read_channel_data_received_read_en;
logic read_channel_data_received_write_done;
logic read_channel_data_received_write_en;
logic arread_channel_ARESET;
logic arread_channel_ARREADY;
logic arread_channel_ARVALID;
logic [63:0] arread_channel_ARADDR;
logic [2:0] arread_channel_ARSIZE;
logic [7:0] arread_channel_ARLEN;
logic [1:0] arread_channel_ARBURST;
logic arread_channel_go;
logic arread_channel_clk;
logic arread_channel_reset;
logic arread_channel_done;
logic [63:0] arread_channel_base_addr_in;
logic [63:0] arread_channel_base_addr_out;
logic arread_channel_base_addr_write_en;
logic arread_channel_base_addr_done;
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
seq_mem_d1 # (
    .IDX_SIZE(64),
    .SIZE(16),
    .WIDTH(32)
) vec1_data (
    .addr0(vec1_data_addr0),
    .clk(vec1_data_clk),
    .read_data(vec1_data_read_data),
    .read_done(vec1_data_read_done),
    .read_en(vec1_data_read_en),
    .reset(vec1_data_reset),
    .write_data(vec1_data_write_data),
    .write_done(vec1_data_write_done),
    .write_en(vec1_data_write_en)
);
std_reg # (
    .WIDTH(64)
) curr_addr (
    .clk(curr_addr_clk),
    .done(curr_addr_done),
    .in(curr_addr_in),
    .out(curr_addr_out),
    .reset(curr_addr_reset),
    .write_en(curr_addr_write_en)
);
std_reg # (
    .WIDTH(64)
) base_addr (
    .clk(base_addr_clk),
    .done(base_addr_done),
    .in(base_addr_in),
    .out(base_addr_out),
    .reset(base_addr_reset),
    .write_en(base_addr_write_en)
);
m_read_channel read_channel (
    .ARESET(read_channel_ARESET),
    .RDATA(read_channel_RDATA),
    .RLAST(read_channel_RLAST),
    .RREADY(read_channel_RREADY),
    .RRESP(read_channel_RRESP),
    .RVALID(read_channel_RVALID),
    .clk(read_channel_clk),
    .curr_addr_done(read_channel_curr_addr_done),
    .curr_addr_in(read_channel_curr_addr_in),
    .curr_addr_out(read_channel_curr_addr_out),
    .curr_addr_write_en(read_channel_curr_addr_write_en),
    .data_received_addr0(read_channel_data_received_addr0),
    .data_received_read_data(read_channel_data_received_read_data),
    .data_received_read_done(read_channel_data_received_read_done),
    .data_received_read_en(read_channel_data_received_read_en),
    .data_received_write_data(read_channel_data_received_write_data),
    .data_received_write_done(read_channel_data_received_write_done),
    .data_received_write_en(read_channel_data_received_write_en),
    .done(read_channel_done),
    .go(read_channel_go),
    .reset(read_channel_reset)
);
m_arread_channel arread_channel (
    .ARADDR(arread_channel_ARADDR),
    .ARBURST(arread_channel_ARBURST),
    .ARESET(arread_channel_ARESET),
    .ARLEN(arread_channel_ARLEN),
    .ARREADY(arread_channel_ARREADY),
    .ARSIZE(arread_channel_ARSIZE),
    .ARVALID(arread_channel_ARVALID),
    .base_addr_done(arread_channel_base_addr_done),
    .base_addr_in(arread_channel_base_addr_in),
    .base_addr_out(arread_channel_base_addr_out),
    .base_addr_write_en(arread_channel_base_addr_write_en),
    .clk(arread_channel_clk),
    .done(arread_channel_done),
    .go(arread_channel_go),
    .reset(arread_channel_reset)
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
wire _guard4 = invoke2_go_out;
wire _guard5 = invoke0_go_out;
wire _guard6 = invoke0_go_out;
wire _guard7 = invoke0_go_out;
wire _guard8 = fsm_out == 2'd3;
wire _guard9 = fsm_out == 2'd0;
wire _guard10 = invoke0_done_out;
wire _guard11 = _guard9 & _guard10;
wire _guard12 = tdcc_go_out;
wire _guard13 = _guard11 & _guard12;
wire _guard14 = _guard8 | _guard13;
wire _guard15 = fsm_out == 2'd1;
wire _guard16 = invoke1_done_out;
wire _guard17 = _guard15 & _guard16;
wire _guard18 = tdcc_go_out;
wire _guard19 = _guard17 & _guard18;
wire _guard20 = _guard14 | _guard19;
wire _guard21 = fsm_out == 2'd2;
wire _guard22 = invoke2_done_out;
wire _guard23 = _guard21 & _guard22;
wire _guard24 = tdcc_go_out;
wire _guard25 = _guard23 & _guard24;
wire _guard26 = _guard20 | _guard25;
wire _guard27 = fsm_out == 2'd0;
wire _guard28 = invoke0_done_out;
wire _guard29 = _guard27 & _guard28;
wire _guard30 = tdcc_go_out;
wire _guard31 = _guard29 & _guard30;
wire _guard32 = fsm_out == 2'd3;
wire _guard33 = fsm_out == 2'd2;
wire _guard34 = invoke2_done_out;
wire _guard35 = _guard33 & _guard34;
wire _guard36 = tdcc_go_out;
wire _guard37 = _guard35 & _guard36;
wire _guard38 = fsm_out == 2'd1;
wire _guard39 = invoke1_done_out;
wire _guard40 = _guard38 & _guard39;
wire _guard41 = tdcc_go_out;
wire _guard42 = _guard40 & _guard41;
wire _guard43 = invoke2_go_out;
wire _guard44 = invoke2_go_out;
wire _guard45 = invoke2_go_out;
wire _guard46 = invoke2_go_out;
wire _guard47 = invoke2_done_out;
wire _guard48 = ~_guard47;
wire _guard49 = fsm_out == 2'd2;
wire _guard50 = _guard48 & _guard49;
wire _guard51 = tdcc_go_out;
wire _guard52 = _guard50 & _guard51;
wire _guard53 = invoke0_go_out;
wire _guard54 = invoke0_go_out;
wire _guard55 = invoke1_go_out;
wire _guard56 = invoke2_go_out;
wire _guard57 = invoke1_go_out;
wire _guard58 = invoke2_go_out;
wire _guard59 = invoke0_done_out;
wire _guard60 = ~_guard59;
wire _guard61 = fsm_out == 2'd0;
wire _guard62 = _guard60 & _guard61;
wire _guard63 = tdcc_go_out;
wire _guard64 = _guard62 & _guard63;
wire _guard65 = invoke2_go_out;
wire _guard66 = invoke2_go_out;
wire _guard67 = invoke2_go_out;
wire _guard68 = invoke2_go_out;
wire _guard69 = invoke2_go_out;
wire _guard70 = invoke2_go_out;
wire _guard71 = invoke2_go_out;
wire _guard72 = invoke2_go_out;
wire _guard73 = invoke2_go_out;
wire _guard74 = invoke2_go_out;
wire _guard75 = invoke2_go_out;
wire _guard76 = invoke1_done_out;
wire _guard77 = ~_guard76;
wire _guard78 = fsm_out == 2'd1;
wire _guard79 = _guard77 & _guard78;
wire _guard80 = tdcc_go_out;
wire _guard81 = _guard79 & _guard80;
wire _guard82 = fsm_out == 2'd3;
wire _guard83 = invoke0_go_out;
wire _guard84 = invoke0_go_out;
wire _guard85 = invoke0_go_out;
wire _guard86 = invoke0_go_out;
wire _guard87 = invoke0_go_out;
assign done = _guard1;
assign m_ARSIZE =
  _guard2 ? arread_channel_ARSIZE :
  3'd0;
assign m_ARVALID =
  _guard3 ? arread_channel_ARVALID :
  1'd0;
assign m_RREADY =
  _guard4 ? read_channel_RREADY :
  1'd0;
assign m_ARLEN =
  _guard5 ? arread_channel_ARLEN :
  8'd0;
assign m_ARID = 1'd0;
assign m_ARBURST =
  _guard6 ? arread_channel_ARBURST :
  2'd0;
assign m_ARADDR =
  _guard7 ? arread_channel_ARADDR :
  64'd0;
assign fsm_write_en = _guard26;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard31 ? 2'd1 :
  _guard32 ? 2'd0 :
  _guard37 ? 2'd3 :
  _guard42 ? 2'd2 :
  2'd0;
assign vec1_data_write_en =
  _guard43 ? read_channel_data_received_write_en :
  1'd0;
assign vec1_data_read_en =
  _guard44 ? read_channel_data_received_read_en :
  1'd0;
assign vec1_data_clk = clk;
assign vec1_data_addr0 = read_channel_data_received_addr0;
assign vec1_data_reset = reset;
assign vec1_data_write_data = read_channel_data_received_write_data;
assign invoke2_go_in = _guard52;
assign base_addr_write_en =
  _guard53 ? arread_channel_base_addr_write_en :
  1'd0;
assign base_addr_clk = clk;
assign base_addr_reset = reset;
assign base_addr_in = arread_channel_base_addr_in;
assign curr_addr_write_en =
  _guard55 ? 1'd1 :
  _guard56 ? read_channel_curr_addr_write_en :
  1'd0;
assign curr_addr_clk = clk;
assign curr_addr_reset = reset;
assign curr_addr_in =
  _guard57 ? base_addr_out :
  _guard58 ? read_channel_curr_addr_in :
  'x;
assign tdcc_go_in = go;
assign invoke0_go_in = _guard64;
assign read_channel_RVALID =
  _guard65 ? m_RVALID :
  1'd0;
assign read_channel_RLAST =
  _guard66 ? m_RLAST :
  1'd0;
assign read_channel_data_received_read_done =
  _guard67 ? vec1_data_read_done :
  1'd0;
assign read_channel_curr_addr_done =
  _guard68 ? curr_addr_done :
  1'd0;
assign read_channel_data_received_write_done =
  _guard69 ? vec1_data_write_done :
  1'd0;
assign read_channel_RDATA =
  _guard70 ? m_RDATA :
  32'd0;
assign read_channel_clk = clk;
assign read_channel_curr_addr_out =
  _guard71 ? curr_addr_out :
  64'd0;
assign read_channel_data_received_read_data =
  _guard72 ? vec1_data_read_data :
  32'd0;
assign read_channel_reset = reset;
assign read_channel_go = _guard73;
assign read_channel_ARESET =
  _guard74 ? m_ARESET :
  1'd0;
assign read_channel_RRESP =
  _guard75 ? m_RRESP :
  2'd0;
assign invoke0_done_in = arread_channel_done;
assign invoke1_go_in = _guard81;
assign invoke2_done_in = read_channel_done;
assign tdcc_done_in = _guard82;
assign arread_channel_base_addr_done =
  _guard83 ? base_addr_done :
  1'd0;
assign arread_channel_clk = clk;
assign arread_channel_reset = reset;
assign arread_channel_go = _guard84;
assign arread_channel_ARESET =
  _guard85 ? m_ARESET :
  1'd0;
assign arread_channel_base_addr_out =
  _guard86 ? base_addr_out :
  64'd0;
assign arread_channel_ARREADY =
  _guard87 ? m_ARREADY :
  1'd0;
assign invoke1_done_in = curr_addr_done;

`ifdef COCOTB_SIM
  initial begin
    $dumpfile ("out.vcd");
    $dumpvars (0, main);
    #1;
  end
`endif
// COMPONENT END: main
        
endmodule
module pow(
  input logic [31:0] base,
  input logic [31:0] exp,
  output logic [31:0] out,
  input logic go,
  input logic clk,
  input logic reset,
  output logic done
);
// COMPONENT START: pow
logic [31:0] t_in;
logic t_write_en;
logic t_clk;
logic t_reset;
logic [31:0] t_out;
logic t_done;
logic [31:0] count_in;
logic count_write_en;
logic count_clk;
logic count_reset;
logic [31:0] count_out;
logic count_done;
logic mul_clk;
logic mul_reset;
logic mul_go;
logic [31:0] mul_left;
logic [31:0] mul_right;
logic [31:0] mul_out;
logic mul_done;
logic [31:0] lt_left;
logic [31:0] lt_right;
logic lt_out;
logic [31:0] incr_left;
logic [31:0] incr_right;
logic [31:0] incr_out;
logic comb_reg_in;
logic comb_reg_write_en;
logic comb_reg_clk;
logic comb_reg_reset;
logic comb_reg_out;
logic comb_reg_done;
logic [2:0] fsm_in;
logic fsm_write_en;
logic fsm_clk;
logic fsm_reset;
logic [2:0] fsm_out;
logic fsm_done;
logic ud_out;
logic [2:0] adder_left;
logic [2:0] adder_right;
logic [2:0] adder_out;
logic ud0_out;
logic [2:0] adder0_left;
logic [2:0] adder0_right;
logic [2:0] adder0_out;
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
logic [1:0] fsm0_in;
logic fsm0_write_en;
logic fsm0_clk;
logic fsm0_reset;
logic [1:0] fsm0_out;
logic fsm0_done;
logic early_reset_cond0_go_in;
logic early_reset_cond0_go_out;
logic early_reset_cond0_done_in;
logic early_reset_cond0_done_out;
logic early_reset_init0_go_in;
logic early_reset_init0_go_out;
logic early_reset_init0_done_in;
logic early_reset_init0_done_out;
logic early_reset_static_seq_go_in;
logic early_reset_static_seq_go_out;
logic early_reset_static_seq_done_in;
logic early_reset_static_seq_done_out;
logic wrapper_early_reset_init0_go_in;
logic wrapper_early_reset_init0_go_out;
logic wrapper_early_reset_init0_done_in;
logic wrapper_early_reset_init0_done_out;
logic wrapper_early_reset_cond0_go_in;
logic wrapper_early_reset_cond0_go_out;
logic wrapper_early_reset_cond0_done_in;
logic wrapper_early_reset_cond0_done_out;
logic while_wrapper_early_reset_static_seq_go_in;
logic while_wrapper_early_reset_static_seq_go_out;
logic while_wrapper_early_reset_static_seq_done_in;
logic while_wrapper_early_reset_static_seq_done_out;
logic tdcc_go_in;
logic tdcc_go_out;
logic tdcc_done_in;
logic tdcc_done_out;
std_reg # (
    .WIDTH(32)
) t (
    .clk(t_clk),
    .done(t_done),
    .in(t_in),
    .out(t_out),
    .reset(t_reset),
    .write_en(t_write_en)
);
std_reg # (
    .WIDTH(32)
) count (
    .clk(count_clk),
    .done(count_done),
    .in(count_in),
    .out(count_out),
    .reset(count_reset),
    .write_en(count_write_en)
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
std_lt # (
    .WIDTH(32)
) lt (
    .left(lt_left),
    .out(lt_out),
    .right(lt_right)
);
std_add # (
    .WIDTH(32)
) incr (
    .left(incr_left),
    .out(incr_out),
    .right(incr_right)
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
    .WIDTH(3)
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
    .WIDTH(3)
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
    .WIDTH(3)
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
) early_reset_cond0_go (
    .in(early_reset_cond0_go_in),
    .out(early_reset_cond0_go_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_cond0_done (
    .in(early_reset_cond0_done_in),
    .out(early_reset_cond0_done_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_init0_go (
    .in(early_reset_init0_go_in),
    .out(early_reset_init0_go_out)
);
std_wire # (
    .WIDTH(1)
) early_reset_init0_done (
    .in(early_reset_init0_done_in),
    .out(early_reset_init0_done_out)
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
) wrapper_early_reset_init0_go (
    .in(wrapper_early_reset_init0_go_in),
    .out(wrapper_early_reset_init0_go_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_init0_done (
    .in(wrapper_early_reset_init0_done_in),
    .out(wrapper_early_reset_init0_done_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_cond0_go (
    .in(wrapper_early_reset_cond0_go_in),
    .out(wrapper_early_reset_cond0_go_out)
);
std_wire # (
    .WIDTH(1)
) wrapper_early_reset_cond0_done (
    .in(wrapper_early_reset_cond0_done_in),
    .out(wrapper_early_reset_cond0_done_out)
);
std_wire # (
    .WIDTH(1)
) while_wrapper_early_reset_static_seq_go (
    .in(while_wrapper_early_reset_static_seq_go_in),
    .out(while_wrapper_early_reset_static_seq_go_out)
);
std_wire # (
    .WIDTH(1)
) while_wrapper_early_reset_static_seq_done (
    .in(while_wrapper_early_reset_static_seq_done_in),
    .out(while_wrapper_early_reset_static_seq_done_out)
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
wire _guard1 = early_reset_static_seq_go_out;
wire _guard2 = early_reset_static_seq_go_out;
wire _guard3 = tdcc_done_out;
wire _guard4 = early_reset_cond0_go_out;
wire _guard5 = early_reset_init0_go_out;
wire _guard6 = _guard4 | _guard5;
wire _guard7 = early_reset_static_seq_go_out;
wire _guard8 = _guard6 | _guard7;
wire _guard9 = fsm_out != 3'd4;
wire _guard10 = early_reset_static_seq_go_out;
wire _guard11 = _guard9 & _guard10;
wire _guard12 = fsm_out != 3'd0;
wire _guard13 = early_reset_cond0_go_out;
wire _guard14 = _guard12 & _guard13;
wire _guard15 = fsm_out != 3'd0;
wire _guard16 = early_reset_init0_go_out;
wire _guard17 = _guard15 & _guard16;
wire _guard18 = fsm_out == 3'd0;
wire _guard19 = early_reset_cond0_go_out;
wire _guard20 = _guard18 & _guard19;
wire _guard21 = fsm_out == 3'd0;
wire _guard22 = early_reset_init0_go_out;
wire _guard23 = _guard21 & _guard22;
wire _guard24 = _guard20 | _guard23;
wire _guard25 = fsm_out == 3'd4;
wire _guard26 = early_reset_static_seq_go_out;
wire _guard27 = _guard25 & _guard26;
wire _guard28 = _guard24 | _guard27;
wire _guard29 = early_reset_cond0_go_out;
wire _guard30 = early_reset_cond0_go_out;
wire _guard31 = wrapper_early_reset_init0_go_out;
wire _guard32 = early_reset_cond0_go_out;
wire _guard33 = fsm_out == 3'd4;
wire _guard34 = early_reset_static_seq_go_out;
wire _guard35 = _guard33 & _guard34;
wire _guard36 = _guard32 | _guard35;
wire _guard37 = early_reset_cond0_go_out;
wire _guard38 = fsm_out == 3'd4;
wire _guard39 = early_reset_static_seq_go_out;
wire _guard40 = _guard38 & _guard39;
wire _guard41 = _guard37 | _guard40;
wire _guard42 = wrapper_early_reset_init0_done_out;
wire _guard43 = ~_guard42;
wire _guard44 = fsm0_out == 2'd0;
wire _guard45 = _guard43 & _guard44;
wire _guard46 = tdcc_go_out;
wire _guard47 = _guard45 & _guard46;
wire _guard48 = early_reset_init0_go_out;
wire _guard49 = fsm_out == 3'd3;
wire _guard50 = early_reset_static_seq_go_out;
wire _guard51 = _guard49 & _guard50;
wire _guard52 = _guard48 | _guard51;
wire _guard53 = early_reset_init0_go_out;
wire _guard54 = fsm_out == 3'd3;
wire _guard55 = early_reset_static_seq_go_out;
wire _guard56 = _guard54 & _guard55;
wire _guard57 = wrapper_early_reset_cond0_go_out;
wire _guard58 = fsm_out == 3'd0;
wire _guard59 = signal_reg_out;
wire _guard60 = _guard58 & _guard59;
wire _guard61 = while_wrapper_early_reset_static_seq_done_out;
wire _guard62 = ~_guard61;
wire _guard63 = fsm0_out == 2'd2;
wire _guard64 = _guard62 & _guard63;
wire _guard65 = tdcc_go_out;
wire _guard66 = _guard64 & _guard65;
wire _guard67 = fsm0_out == 2'd3;
wire _guard68 = fsm0_out == 2'd0;
wire _guard69 = wrapper_early_reset_init0_done_out;
wire _guard70 = _guard68 & _guard69;
wire _guard71 = tdcc_go_out;
wire _guard72 = _guard70 & _guard71;
wire _guard73 = _guard67 | _guard72;
wire _guard74 = fsm0_out == 2'd1;
wire _guard75 = wrapper_early_reset_cond0_done_out;
wire _guard76 = _guard74 & _guard75;
wire _guard77 = tdcc_go_out;
wire _guard78 = _guard76 & _guard77;
wire _guard79 = _guard73 | _guard78;
wire _guard80 = fsm0_out == 2'd2;
wire _guard81 = while_wrapper_early_reset_static_seq_done_out;
wire _guard82 = _guard80 & _guard81;
wire _guard83 = tdcc_go_out;
wire _guard84 = _guard82 & _guard83;
wire _guard85 = _guard79 | _guard84;
wire _guard86 = fsm0_out == 2'd0;
wire _guard87 = wrapper_early_reset_init0_done_out;
wire _guard88 = _guard86 & _guard87;
wire _guard89 = tdcc_go_out;
wire _guard90 = _guard88 & _guard89;
wire _guard91 = fsm0_out == 2'd3;
wire _guard92 = fsm0_out == 2'd2;
wire _guard93 = while_wrapper_early_reset_static_seq_done_out;
wire _guard94 = _guard92 & _guard93;
wire _guard95 = tdcc_go_out;
wire _guard96 = _guard94 & _guard95;
wire _guard97 = fsm0_out == 2'd1;
wire _guard98 = wrapper_early_reset_cond0_done_out;
wire _guard99 = _guard97 & _guard98;
wire _guard100 = tdcc_go_out;
wire _guard101 = _guard99 & _guard100;
wire _guard102 = early_reset_init0_go_out;
wire _guard103 = fsm_out == 3'd0;
wire _guard104 = early_reset_static_seq_go_out;
wire _guard105 = _guard103 & _guard104;
wire _guard106 = _guard102 | _guard105;
wire _guard107 = early_reset_init0_go_out;
wire _guard108 = fsm_out == 3'd0;
wire _guard109 = early_reset_static_seq_go_out;
wire _guard110 = _guard108 & _guard109;
wire _guard111 = fsm_out == 3'd0;
wire _guard112 = early_reset_static_seq_go_out;
wire _guard113 = _guard111 & _guard112;
wire _guard114 = fsm_out == 3'd0;
wire _guard115 = early_reset_static_seq_go_out;
wire _guard116 = _guard114 & _guard115;
wire _guard117 = early_reset_init0_go_out;
wire _guard118 = early_reset_init0_go_out;
wire _guard119 = while_wrapper_early_reset_static_seq_go_out;
wire _guard120 = fsm_out == 3'd0;
wire _guard121 = signal_reg_out;
wire _guard122 = _guard120 & _guard121;
wire _guard123 = fsm_out == 3'd0;
wire _guard124 = signal_reg_out;
wire _guard125 = _guard123 & _guard124;
wire _guard126 = fsm_out == 3'd0;
wire _guard127 = signal_reg_out;
wire _guard128 = ~_guard127;
wire _guard129 = _guard126 & _guard128;
wire _guard130 = wrapper_early_reset_init0_go_out;
wire _guard131 = _guard129 & _guard130;
wire _guard132 = _guard125 | _guard131;
wire _guard133 = fsm_out == 3'd0;
wire _guard134 = signal_reg_out;
wire _guard135 = ~_guard134;
wire _guard136 = _guard133 & _guard135;
wire _guard137 = wrapper_early_reset_cond0_go_out;
wire _guard138 = _guard136 & _guard137;
wire _guard139 = _guard132 | _guard138;
wire _guard140 = fsm_out == 3'd0;
wire _guard141 = signal_reg_out;
wire _guard142 = ~_guard141;
wire _guard143 = _guard140 & _guard142;
wire _guard144 = wrapper_early_reset_init0_go_out;
wire _guard145 = _guard143 & _guard144;
wire _guard146 = fsm_out == 3'd0;
wire _guard147 = signal_reg_out;
wire _guard148 = ~_guard147;
wire _guard149 = _guard146 & _guard148;
wire _guard150 = wrapper_early_reset_cond0_go_out;
wire _guard151 = _guard149 & _guard150;
wire _guard152 = _guard145 | _guard151;
wire _guard153 = fsm_out == 3'd0;
wire _guard154 = signal_reg_out;
wire _guard155 = _guard153 & _guard154;
wire _guard156 = wrapper_early_reset_cond0_done_out;
wire _guard157 = ~_guard156;
wire _guard158 = fsm0_out == 2'd1;
wire _guard159 = _guard157 & _guard158;
wire _guard160 = tdcc_go_out;
wire _guard161 = _guard159 & _guard160;
wire _guard162 = fsm0_out == 2'd3;
wire _guard163 = fsm_out < 3'd3;
wire _guard164 = early_reset_static_seq_go_out;
wire _guard165 = _guard163 & _guard164;
wire _guard166 = fsm_out < 3'd3;
wire _guard167 = early_reset_static_seq_go_out;
wire _guard168 = _guard166 & _guard167;
wire _guard169 = fsm_out < 3'd3;
wire _guard170 = early_reset_static_seq_go_out;
wire _guard171 = _guard169 & _guard170;
wire _guard172 = early_reset_cond0_go_out;
wire _guard173 = fsm_out == 3'd4;
wire _guard174 = early_reset_static_seq_go_out;
wire _guard175 = _guard173 & _guard174;
wire _guard176 = _guard172 | _guard175;
wire _guard177 = early_reset_cond0_go_out;
wire _guard178 = fsm_out == 3'd4;
wire _guard179 = early_reset_static_seq_go_out;
wire _guard180 = _guard178 & _guard179;
wire _guard181 = _guard177 | _guard180;
wire _guard182 = comb_reg_out;
wire _guard183 = ~_guard182;
wire _guard184 = fsm_out == 3'd0;
wire _guard185 = _guard183 & _guard184;
assign adder1_left =
  _guard1 ? fsm_out :
  3'd0;
assign adder1_right =
  _guard2 ? 3'd1 :
  3'd0;
assign done = _guard3;
assign out = t_out;
assign fsm_write_en = _guard8;
assign fsm_clk = clk;
assign fsm_reset = reset;
assign fsm_in =
  _guard11 ? adder1_out :
  _guard14 ? adder_out :
  _guard17 ? adder0_out :
  _guard28 ? 3'd0 :
  3'd0;
assign adder_left =
  _guard29 ? fsm_out :
  3'd0;
assign adder_right =
  _guard30 ? 3'd1 :
  3'd0;
assign early_reset_init0_go_in = _guard31;
assign comb_reg_write_en = _guard36;
assign comb_reg_clk = clk;
assign comb_reg_reset = reset;
assign comb_reg_in =
  _guard41 ? lt_out :
  1'd0;
assign early_reset_cond0_done_in = ud_out;
assign wrapper_early_reset_init0_go_in = _guard47;
assign t_write_en = _guard52;
assign t_clk = clk;
assign t_reset = reset;
assign t_in =
  _guard53 ? 32'd1 :
  _guard56 ? mul_out :
  'x;
assign early_reset_cond0_go_in = _guard57;
assign wrapper_early_reset_cond0_done_in = _guard60;
assign while_wrapper_early_reset_static_seq_go_in = _guard66;
assign tdcc_go_in = go;
assign fsm0_write_en = _guard85;
assign fsm0_clk = clk;
assign fsm0_reset = reset;
assign fsm0_in =
  _guard90 ? 2'd1 :
  _guard91 ? 2'd0 :
  _guard96 ? 2'd3 :
  _guard101 ? 2'd2 :
  2'd0;
assign early_reset_init0_done_in = ud0_out;
assign count_write_en = _guard106;
assign count_clk = clk;
assign count_reset = reset;
assign count_in =
  _guard107 ? 32'd0 :
  _guard110 ? incr_out :
  'x;
assign incr_left = 32'd1;
assign incr_right = count_out;
assign adder0_left =
  _guard117 ? fsm_out :
  3'd0;
assign adder0_right =
  _guard118 ? 3'd1 :
  3'd0;
assign early_reset_static_seq_go_in = _guard119;
assign wrapper_early_reset_init0_done_in = _guard122;
assign signal_reg_write_en = _guard139;
assign signal_reg_clk = clk;
assign signal_reg_reset = reset;
assign signal_reg_in =
  _guard152 ? 1'd1 :
  _guard155 ? 1'd0 :
  1'd0;
assign wrapper_early_reset_cond0_go_in = _guard161;
assign tdcc_done_in = _guard162;
assign early_reset_static_seq_done_in = ud1_out;
assign mul_clk = clk;
assign mul_left = base;
assign mul_reset = reset;
assign mul_go = _guard168;
assign mul_right = t_out;
assign lt_left =
  _guard176 ? count_out :
  32'd0;
assign lt_right =
  _guard181 ? exp :
  32'd0;
assign while_wrapper_early_reset_static_seq_done_in = _guard185;
// COMPONENT END: pow
endmodule
