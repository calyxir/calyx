`ifndef __DIVSQRTFN_V__
`define __DIVSQRTFN_V__

`include "HardFloat_consts.vi"

module std_divSqrtFN #(
    parameter expWidth = 8,
    parameter sigWidth = 24,
    parameter numWidth = 32
) (
    input clk,
    input reset,
    input go,
    input [(`floatControlWidth - 1):0] control,
    input sqrtOp,
    input [(expWidth + sigWidth - 1):0] left,
    input [(expWidth + sigWidth - 1):0] right,
    input [2:0] roundingMode,
    output logic [(expWidth + sigWidth - 1):0] out,
    output logic [4:0] exceptionFlags,
    output logic done
);

    // Intermediate signals for recoded formats
    wire [(expWidth + sigWidth):0] l_recoded, r_recoded;

    // Convert inputs from IEEE-754 to recoded format
    fNToRecFN #(expWidth, sigWidth) convert_l(
        .in(left),
        .out(l_recoded)
    );

    fNToRecFN #(expWidth, sigWidth) convert_r(
        .in(right),
        .out(r_recoded)
    );

    // Intermediate signals from `divSqrtRecFNToRaw_small`
    wire inReady, outValid;
    wire [2:0] roundingModeOut;
    wire invalidExc, infiniteExc, out_isNaN, out_isInf, out_isZero, out_sign;
    wire signed [(expWidth + 1):0] out_sExp;
    wire [(sigWidth + 2):0] out_sig;

    // Call HardFloat's `divSqrtRecFNToRaw_small` instead of `divSqrtRecFN_small` because the latter has signal duplicate declaration issue.
    divSqrtRecFNToRaw_small #(expWidth, sigWidth) divSqrtRecFNToRaw (
        .nReset(~reset),
        .clock(clk),
        .control(control),
        .inReady(inReady),
        .inValid(go),
        .sqrtOp(sqrtOp),
        .a(l_recoded),
        .b(r_recoded),
        .roundingMode(roundingMode),
        .outValid(outValid),
        .sqrtOpOut(),  // Not connected
        .roundingModeOut(roundingModeOut),
        .invalidExc(invalidExc),
        .infiniteExc(infiniteExc),
        .out_isNaN(out_isNaN),
        .out_isInf(out_isInf),
        .out_isZero(out_isZero),
        .out_sign(out_sign),
        .out_sExp(out_sExp),
        .out_sig(out_sig)
    );

    // Intermediate signals for rounded result
    wire [(expWidth + sigWidth):0] res_recoded;

    // Round the output using HardFloat's rounding module
    roundRawFNToRecFN#(expWidth, sigWidth, 0) 
        roundRawOut (
            control,
            invalidExc,
            infiniteExc,
            out_isNaN,
            out_isInf,
            out_isZero,
            out_sign,
            out_sExp,
            out_sig,
            roundingModeOut,
            res_recoded,
            exceptionFlags
    );

    // Convert result back to IEEE754 format
    wire [(expWidth + sigWidth - 1):0] res_std;
    recFNToFN #(expWidth, sigWidth) convert_res(
        .in(res_recoded),
        .out(res_std)
    );

    logic done_buf[31:0];  // Extend pipeline buffer to 10 cycles
    assign done = done_buf[31];  // Assert `done` only after 10 cycles

    // Start execution
    logic start;
    assign start = go;

    // Reset all pipeline registers on reset
    always_ff @(posedge clk) begin
        if (reset) begin
            done_buf <= '{default: 0};  // Reset entire pipeline
        end else if (start) begin
            done_buf[0] <= 1;  // Start first stage
        end else begin
            done_buf[0] <= 0;  // Clear when not starting
        end
    end

    // Shift `done_buf` through 32 cycles. According to the Berkeley Hardfloat documentation:
    // > After some number of clock cycles, outValid is asserted true for exactly one clock cycle ...
    // And after running real examples, this is 30 cycles; adding 2 more cycles to be more conservative.
    always_ff @(posedge clk) begin
        if (reset) begin
            done_buf <= '{default: 0};  // Reset again
        end else begin
            for (int i = 1; i < 32; i++) begin
                done_buf[i] <= done_buf[i-1];  // Shift pipeline correctly
            end
        end
    end

    // Store the computed output value
    always_ff @(posedge clk) begin
        if (reset) begin
            out <= 0;
        end else if (outValid) begin
            out <= res_std;
        end
    end

endmodule

`endif /* __DIVSQRTFN_V__ */
