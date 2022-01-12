/**
* Synchronization primitives for Calyx
*/

// M-structure: Register primitive that blocks writes until a read happens.
module std_sync_reg #(
    parameter WIDTH = 32
) (
   input wire [ WIDTH-1:0]    in,
   input wire                 read_en,
   input wire                 write_en,
   input wire                 clk,
   input wire                 reset,
    // output
   output logic [WIDTH - 1:0] out,
   output logic               done,
   output logic               blocked
);

  logic is_full;
  logic [WIDTH - 1:0] state;

  // States
  logic READ_ST, WRITE_ST;

  assign READ_ST = is_full && read_en;
  assign WRITE_ST = !is_full && write_en;

  // State transitions
  always_ff @(posedge clk) begin
    if (reset)
      is_full <= 0;
    else if (WRITE_ST)
      is_full <= 1;
    else if (READ_ST)
      is_full <= 0;
    else
      is_full <= is_full;
  end

  // Value of output port.
  // Note that output is only available for one cycle.
  always_ff @(posedge clk) begin
    if (reset)
      out <= 0;
    else if (READ_ST)
      out <= state;
    else
      out <= 'x; // This could've been a latch but we explicitly define the output as undefined.
  end

  // Writing values
  always_ff @(posedge clk) begin
    if (reset)
      state <= 0;
    else if (WRITE_ST)
      state <= in;
    else if (READ_ST)
      state <= 'x;  // This could've been a latch but explicitly make it undefined.
    else
      state <= state;
  end

  // Done signal
  always_ff @(posedge clk) begin
    if (reset)
      done <= 0;
    else if (WRITE_ST)
      done <= 1;
    else
      done <= 0;
  end

  // Blocked signal
  always_ff @(posedge clk) begin
    if (reset)
      blocked <= 0;
    else if ((!is_full && read_en) || (is_full && write_en))
      blocked <= 1;
    else
      blocked <= 0;
  end

endmodule
