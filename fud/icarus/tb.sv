module TOP;

// Signals for the main module.
logic go, done, clk, reset;
main #() main (
  .go(go),
  .clk(clk),
  .reset(reset),
  .done(done)
);

// Cycle counter. Make this signed to catch errors with cycle simulation
// counts.
logic signed [0:63] cycle_count;

always_ff @(posedge clk) begin
  cycle_count <= cycle_count + 1;
end

// Output location of the VCD file
string OUT;
// Disable VCD tracing
int NOTRACE;
// Maximum number of cycles to simulate
int CYCLE_LIMIT;
// Dummy variable to track value returned by $value$plusargs
int CODE;

initial begin
  CODE = $value$plusargs("OUT=%s", OUT);
  CODE = $value$plusargs("CYCLE_LIMIT=%d", CYCLE_LIMIT);
  if (CYCLE_LIMIT != 0) begin
    $display("cycle limit set to %d", CYCLE_LIMIT);
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
  go = 0;
  clk = 0;
  reset = 0;
  cycle_count = 0;

  // Reset phase for 5 cycles
  #10;
  reset = 1;
  clk = 1;
  repeat(5) begin
    #10 clk = ~clk;
  end


  // Start the design
  #10;
  reset = 0;
  clk = 1;
  go = 1;

  forever begin
    #10 clk = ~clk;
    if (done == 1) begin
      $display("Simulated %d cycles", cycle_count - 5);
      $finish;
    end else if (cycle_count != 0 && cycle_count == CYCLE_LIMIT) begin
      $display("Cycle limit reached");
      $display("Simulated %d cycles", cycle_count - 5);
      $finish;
    end
  end
end

endmodule
