/// Memories
module std_add #(
    parameter WIDTH = 32
) (
    input  wire logic [WIDTH-1:0] left,
    input  wire logic [WIDTH-1:0] right,
    output logic      [WIDTH-1:0] out
);
  assign out = left + right;
endmodule
