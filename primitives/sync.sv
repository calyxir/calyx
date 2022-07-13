/**
* Synchronization primitives for Calyx
* Requirements: 
* 1. write_en_* signals should remain 1 until the 
*    corresponding done_* signals are set to 1 once they are set to 1.
* 2. in_* should remain the same value once their corresponding write_en_* signals
*    are set high until their corresponding done_* signals are set to 1.
* 3. read_en_* signals should remain 1 until the read_done_* signal is set to 1. 
* 4. read_en_* signals should be set to 0 once the read_done_* signals are high.
*/

// M-structure: Register primitive that blocks writes until a read happens.
module std_sync_reg #(
    parameter WIDTH = 32
) (
   input wire [ WIDTH-1:0]    in_0,
   input wire [ WIDTH-1:0]    in_1,
   input wire                 read_en_0,
   input wire                 read_en_1,
   input wire                 write_en_0,
   input wire                 write_en_1,
   input wire                 clk,
   input wire                 reset,
    // output
   output logic [WIDTH - 1:0] out_0,
   output logic [WIDTH - 1:0] out_1,
   output logic               write_done_0,
   output logic               write_done_1,
   output logic               read_done_0,
   output logic               read_done_1,
   output logic [WIDTH - 1:0] peek
);

  logic is_full;
  logic [WIDTH - 1:0] state;
  logic arbiter_w;
  logic arbiter_r;

  // States
  logic READ_ONE_HOT, READ_MULT, WRITE_ONE_HOT, WRITE_MULT, WRITE_0, WRITE_1, READ_0, READ_1;

  assign READ_ONE_HOT = is_full && (read_en_0 ^ read_en_1);
  assign READ_MULT = is_full && (read_en_0 && read_en_1);
  assign WRITE_ONE_HOT = !is_full && (write_en_0 ^ write_en_1);
  assign WRITE_MULT = !is_full && (write_en_0 && write_en_1);
  assign WRITE_0 = (WRITE_ONE_HOT && write_en_0 == 1) || (WRITE_MULT && arbiter_w == 0);
  assign WRITE_1 = (WRITE_ONE_HOT && write_en_1 == 1) || (WRITE_MULT && arbiter_w == 1);
  assign READ_0 = (READ_ONE_HOT && read_en_0 == 1) || (READ_MULT && arbiter_r == 0);
  assign READ_1 = (READ_ONE_HOT && read_en_1 == 1) || (READ_MULT && arbiter_r == 1);

  // State transitions
  always_ff @(posedge clk) begin
    if (reset)
      is_full <= 0;
    else if (WRITE_ONE_HOT || WRITE_MULT)
      is_full <= 1;
    else if (READ_ONE_HOT || READ_MULT)
      is_full <= 0;
    else
      is_full <= is_full;
  end

  // Value of writer arbiter.
  // The arbiter is round robin: if in the current cycle it is 0, the next cycle
  // it is set to 1 and vice versa.
  // If the arbiter is not used to decide which value to write in for the current
  // cycle, then in the next cycle its value does not change.
  always_ff @(posedge clk) begin
    if (reset) 
      arbiter_w <= 0;
    else if (WRITE_MULT && arbiter_w == 0)
      arbiter_w <= 1;
    else if (WRITE_MULT && arbiter_w == 1)
      arbiter_w <= 0;
    else 
      arbiter_w <= arbiter_w;
  end

  // Value of reader arbiter.
  // The arbiter is round robin: if in the current cycle it is 0, the next cycle
  // it is set to 1 and vice versa.
  // If the arbiter is not used to decide which reader to give its value to for the current
  // cycle, then in the next cycle its value does not change.
  always_ff @(posedge clk) begin
    if (reset) 
      arbiter_r <= 0;
    else if (READ_MULT && arbiter_r == 0)
      arbiter_r <= 1;
    else if (READ_MULT && arbiter_r == 1)
      arbiter_r <= 0;
    else 
      arbiter_r <= arbiter_r;
  end
      

  // Value of out_0 port.
  // Note that output is only available for one cycle.
  // out_0 has value only when 
  // 1. only read_en_0 is high
  // 2. read_en_0 and read_en_1 are both high and arbiter_r == 0.
  always_ff @(posedge clk) begin
    if (reset)
      out_0 <= 0;
    else if (READ_0)
      out_0 <= state;
    else
      out_0 <= 'x; // This could've been a latch but we explicitly define the output as undefined.
  end

  // Value of out_1 port.
  // Note that output is only available for one cycle.
  // out_1 has value only when 
  // 1. only read_en_1 is high
  // 2. read_en_0 and read_en_1 are both high and arbiter_r == 1.
  always_ff @(posedge clk) begin
    if (reset)
      out_1 <= 0;
    else if (READ_1)
      out_1 <= state;
    else
      out_1 <= 'x; // This could've been a latch but we explicitly define the output as undefined.
  end

  // Writing values
  // If only one writer is active, we write the active value in
  // If multiple writers are active at the same time, the arbiter decides which 
  // writer's input to take
  always_ff @(posedge clk) begin
    if (reset)
      state <= 0;
    else if (WRITE_0)
      state <= in_0;
    else if (WRITE_1)
      state <= in_1;
    else if (READ_ONE_HOT || READ_MULT)
      state <= 'x;  // This could've been a latch but explicitly make it undefined.
    else
      state <= state;
  end

  //Value of the "peek" port.
  // If the register is full, peek holds the same value as the state of the register.
  // If the register is empty, peek holds the most recent valid state of the register.
  always_ff @(posedge clk) begin
    if (reset)
      peek <= 0;
    else if (WRITE_0)
      peek <= in_0;
    else if (WRITE_1)
      peek <= in_1;
    else
      peek <= peek;
  end

  // Done signal for write_0 commital
  // Two scenarios that write_done_0 is set to 1:
  // 1. in_0 is the only writer for the current cycle
  // 2. Two writers are active at the same time, and the arbiter chooses
  //    in_0 to write into the register
  always_ff @(posedge clk) begin
    if (reset)
      write_done_0 <= 0;
    else if (WRITE_0)
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
    else if (WRITE_1)
      write_done_1 <= 1;
    else
      write_done_1 <= 0;
    end

  // Done signal for read_0 commital
  always_ff @(posedge clk) begin
    if (reset)
      read_done_0 <= 0;
    else if (READ_0)
      read_done_0 <= 1;
    else
      read_done_0 <= 0;
  end

  // Done signal for read_1 commital
  always_ff @(posedge clk) begin
    if (reset)
      read_done_1 <= 0;
    else if (READ_1)
      read_done_1 <= 1;
    else
      read_done_1 <= 0;
  end

  //Simulation self test against overlapping of mutually exclusive states
  `ifdef VERILATOR
    always @(posedge clk) begin
      if (READ_0 && READ_1) 
        $error(
          "\nstd_sync_reg: overlapping of mutually exclusive states!\n",
          "can be at only one of READ_0 and READ_1", 
        );
      else if (WRITE_0 && WRITE_1)
        $error(
          "\nstd_sync_reg: overlapping of mutually exclusive states!\n",
          "can be at only one of WRITE_0 and WRITE_1", 
        );
    end
  `endif

endmodule
