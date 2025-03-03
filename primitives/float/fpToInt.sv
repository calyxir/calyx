`ifndef __FPTOINT_V__
`define __FPTOINT_V__

`include "HardFloat_consts.vi"

`ifndef HARD_FLOAT_CONTROL
    `define HARD_FLOAT_CONTROL `flControl_tininessAfterRounding
`endif

module std_fpToInt #(parameter expWidth = 8, parameter sigWidth = 23, parameter floatWidth = 32,  parameter intWidth = 32)
(
    input clk,
    input reset,
    input go,
    input signedOut,
    input [expWidth + sigWidth - 1:0] in,
    output logic signed [intWidth-1:0] out,
    output logic done
);

    // Intermediate signals
    wire [expWidth + sigWidth:0] recoded_fp;
    wire signed [intWidth-1:0] converted_int;

    // Convert IEEE754 standard format to HardFloat recoded format
    fNToRecFN #(expWidth, sigWidth) convert_to_rec (
        .in(in),
        .out(recoded_fp)
    );

    // Convert recoded floating-point to integer
    recFNToIN #(expWidth, sigWidth, intWidth) convert_to_int (
        .control(`HARD_FLOAT_CONTROL),
        .in(recoded_fp),
        .roundingMode(`round_minMag), // Rounds towards zero
        .signedOut(1'b1), // Signed integer output
        .out(converted_int),
        .intExceptionFlags() // We may handle exceptions later if needed
    );

    logic done_buf[1:0];
    assign done = done_buf[1];

    // Done signal logic
    always_ff @(posedge clk) begin
        if (go)
            done_buf[0] <= 1;
        else
            done_buf[0] <= 0;
    end

    always_ff @(posedge clk) begin
        if (go) 
            done_buf[1] <= done_buf[0];
        else 
            done_buf[1] <= 0;
    end

    // Output result on clock edge
    always_ff @(posedge clk) begin
        if (reset)
            out <= 0;
        else if (go)
            out <= converted_int;
    end

endmodule

`endif /* __FPTOINT_V__ */