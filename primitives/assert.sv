module std_assert (
   input wire logic clk,
   input wire logic reset,
   input logic in,
   input logic en,
   output logic out
);
    always_ff @(posedge clk) begin
       out <= in;
       if (in == '0 & en) begin
            $fatal(1, "Assertion failed in assert primitive: input was 0");
        end
    end
endmodule
