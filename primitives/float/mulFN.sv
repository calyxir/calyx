`ifndef __MULFN_V__
`define __MULFN_V__

`include "mulRecFN.v"

module mulFN #(parameter expWidth = 8, parameter sigWidth = 24, parameter numWidth = 32) (
    input clk,
    input reset,
    input val,
    input [(`floatControlWidth - 1):0] control,
    input [(expWidth + sigWidth - 1):0] a,
    input [(expWidth + sigWidth - 1):0] b,
    input [2:0] roundingMode,
    output logic [(expWidth + sigWidth - 1):0] out,
    output logic [4:0] exceptionFlags,
    output done
);

    // Intermediate signals for recoded formats
    wire [(expWidth + sigWidth):0] a_recoded, b_recoded;

    // Convert 'a' and 'b' from standard to recoded format
    fNToRecFN #(expWidth, sigWidth) convert_a(
        .in(a),
        .out(a_recoded)
    );

    fNToRecFN #(expWidth, sigWidth) convert_b(
        .in(b),
        .out(b_recoded)
    );

    // Intermediate signals after the multiplier
    wire [(expWidth + sigWidth):0] res_recoded;
    wire [4:0] except_flag;

    // Compute recoded numbers
    mulRecFN #(expWidth, sigWidth) multiplier(
        .control(control),
        .a(a_recoded),
        .b(b_recoded),
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
        valid_shift_reg <= {valid_shift_reg[2:0], val};
    end
    end

    assign done = valid_shift_reg[3];

endmodule


`endif /* __MULFN_V__ */
