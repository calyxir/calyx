module TOP;

// Signals for the main module.
logic go, done, clk, reset;

// Adding in fields for memory controlled externally
wire mem_addr0;
wire [31:0] mem_write_data;
wire mem_write_en;
wire [31:0] mem_read_data;
wire mem_done;

// Declaring memory
std_mem_d1 # (
    .IDX_SIZE(1),
    .SIZE(1),
    .WIDTH(32)
) mem (
    .addr0(mem_addr0),
    .clk(clk),
    .done(mem_done),
    .read_data(mem_read_data),
    .reset(reset),
    .write_data(mem_write_data),
    .write_en(mem_write_en)
);

main #() main (
  .go(go),
  .clk(clk),
  .reset(reset),
  .done(done),
  .mem_addr0(mem.addr0),
  .mem_write_data(mem.write_data),
  .mem_write_en(mem.write_en),
  .mem_read_data(mem.read_data),
  .mem_done(mem.done)
);

localparam RESET_CYCLES = 3;

// Cycle counter. Make this signed to catch errors with cycle simulation
// counts.
logic signed [63:0] cycle_count;

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
longint CYCLE_LIMIT;
// Dummy variable to track value returned by $value$plusargs
int CODE;
// Directory to read/write memory
string DATA;

initial begin
  CODE = $value$plusargs("DATA=%s", DATA);
  $display("DATA (path to meminit files): %s", DATA);
  $readmemh({DATA, "/mem.dat"}, mem.mem); 
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
  reset = 1;
  cycle_count = 0;

  forever begin
    #10 clk = ~clk;
    if (cycle_count > RESET_CYCLES && done == 1) begin
      // Subtract 1 because the cycle counter is incremented at the end of the
      // cycle.
      $display("Simulated %d cycles", cycle_count - RESET_CYCLES - 1);
      $finish;
    end else if (cycle_count != 0 && cycle_count == CYCLE_LIMIT + RESET_CYCLES) begin
      $display("reached limit of %d cycles", CYCLE_LIMIT);
      $finish;
    end
  end
end

final begin
    $writememh({DATA, "/mem.out"}, mem.mem);
end

endmodule
