// Component Signature
module main
(
    input logic valid,
    input logic reset,
    input logic clk,
    output logic ready
);

// Wire declarations
logic [31:0] const0_out;
logic [31:0] const1_out;
logic [31:0] const2_out;
logic a0_ready;
logic fsm_enable_a0_const0_valid_a0;
logic const0_ready;
logic fsm_enable_a0_const0_valid_const0;
logic b0_ready;
logic fsm_enable_b0_const1_valid_b0;
logic const1_ready;
logic fsm_enable_b0_const1_valid_const1;
logic c0_ready;
logic fsm_enable_c0_const2_valid_c0;
logic const2_ready;
logic fsm_enable_c0_const2_valid_const2;
logic fsm_enable_a0_const0_ready;
logic fsm_seq_0_valid_fsm_enable_a0_const0;
logic fsm_enable_b0_const1_ready;
logic fsm_seq_0_valid_fsm_enable_b0_const1;
logic fsm_enable_c0_const2_ready;
logic fsm_seq_0_valid_fsm_enable_c0_const2;

// Subcomponent Instances
fsm_enable_a0_const0 #() fsm_enable_a0_const0 (
    .clock(),
    .ready(fsm_enable_a0_const0_ready),
    .ready_a0(a0_ready),
    .valid_a0(fsm_enable_a0_const0_valid_a0),
    .valid(fsm_seq_0_valid_fsm_enable_a0_const0),
    .ready_const0(const0_ready),
    .reset(),
    .valid_const0(fsm_enable_a0_const0_valid_const0)
);

fsm_enable_b0_const1 #() fsm_enable_b0_const1 (
    .reset(),
    .valid_b0(fsm_enable_b0_const1_valid_b0),
    .valid_const1(fsm_enable_b0_const1_valid_const1),
    .clock(),
    .ready(fsm_enable_b0_const1_ready),
    .ready_b0(b0_ready),
    .valid(fsm_seq_0_valid_fsm_enable_b0_const1),
    .ready_const1(const1_ready)
);

fsm_enable_c0_const2 #() fsm_enable_c0_const2 (
    .reset(),
    .ready_const2(const2_ready),
    .valid(fsm_seq_0_valid_fsm_enable_c0_const2),
    .ready_c0(c0_ready),
    .valid_const2(fsm_enable_c0_const2_valid_const2),
    .valid_c0(fsm_enable_c0_const2_valid_c0),
    .clock(),
    .ready(fsm_enable_c0_const2_ready)
);

fsm_seq_0 #() fsm_seq_0 (
    .valid_fsm_enable_b0_const1(fsm_seq_0_valid_fsm_enable_b0_const1),
    .valid(),
    .ready_fsm_enable_b0_const1(fsm_enable_b0_const1_ready),
    .clk(),
    .reset(),
    .valid_fsm_enable_c0_const2(fsm_seq_0_valid_fsm_enable_c0_const2),
    .ready(),
    .valid_fsm_enable_a0_const0(fsm_seq_0_valid_fsm_enable_a0_const0),
    .ready_fsm_enable_c0_const2(fsm_enable_c0_const2_ready),
    .ready_fsm_enable_a0_const0(fsm_enable_a0_const0_ready)
);
register #(32, 0) a0 (
    .valid(fsm_enable_a0_const0_valid_a0),
    .out(),
    .ready(a0_ready),
    .in(const0_out),
    .reset()
);

c_const #(32, 0) const0 (
    .out(const0_out),
    .reset(),
    .valid(fsm_enable_a0_const0_valid_const0),
    .ready(const0_ready)
);

register #(32, 0) b0 (
    .out(),
    .valid(fsm_enable_b0_const1_valid_b0),
    .in(const1_out),
    .ready(b0_ready),
    .reset()
);

c_const #(32, 0) const1 (
    .out(const1_out),
    .ready(const1_ready),
    .valid(fsm_enable_b0_const1_valid_const1),
    .reset()
);

register #(32, 0) c0 (
    .reset(),
    .in(const2_out),
    .valid(fsm_enable_c0_const2_valid_c0),
    .ready(c0_ready),
    .out()
);

c_const #(32, 0) const2 (
    .valid(fsm_enable_c0_const2_valid_const2),
    .out(const2_out),
    .ready(const2_ready),
    .reset()
);

endmodule
module fsm_enable_a0_const0 (
        input  logic clk,
    input  logic valid,
    input  logic ready_a0,
    input  logic ready_const0,
    input  logic reset,
    output logic valid_a0,
    output logic valid_const0,
    output logic ready
);
logic [1:0] state, next_state;
always_ff @(posedge clk) begin
    if ( reset )
        state <= 2'd0; // 0 default state?
    else
        state <= next_state;
