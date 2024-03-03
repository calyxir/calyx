module stallable_mult #(
    parameter WIDTH = 32
) (
    input wire clk,
    input wire reset,
    input wire stall,
    // inputs
    input wire [WIDTH-1:0] left,
    input wire [WIDTH-1:0] right,
    // The input has been committed
    output wire [WIDTH-1:0] out
);

logic [WIDTH-1:0] lt, rt, buff0, buff1, buff2, tmp_prod;

assign out = buff2;
assign tmp_prod = lt * rt;

always_ff @(posedge clk) begin
    if (reset) begin
        lt <= 0;
        rt <= 0;
        buff0 <= 0;
        buff1 <= 0;
        buff2 <= 0;
    end else if (!stall) begin
        lt <= left;
        rt <= right;
        buff0 <= tmp_prod;
        buff1 <= buff0;
        buff2 <= buff1;
    end
end

endmodule
