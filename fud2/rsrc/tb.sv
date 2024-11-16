module toplevel;

// Signals for the main module.
logic go, done, clk, reset;
main #() main (
  .go(go),
  .clk(clk),
  .reset(reset),
  .done(done)
);

localparam RESET_CYCLES = 3;

// Cycle counter. Make this signed to catch errors with cycle simulation
// counts.
logic signed [63:0] cycle_count = 64'd0;

always_ff @(posedge clk) begin
  cycle_count <= cycle_count + 1;
end

always_ff @(posedge clk) begin
  // Reset the design for a few cycles
  if (cycle_count < RESET_CYCLES) begin
    reset <= 1;
    go <= 0;
  end else begin
    reset <= 0;
    go <= 1;
  end
end

// Output location of the VCD file
string OUT;
// Disable VCD tracing
int NOTRACE;
// Maximum number of cycles to simulate
longint cycle_limit = `CYCLE_LIMIT;
// Dummy variable to track value returned by $value$plusargs
int CODE;

initial begin
  CODE = $value$plusargs("OUT=%s", OUT);
  CODE = $value$plusargs("cycle_limit=%d", cycle_limit);
  if (cycle_limit != 0) begin
    $display("cycle limit set to %d", cycle_limit);
  end
  CODE = $value$plusargs("NOTRACE=%d", NOTRACE);
  if (NOTRACE == 0) begin
    $display("VCD tracing enabled");
    $dumpfile(OUT);
    $dumpvars(0,main);
  end else begin
    $display("VCD tracing disabled");
  end

  // Initial values
  clk = 0;

  forever begin
    #10 clk = ~clk;
    if (cycle_count > RESET_CYCLES && done == 1) begin
      // Subtract 1 because the cycle counter is incremented at the end of the
      // cycle.
      $display("Simulated %d cycles", cycle_count - RESET_CYCLES - 1);
      $finish;
    end else if (cycle_count != 0 && cycle_count == cycle_limit + RESET_CYCLES) begin
      $display("reached limit of %d cycles", cycle_limit);
      $finish;
    end
  end
end

endmodule
