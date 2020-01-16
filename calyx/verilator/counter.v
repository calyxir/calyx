module counter #(
  parameter WIDTH = 8
)(
  // system signals
  input  wire             clk,
  input  wire             rst,
  // counter signas
  input  wire             cen,  // counter enable
  input  wire             wen,  // write enable
  input  wire [WIDTH-1:0] dat,  // input data
  output reg  [WIDTH-1:0] o_p,  // output value (posedge counter)
  output reg  [WIDTH-1:0] o_n   // output value (negedge counter)
);


always @ (posedge clk, posedge rst)
if (rst) o_p <= {WIDTH{1'b0}};
else     o_p <= wen ? dat : o_p + {{WIDTH-1{1'b0}}, cen};


always @ (negedge clk, posedge rst)
if (rst) o_n <= {WIDTH{1'b0}};
else     o_n <= wen ? dat : o_n + {{WIDTH-1{1'b0}}, cen};


endmodule
