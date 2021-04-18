// 1-dimensional memory with the ability
// to read each line in parallel.
module par_mem_d1 #(
    parameter WIDTH = 32,
    parameter SIZE = 8,
    parameter INDEX_SIZE = 3
) (
   input wire  logic [INDEX_SIZE-1:0] index,
   input wire  logic      [WIDTH-1:0] write_data,
   input wire  logic                  write_en,
   input wire  logic                  clk,
   output      logic      [WIDTH-1:0] read0,
   output      logic      [WIDTH-1:0] read1,
   output      logic      [WIDTH-1:0] read2,
   output      logic      [WIDTH-1:0] read3,
   output      logic      [WIDTH-1:0] read4,
   output      logic      [WIDTH-1:0] read5,
   output      logic      [WIDTH-1:0] read6,
   output      logic      [WIDTH-1:0] read7,
   output      logic                  done
);

  logic [WIDTH-1:0] mem[SIZE-1:0];

  assign read0 = mem[0];
  assign read1 = mem[1];
  assign read2 = mem[2];
  assign read3 = mem[3];
  assign read4 = mem[4];
  assign read5 = mem[5];
  assign read6 = mem[6];
  assign read7 = mem[7];

  always_ff @(posedge clk) begin
    if (write_en) begin
      mem[index] <= write_data;
      done <= 1'd1;
    end else done <= 1'd0;
  end
endmodule
