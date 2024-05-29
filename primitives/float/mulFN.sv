`ifndef __MULFN_V__
`define __MULFN_V__

`include "mulRecFN.v"

module std_mulFN #(parameter expWidth = 8, parameter sigWidth = 24, parameter numWidth = 32) (
    input clk,
    input reset,
    input go,
    input [(`floatControlWidth - 1):0] control,
    input [(expWidth + sigWidth - 1):0] left,
    input [(expWidth + sigWidth - 1):0] right,
    input [2:0] roundingMode,
    output logic [(expWidth + sigWidth - 1):0] out,
    output logic [4:0] exceptionFlags,
    output done
);

    // Intermediate signals for recoded formats
    wire [(expWidth + sigWidth):0] l_recoded, r_recoded;

    // Convert 'a' and 'b' from standard to recoded format
    fNToRecFN #(expWidth, sigWidth) convert_l(
        .in(left),
        .out(l_recoded)
    );

    fNToRecFN #(expWidth, sigWidth) convert_r(
        .in(right),
        .out(r_recoded)
    );

    // Intermediate signals after the multiplier
    wire [(expWidth + sigWidth):0] res_recoded;
    wire [4:0] except_flag;

    // Compute recoded numbers
    mulRecFN #(expWidth, sigWidth) multiplier(
        .control(control),
        .a(l_recoded),
        .b(r_recoded),
        .roundingMode(roundingMode),
        .out(res_recoded),
        .exceptionFlags(except_flag)
    );

    wire [(expWidth + sigWidth - 1):0] res_std;

    // Convert the result back to standard format
    recFNToFN #(expWidth, sigWidth) convert_res(
        .in(res_recoded),
        .out(res_std)
    );

    logic done_buf[1:0];

    assign done = done_buf[1];

    // If the done buffer is completely empty and go is high then execution
    // just started.
    logic start;
    assign start = go;

    // Start sending the done signal.
    always_ff @(posedge clk) begin
        if (start)
            done_buf[0] <= 1;
        else
            done_buf[0] <= 0;
    end

    // Push the done signal through the pipeline.
    always_ff @(posedge clk) begin
        if (go) begin
            done_buf[1] <= done_buf[0];
        end else begin
            done_buf[1] <= 0;
        end
    end

    // Compute the output and save it into out
    always_ff @(posedge clk) begin
        if (reset) begin
            out <= 0;
        end else if (go) begin
            out <= res_std;
        end else begin
            out <= out;
        end
    end

endmodule


`endif /* __MULFN_V__ */

