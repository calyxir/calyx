`ifndef __INTOFP_V__
`define __INTOFP_V__

`include "HardFloat_consts.vi"

`ifndef HARD_FLOAT_CONTROL
    `define HARD_FLOAT_CONTROL `flControl_tininessAfterRounding
`endif

module std_intToFp #(parameter intWidth = 32, parameter expWidth = 8, parameter sigWidth = 24, parameter floatWidth = 32)
(
    input clk,
    input reset,
    input go,
    input [intWidth-1:0] in,
    input signedIn,
    output logic [expWidth+sigWidth-1:0] out,
    output logic done
);

    // Intermediate signals
    wire [expWidth + sigWidth:0] recoded_fp;

    // Convert integer to HardFloat recoded format
    iNToRecFN #(intWidth, expWidth, sigWidth) convert_to_rec (
        .control(`HARD_FLOAT_CONTROL),
        .signedIn(signedIn), // Signed or unsigned integer
        .in(in),
        .roundingMode(`round_near_even), // Default rounding mode
        .out(recoded_fp),
        .exceptionFlags() // Exception handling (to be added if needed)
    );

    // Convert recoded floating-point back to standard IEEE754 format
    recFNToFN #(expWidth, sigWidth) convert_to_fp (
        .in(recoded_fp),
        .out(out)
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

endmodule

`endif /* __INTOFP_V__ */
