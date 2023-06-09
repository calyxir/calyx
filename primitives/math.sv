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
