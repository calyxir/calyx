module test_bench;

// Signals for the main module.
logic go, done, clk, reset;
main #() m (
  .go(go),
  .clk(clk),
  .reset(reset),
  .done(done)
);

string OUT;

initial begin
  $value$plusargs("OUT=%s", OUT);
  $dumpfile(OUT);
  $dumpvars(0,m);

  // Initial values
  go = 0;
  clk = 0;
  reset = 0;

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
      $finish;
    end
  end
end

endmodule
