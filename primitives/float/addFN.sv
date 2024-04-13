`ifndef __ADDFN_V__
`define __ADDFN_V__

`include "addRecFN.v"

module addFN #(parameter expWidth = 8, parameter sigWidth = 24, parameter numWidth = 32) (
    input clk,
    input reset,
    input go,
    input [(`floatControlWidth - 1):0] control,
    input subOp,
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

    // Intermediate signals after the adder
    wire [(expWidth + sigWidth):0] res_recoded;
    wire [4:0] except_flag;

    // Compute recoded numbers
    addRecFN #(expWidth, sigWidth adder(
        .control(control),
        .subOp(subOp),
        .left(l_recoded),
        .right(r_recoded),
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

    // Dummy registers for storing results before output
    reg [(expWidth + sigWidth - 1):0] out_regs[0:1];
    reg [4:0] except_flag_regs[0:1];

    always_ff @(posedge clk) begin
        if (reset) begin
            out_regs[0] <= 0;
            out_regs[1] <= 0;
            except_flag_regs[0] <= 0;
            except_flag_regs[1] <= 0;
        end else begin
            // out
            out_regs[0] <= res_std;
            out_regs[1] <= out_regs[0];
            out <= out_regs[1];
            // exceptionFlags
            except_flag_regs[0] <= except_flag;
            except_flag_regs[1] <= except_flag_regs[0];
            exceptionFlags <= except_flag_regs[1];
        end
    end

    // 4-bit shift register for valid signal
    reg [3:0] valid_shift_reg = 4'b0000; 

    always @(posedge clk) begin
    if (reset) begin
        valid_shift_reg <= 4'b0000;
    end else begin
        valid_shift_reg <= {valid_shift_reg[2:0], go};
    end
    end

    assign done = valid_shift_reg[3];

endmodule


`endif /* __ADDFN_V__ */

