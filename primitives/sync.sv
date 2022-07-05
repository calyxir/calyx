/**
* Synchronization primitives for Calyx
* Requirements: 
* 1. write_en_* signals should remain 1 until the 
*    corresponding done_* signals are set to 1 once they are set to 1.
* 2. in_* should remain the same value once their corresponding write_en_* signals
*    are set high until their corresponding done_* signals are set to 1.
* 3. read_en signal should remain 1 until the read_done signal is set to 1. 
*/

// M-structure: Register primitive that blocks writes until a read happens.
module std_sync_reg #(
    parameter WIDTH = 32
) (
   input wire [ WIDTH-1:0]    in_0,
   input wire [ WIDTH-1:0]    in_1,
   input wire                 read_en,
   input wire                 write_en_0,
   input wire                 write_en_1,
   input wire                 clk,
   input wire                 reset,
    // output
   output logic [WIDTH - 1:0] out,
   output logic               write_done_0,
   output logic               write_done_1,
   output logic               read_done
);

  logic is_full;
  logic [WIDTH - 1:0] state;
  logic arbiter;

  // States
  logic READ_ST, WRITE_ST, WRITE_ONE_HOT, WRITE_MULT;

  assign READ_ST = is_full && read_en;
  assign WRITE_ONE_HOT = !is_full && (write_en_0 ^ write_en_1);
  assign WRITE_MULT = !is_full && (write_en_0 && write_en_1);

  // State transitions
  always_ff @(posedge clk) begin
    if (reset)
      is_full <= 0;
    else if (WRITE_ONE_HOT || WRITE_MULT)
      is_full <= 1;
    else if (READ_ST)
      is_full <= 0;
    else
      is_full <= is_full;
  end

  // Value of arbiter.
  // The arbiter is round robin: if in the current cycle it is 0, the next cycle
  // it is set to 1 and vice versa.
  // If the arbiter is not used to decide which value to write in for the current
  // cycle, then in the next cycle its value does not change.
  always_ff @(posedge clk) begin
    if (reset) 
      arbiter <= 0;
    else if (WRITE_MULT && arbiter == 0)
      arbiter <= 1;
    else if (WRITE_MULT && arbiter == 1)
      arbiter <= 0;
    else 
      arbiter <= arbiter;
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
  // If only one writer is active, we write the active value in
  // If multiple writers are active at the same time, the arbiter decides which 
  // writer's input to take
  always_ff @(posedge clk) begin
    if (reset)
      state <= 0;
    else if (WRITE_ONE_HOT && write_en_0 == 1)
      state <= in_0;
    else if (WRITE_ONE_HOT && write_en_1 == 1)
      state <= in_1;
    else if (WRITE_MULT && arbiter == 0)
      state <= in_0;
    else if (WRITE_MULT && arbiter == 1)
      state <= in_1;
    else if (READ_ST)
      state <= 'x;  // This could've been a latch but explicitly make it undefined.
    else
      state <= state;
  end

  // Done signal for write_0 commital
  // Two scenarios that write_done_0 is set to 1:
  // 1. in_0 is the only writer for the current cycle
  // 2. Two writers are active at the same time, and the arbiter chooses
  //    in_0 to write into the register
  always_ff @(posedge clk) begin
    if (reset)
      write_done_0 <= 0;
    else if (WRITE_ONE_HOT && write_en_0 == 1)
      write_done_0 <= 1;
    else if (WRITE_MULT && arbiter == 0)
      write_done_0 <= 1;
    else
      write_done_0 <= 0;
  end

  //Done signal for write_1 commital
  // Two scenarios that write_done_1 is set to 1:
  // 1. in_1 is the only writer for the current cycle
  // 2. Two writers are active at the same time, and the arbiter chooses
  //    in_1 to write into the register
  always_ff @(posedge clk) begin
    if (reset)
      write_done_1 <= 0;
    else if (WRITE_ONE_HOT && write_en_1 == 1)
      write_done_1 <= 1;
    else if (WRITE_MULT && arbiter == 1)
      write_done_1 <= 1;
    else
      write_done_1 <= 0;
    end

  // Done signal for read commital
  always_ff @(posedge clk) begin
    if (reset)
      read_done <= 0;
    else if (READ_ST)
      read_done <= 1;
    else
      read_done <= 0;
  end

endmodule