end
always_comb begin
    case (state)
        2'd0: begin
            if ( valid == 1'd1 )
                next_state = 2'd2;
            else
                next_state = 2'd0;
        end
        2'd1: begin
            if ( ready_a0 == 1'd1 && ready_const0 == 1'd1 )
                next_state = 2'd3;
            else
                next_state = 2'd1;
        end
        2'd2: begin
            if ( reset == 1'd1 )
                next_state = 2'd1;
            else
                next_state = 2'd2;
        end
    endcase
end
always_comb begin
    case (state)
        2'd0: begin
            valid_a0 = 1'd0;
            valid_const0 = 1'd0;
            ready = 1'd0;
        end
        2'd1: begin
            valid_a0 = 1'd1;
            valid_const0 = 1'd1;
            ready = 1'd0;
        end
        2'd2: begin
            ready = 1'd1;
            valid_a0 = 1'd0;
            valid_const0 = 1'd0;
        end
    endcase
end
endmodule

module fsm_enable_b0_const1 (
        input  logic clk,
    input  logic ready_b0,
    input  logic ready_const1,
    input  logic valid,
    input  logic reset,
    output logic valid_b0,
    output logic valid_const1,
    output logic ready
);
logic [1:0] state, next_state;
always_ff @(posedge clk) begin
    if ( reset )
        state <= 2'd0; // 0 default state?
    else
        state <= next_state;
end
always_comb begin
    case (state)
        2'd1: begin
            if ( ready_b0 == 1'd1 && ready_const1 == 1'd1 )
                next_state = 2'd3;
            else
                next_state = 2'd1;
        end
        2'd0: begin
            if ( valid == 1'd1 )
                next_state = 2'd2;
            else
                next_state = 2'd0;
        end
        2'd2: begin
            if ( reset == 1'd1 )
                next_state = 2'd1;
            else
                next_state = 2'd2;
        end
    endcase
end
always_comb begin
    case (state)
        2'd1: begin
            valid_b0 = 1'd1;
            valid_const1 = 1'd1;
            ready = 1'd0;
        end
        2'd0: begin
            valid_b0 = 1'd0;
            valid_const1 = 1'd0;
            ready = 1'd0;
        end
        2'd2: begin
            ready = 1'd1;
            valid_b0 = 1'd0;
            valid_const1 = 1'd0;
        end
    endcase
end
endmodule

module fsm_enable_c0_const2 (
        input  logic clk,
    input  logic ready_c0,
    input  logic ready_const2,
    input  logic valid,
    input  logic reset,
    output logic valid_c0,
    output logic valid_const2,
    output logic ready
);
logic [1:0] state, next_state;
always_ff @(posedge clk) begin
    if ( reset )
        state <= 2'd0; // 0 default state?
    else
        state <= next_state;
end
always_comb begin
    case (state)
        2'd1: begin
            if ( ready_c0 == 1'd1 && ready_const2 == 1'd1 )
                next_state = 2'd3;
            else
                next_state = 2'd1;
        end
        2'd0: begin
            if ( valid == 1'd1 )
                next_state = 2'd2;
            else
                next_state = 2'd0;
        end
        2'd2: begin
            if ( reset == 1'd1 )
                next_state = 2'd1;
            else
                next_state = 2'd2;
        end
    endcase
end
always_comb begin
    case (state)
        2'd1: begin
            valid_c0 = 1'd1;
            valid_const2 = 1'd1;
            ready = 1'd0;
        end
        2'd0: begin
            valid_c0 = 1'd0;
            valid_const2 = 1'd0;
            ready = 1'd0;
        end
        2'd2: begin
            ready = 1'd1;
            valid_c0 = 1'd0;
            valid_const2 = 1'd0;
        end
    endcase
end
endmodule

module fsm_seq_0 (
        input  logic clk,
    input  logic valid,
    input  logic ready_fsm_enable_b0_const1,
    input  logic ready_fsm_enable_c0_const2,
    input  logic ready_fsm_enable_a0_const0,
    input  logic reset,
    output logic valid_fsm_enable_b0_const1,
    output logic valid_fsm_enable_c0_const2,
    output logic valid_fsm_enable_a0_const0,
    output logic ready
);
logic [2:0] state, next_state;
always_ff @(posedge clk) begin
    if ( reset )
        state <= 3'd0; // 0 default state?
    else
        state <= next_state;
end
always_comb begin
    case (state)
        3'd0: begin
            if ( valid == 1'd1 )
                next_state = 3'd2;
            else
                next_state = 3'd0;
        end
        3'd2: begin
            if ( ready_fsm_enable_b0_const1 == 1'd1 )
                next_state = 3'd4;
            else
                next_state = 3'd2;
        end
        3'd3: begin
            if ( ready_fsm_enable_c0_const2 == 1'd1 )
                next_state = 3'd5;
            else
                next_state = 3'd3;
        end
        3'd1: begin
            if ( ready_fsm_enable_a0_const0 == 1'd1 )
                next_state = 3'd3;
            else
                next_state = 3'd1;
        end
        3'd4: begin
            if ( reset == 1'd1 )
                next_state = 3'd1;
            else
                next_state = 3'd4;
        end
    endcase
end
always_comb begin
    case (state)
        3'd0: begin
            valid_fsm_enable_b0_const1 = 1'd0;
            valid_fsm_enable_c0_const2 = 1'd0;
            valid_fsm_enable_a0_const0 = 1'd0;
            ready = 1'd0;
        end
        3'd2: begin
            valid_fsm_enable_b0_const1 = 1'd1;
            valid_fsm_enable_c0_const2 = 1'd0;
            valid_fsm_enable_a0_const0 = 1'd0;
            ready = 1'd0;
        end
        3'd3: begin
            valid_fsm_enable_c0_const2 = 1'd1;
            valid_fsm_enable_b0_const1 = 1'd0;
            valid_fsm_enable_a0_const0 = 1'd0;
            ready = 1'd0;
        end
        3'd1: begin
            valid_fsm_enable_a0_const0 = 1'd1;
            valid_fsm_enable_b0_const1 = 1'd0;
            valid_fsm_enable_c0_const2 = 1'd0;
            ready = 1'd0;
        end
        3'd4: begin
            ready = 1'd1;
            valid_fsm_enable_b0_const1 = 1'd0;
            valid_fsm_enable_c0_const2 = 1'd0;
            valid_fsm_enable_a0_const0 = 1'd0;
        end
    endcase
end
endmodule

