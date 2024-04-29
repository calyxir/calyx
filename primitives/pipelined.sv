
/// This is mostly used for testing the static guarantees currently.
/// A realistic implementation would probably take four cycles.
module pipelined_mult #(
    parameter WIDTH = 32
) (
    input wire clk,
    // inputs
    input wire [WIDTH-1:0] left,
    input wire [WIDTH-1:0] right,
    // The input has been committed
    output wire [WIDTH-1:0] out
);

logic [WIDTH-1:0] buff0, buff1, buff2, tmp_left, tmp_right, tmp_prod;

assign out = buff2;
assign tmp_prod = tmp_left * tmp_right;

always_ff @(posedge clk) begin
    tmp_left <= left;
    tmp_right <= right;
    buff0 <= tmp_prod;
    buff1 <= buff0;
    buff2 <= buff1;
end

endmodule 

/// 
module pipelined_fp_smult #(
    parameter WIDTH = 32,
    parameter INT_WIDTH = 16,
    parameter FRAC_WIDTH = 16
)(
    input wire clk,
    input wire reset,
    // inputs
    input wire [WIDTH-1:0] left,
    input wire [WIDTH-1:0] right,
    // The input has been committed
    output wire [WIDTH-1:0] out
);

logic [WIDTH-1:0] lt, rt, buff0, buff1, buff2;
logic [(WIDTH << 1) - 1:0] tmp_prod;

assign out = buff2;
assign tmp_prod = $signed(
          { {WIDTH{lt[WIDTH-1]}}, lt} *
          { {WIDTH{rt[WIDTH-1]}}, rt}
        );

always_ff @(posedge clk) begin
    if (reset) begin
        lt <= 0;
        rt <= 0;
        buff0 <= 0;
        buff1 <= 0;
        buff2 <= 0;
    end else begin
        lt <= $signed(left);
        rt <= $signed(right);
        buff0 <= tmp_prod[(WIDTH << 1) - INT_WIDTH - 1 : WIDTH - INT_WIDTH];
        buff1 <= buff0;
        buff2 <= buff1;
    end
end

endmodule
