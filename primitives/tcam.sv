// 1-dimensional memory with the ability
// to read each line in parallel.
module par_mem_d1 #(
    parameter WIDTH,
    parameter SIZE,
    parameter INDEX_SIZE
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
   output      logic      [WIDTH-1:0] read8,
   output      logic      [WIDTH-1:0] read9,
   output      logic      [WIDTH-1:0] read10,
   output      logic      [WIDTH-1:0] read11,
   output      logic      [WIDTH-1:0] read12,
   output      logic      [WIDTH-1:0] read13,
   output      logic      [WIDTH-1:0] read14,
   output      logic      [WIDTH-1:0] read15,
   output      logic      [WIDTH-1:0] read16,
   output      logic      [WIDTH-1:0] read17,
   output      logic      [WIDTH-1:0] read18,
   output      logic      [WIDTH-1:0] read19,
   output      logic      [WIDTH-1:0] read20,
   output      logic      [WIDTH-1:0] read21,
   output      logic      [WIDTH-1:0] read22,
   output      logic      [WIDTH-1:0] read23,
   output      logic      [WIDTH-1:0] read24,
   output      logic      [WIDTH-1:0] read25,
   output      logic      [WIDTH-1:0] read26,
   output      logic      [WIDTH-1:0] read27,
   output      logic      [WIDTH-1:0] read28,
   output      logic      [WIDTH-1:0] read29,
   output      logic      [WIDTH-1:0] read30,
   output      logic      [WIDTH-1:0] read31,
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
  assign read8 = mem[8];
  assign read9 = mem[9];
  assign read10 = mem[10];
  assign read11 = mem[11];
  assign read12 = mem[12];
  assign read13 = mem[13];
  assign read14 = mem[14];
  assign read15 = mem[15];
  assign read16 = mem[16];
  assign read17 = mem[17];
  assign read18 = mem[18];
  assign read19 = mem[19];
  assign read20 = mem[20];
  assign read21 = mem[21];
  assign read22 = mem[22];
  assign read23 = mem[23];
  assign read24 = mem[24];
  assign read25 = mem[25];
  assign read26 = mem[26];
  assign read27 = mem[27];
  assign read28 = mem[28];
  assign read29 = mem[29];
  assign read30 = mem[30];
  assign read31 = mem[31];

  always_ff @(posedge clk) begin
    if (write_en) begin
      mem[index] <= write_data;
      done <= 1'd1;
    end else done <= 1'd0;
  end
endmodule
