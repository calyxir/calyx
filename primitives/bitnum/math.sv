/**
* Implements the non-restoring square root algorithm's iterative
* implementation (Figure 8): https://ieeexplore.ieee.org/document/563604
*/
module sqrt (
    input  logic [31:0] in,
    input  logic        go,
    input  logic        clk,
    output logic [31:0] out,
    output logic        done
);
  // Done state
  localparam END = 17;

  // declare the variables
  logic [31:0] a;
  logic [15:0] Q;
  logic [17:0] left, right, R;
  logic [17:0] tmp;
  logic [5:0] i;

  // Input to the add/sub circuit.
  assign right = {Q, R[17], 1'b1};
  assign left = {R[15:0], a[31:30]};

  // Done condition
  always_ff @(posedge clk) begin
    if (i == END) begin
      done <= 1;
    end else begin
      done <= 0;
    end
  end

  // Latch for final value
  always_latch @(posedge clk) begin
    if (i == END) begin
      out <= {16'd0, Q};
    end
  end

  // Output is based on current value of r
  always_comb begin
    if (R[17] == 1)
      tmp = left + right;
    else
      tmp = left - right;
  end

  // Update the current iteration counter
  always_ff @(posedge clk) begin
    if (go && i < END)
      i <= i + 1;
    else
      i <= 0;
  end

  // Quotient and remainder updates
  always_ff @(posedge clk) begin
    if (go && i < END) begin
      Q <= {Q[14:0], !tmp[17]};
      R <= tmp;
    end else begin
      Q <= 0;
      R <= 0;
    end
  end

  // Input stream update
  always_ff @(posedge clk) begin
    if (go) begin
      if (i == 0) begin
        a <= in;
      end else if (i < END) begin
        a <= {a[29:0], 2'b00};
      end else begin
        a <= 0;
      end
    end else begin
      a <= 0;
    end
  end

  // Simulation self test against unsynthesizable implementation.
  `ifdef VERILATOR
    // Save the original value of the input
    always @(posedge clk) begin
      if (i == END && Q != $floor($sqrt(in)))
        $error(
          "\nstd_sqrt: Computed and golden outputs do not match!\n",
          "input: %0d\n", in,
          /* verilator lint_off REALCVT */
          "expected: %0d", $floor($sqrt(in)),
          "  computed: %0d", Q
        );
    end
  `endif

endmodule

