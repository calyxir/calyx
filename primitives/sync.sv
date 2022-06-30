/**
* Synchronization primitives for Calyx
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
  logic one_hot;
  logic arbiter;
  logic write_en;

  // States
  logic READ_ST, WRITE_ST;

  assign write_en = write_en_0 || write_en_1;
  assign READ_ST = is_full && read_en;
  assign WRITE_ST = !is_full && write_en;
  assign one_hot = !(write_en_0 && write_en_1);

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

  // Value of arbiter.
  always_ff @(posedge clk) begin
    if (reset) 
      arbiter <= 0;
    else if (WRITE_ST) begin
      if (one_hot)
        arbiter <= arbiter;
      else if (arbiter == 0)
        arbiter <= 1;
      else 
        arbiter <= 0;
      end
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
  always_ff @(posedge clk) begin
    if (reset)
      state <= 0;
    else if (WRITE_ST) begin
      if (one_hot) begin
        if (write_en_0) 
          state <= in_0;
        else 
          state <= in_1;
      end
      else if (arbiter == 0)
        state <= in_0;
      else 
        state <= in_1;
    end
    else if (READ_ST)
      state <= 'x;  // This could've been a latch but explicitly make it undefined.
    else
      state <= state;
  end

  // Done signal for write_0 commital
  always_ff @(posedge clk) begin
    if (reset)
      write_done_0 <= 0;
    else if (WRITE_ST) begin
      if (one_hot && write_en_0 == 1) 
        write_done_0 <= 1;
      else if (arbiter == 0)
        write_done_0 <= 1;
      else 
        write_done_0 <= 0;
    end
    else
      write_done_0 <= 0;
  end

  //Done signal for write_1 commital
  always_ff @(posedge clk) begin
    if (reset)
      write_done_1 <= 1;
    else if (WRITE_ST) begin
      if (one_hot && write_en_1 == 1)
        write_done_1 <= 1;
      else if (arbiter == 1)
        write_done_1 <= 1;
      else 
        write_done_1 <= 0;
    end
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
