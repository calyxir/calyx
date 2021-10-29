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
    localparam ITERATIONS = WIDTH+FRAC_WIDTH >> 1;
    logic [$clog2(ITERATIONS)-1:0] idx;

    logic [WIDTH-1:0] x, x_next;
    logic [WIDTH-1:0] quotient, quotient_next;
    logic [WIDTH+1:0] acc, acc_next;
    logic [WIDTH+1:0] tmp;
    logic start, running, finished;

    assign start = go && !running;
    /* verilator lint_off WIDTH */
    assign finished = (idx == (ITERATIONS - 1));

    always_ff @(posedge clk) begin
      if (reset) running <= 0;
      else if (start) running <= 1;
      else if (finished) running <= 0;
      else running <= running;
    end

    always_comb begin
      tmp = acc - {quotient, 2'b01};
      if (tmp[WIDTH+1]) begin
        // tmp is negative.
        {acc_next, x_next} = {acc[WIDTH-1:0], x, 2'b0};
        // Append a 0 to the result.
        quotient_next = quotient << 1;
      end else begin
        // tmp is positive.
        {acc_next, x_next} = {tmp[WIDTH-1:0], x, 2'b0};
        // Append a 1 to the result.
        quotient_next = {quotient[WIDTH-2:0], 1'b1};
      end
    end

    // Current idx value
    always_ff @(posedge clk) begin
      if (start)
        idx <= 0;
      else
        idx <= idx + 1;
    end

    always_ff @(posedge clk) begin
      if (start) begin
        quotient <= 0;
        {acc, x} <= {{WIDTH{1'b0}}, in, 2'b0};
      end else begin
        x <= x_next;
        acc <= acc_next;
        quotient <= quotient_next;
      end
    end

    // Done condition.
    always_ff @(posedge clk) begin
      if (idx == ITERATIONS - 1) begin
        done <= 1;
      end else begin
        done <= 0;
      end
    end

    // Latch for final value.
    always_ff @(posedge clk) begin
      if (idx == ITERATIONS-1) begin
        out <= quotient_next;
      end else begin
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
    logic [WIDTH-1:0] inp_save;

    always_latch @(posedge clk) begin
      if (go)
        inp_save <= in;
    end

    always @(posedge clk) begin
      if (done && out != $floor($sqrt(inp_save)))
        $error(
          "\nsqrt: Computed and golden outputs do not match!\n",
          "input: %0d\n", inp_save,
          /* verilator lint_off REALCVT */
          "expected: %0d\n", $floor($sqrt(inp_save)),
          "computed: %0d", out
        );
    end
  `endif
endmodule
