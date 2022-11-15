module mult_pipe (
    input wire clk,
    input wire reset,
    // The input data is valid
    input wire valid,
    // inputs
    input wire [31:0] left,
    input wire [31:0] right,
    // The input has been committed
    output reg read_done,
    output wire [31:0] out
);

    reg [31:0] reg_out, tmp;

    assign tmp = left * right;

    always @(posedge clk) begin
        if (reset) begin
            reg_out <= 0;
        end else begin
            reg_out <= tmp;
        end
    end

    always @(posedge clk) begin
        if (reset)
            read_done <= 0;
        else if (valid)
            read_done <= 1;
        else
            read_done <= 0;
    end

    assign out = reg_out;

endmodule