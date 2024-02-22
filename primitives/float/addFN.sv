`ifndef __ADDFN_V__
`define __ADDFN_V__

`include "primitives/float/source/fNToRecFN.v"
`include "primitives/float/source/addRecFN.v"
`include "primitives/float/source/recFNToFN.v"

module addFN #(parameter expWidth = 3, parameter sigWidth = 3) (
    input [(`floatControlWidth - 1):0] control,
    input subOp,
    input [(expWidth + sigWidth - 1):0] a,
    input [(expWidth + sigWidth - 1):0] b,
    input [2:0] roundingMode,
    output [(expWidth + sigWidth - 1):0] out,
    output [4:0] exceptionFlags
);

    // Intermediate signals for recoded formats
    wire [(expWidth + sigWidth):0] a_recoded, b_recoded;
    wire [(expWidth + sigWidth):0] result_recoded;

    // Convert 'a' and 'b' from standard to recoded format
    fNToRecFN #(expWidth, sigWidth) convert_a(
        .in_(a),
        .out(a_recoded)
    );

    fNToRecFN #(expWidth, sigWidth) convert_b(
        .in_(b),
        .out(b_recoded)
    );

    // Compute recoded numbers
    addRecFN #(expWidth, sigWidth) adder(
        .control(control),
        .subOp(subOp),
        .a(a_recoded),
        .b(b_recoded),
        .roundingMode(roundingMode),
        .out(result_recoded),
        .exceptionFlags(exceptionFlags)
    );

    // Convert the result back to standard format
    recFNToFN #(expWidth, sigWidth) convert_res(
        .in_(result_recoded),
        .out(out)
    );

endmodule


`endif /* __ADDFN_V__ */
