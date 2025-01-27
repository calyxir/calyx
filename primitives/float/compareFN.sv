`ifndef __COMPAREFN_V__
`define __COMPAREFN_V__

module std_compareFN #(parameter expWidth = 8, parameter sigWidth = 24, parameter numWidth = 32) (
    input clk,
    input reset,
    input go,
    input [(expWidth + sigWidth - 1):0] left,
    input [(expWidth + sigWidth - 1):0] right,
    input signaling,
    output logic lt,
    output logic eq,
    output logic gt,
    output logic unordered,
    output logic [4:0] exceptionFlags,
    output done
);

    // Intermediate signals for recoded formats
    wire [(expWidth + sigWidth):0] l_recoded, r_recoded;

    // Convert 'left' and 'right' from standard to recoded format
    fNToRecFN #(expWidth, sigWidth) convert_l(
        .in(left),
        .out(l_recoded)
    );

    fNToRecFN #(expWidth, sigWidth) convert_r(
        .in(right),
        .out(r_recoded)
    );

    // Intermediate signals for comparison outputs
    wire comp_lt, comp_eq, comp_gt, comp_unordered;
    wire [4:0] comp_exceptionFlags;

    // Compare recoded numbers
    compareRecFN #(expWidth, sigWidth) comparator(
        .a(l_recoded),
        .b(r_recoded),
        .signaling(signaling),
        .lt(comp_lt),
        .eq(comp_eq),
        .gt(comp_gt),
        .unordered(comp_unordered),
        .exceptionFlags(comp_exceptionFlags)
    );

    logic done_buf[1:0];

    assign done = done_buf[1];

    // If the done buffer is empty and go is high, execution just started.
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

    // Capture the comparison results
    always_ff @(posedge clk) begin
        if (reset) begin
            lt <= 0;
            eq <= 0;
            gt <= 0;
            unordered <= 0;
            exceptionFlags <= 0;
        end else if (go) begin
            lt <= comp_lt;
            eq <= comp_eq;
            gt <= comp_gt;
            unordered <= comp_unordered;
            exceptionFlags <= comp_exceptionFlags;
        end else begin
            lt <= lt;
            eq <= eq;
            gt <= gt;
            unordered <= unordered;
            exceptionFlags <= exceptionFlags;
        end
    end

endmodule


`endif /* __COMPAREFN_V__ */
