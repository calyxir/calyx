
/// This is mostly used for testing the static guarantees currently.
/// A realistic implementation would probably take four cycles.
module pipelined_mult (
    input wire clk,
    input wire reset,
    // inputs
    input wire [31:0] left,
    input wire [31:0] right,
    // The input has been committed
    output wire [31:0] out
);

logic [31:0] lt, rt, buff0, buff1, buff2, tmp_prod;

assign out = buff2;
assign tmp_prod = lt * rt;

always_ff @(posedge clk) begin
    if (reset) begin
        lt <= 0;
        rt <= 0;
        buff0 <= 0;
        buff1 <= 0;
        buff2 <= 0;
    end else begin
        lt <= left;
        rt <= right;
        buff0 <= tmp_prod;
        buff1 <= buff0;
        buff2 <= buff1;
    end
end

endmodule
