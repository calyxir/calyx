`include "sim/lib/std.v"
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
logic [31:0] const5_out;
logic [31:0] const2_out;
logic [31:0] const3_out;
logic [31:0] const4_out;
logic a0_ready;
logic fsm_enable_a0_const0_valid_a0;
logic const0_ready;
logic fsm_enable_a0_const0_valid_const0;
logic b0_ready;
logic fsm_enable_b0_const1_valid_b0;
logic const1_ready;
logic fsm_enable_b0_const1_valid_const1;
logic gt0_ready;
logic fsm_enable_gt0const5_const2_valid_gt0;
logic const5_ready;
logic fsm_enable_gt0const5_const2_valid_const5;
logic const2_ready;
logic fsm_enable_gt0const5_const2_valid_const2;
logic y0_ready;
logic fsm_enable_y0_const3_valid_y0;
logic const3_ready;
logic fsm_enable_y0_const3_valid_const3;
logic z0_ready;
logic fsm_enable_z0_const4_valid_z0;
logic const4_ready;
logic fsm_enable_z0_const4_valid_const4;
logic fsm_enable_y0_const3_ready;
logic fsm_if_0_valid_t_fsm_enable_y0_const3;
logic fsm_enable_z0_const4_ready;
logic fsm_if_0_valid_f_fsm_enable_z0_const4;
logic gt0_out;
logic fsm_enable_a0_const0_ready;
logic fsm_par_0_valid_fsm_enable_a0_const0;
logic fsm_enable_b0_const1_ready;
logic fsm_par_0_valid_fsm_enable_b0_const1;
logic fsm_par_0_ready;
logic fsm_seq_1_valid_fsm_par_0;
logic fsm_enable_gt0const5_const2_ready;
logic fsm_seq_1_valid_fsm_enable_gt0const5_const2;
logic fsm_if_0_ready;
logic fsm_seq_1_valid_fsm_if_0;

// Subcomponent Instances
fsm_enable_a0_const0 #() fsm_enable_a0_const0 (
    .valid(fsm_par_0_valid_fsm_enable_a0_const0),
    .ready_const0(const0_ready),
    .valid_const0(fsm_enable_a0_const0_valid_const0),
    .clk(clk),
    .reset(reset),
    .valid_a0(fsm_enable_a0_const0_valid_a0),
    .ready_a0(a0_ready),
    .ready(fsm_enable_a0_const0_ready)
);

fsm_enable_b0_const1 #() fsm_enable_b0_const1 (
    .valid_const1(fsm_enable_b0_const1_valid_const1),
    .valid(fsm_par_0_valid_fsm_enable_b0_const1),
    .reset(reset),
    .ready_b0(b0_ready),
    .ready_const1(const1_ready),
    .ready(fsm_enable_b0_const1_ready),
    .clk(clk),
    .valid_b0(fsm_enable_b0_const1_valid_b0)
);

fsm_enable_gt0const5_const2 #() fsm_enable_gt0const5_const2 (
    .ready_gt0(gt0_ready),
    .valid_const5(fsm_enable_gt0const5_const2_valid_const5),
    .ready_const5(const5_ready),
    .valid_gt0(fsm_enable_gt0const5_const2_valid_gt0),
    .ready(fsm_enable_gt0const5_const2_ready),
    .ready_const2(const2_ready),
    .reset(reset),
    .valid_const2(fsm_enable_gt0const5_const2_valid_const2),
    .valid(fsm_seq_1_valid_fsm_enable_gt0const5_const2),
    .clk(clk)
);

fsm_enable_y0_const3 #() fsm_enable_y0_const3 (
    .clk(clk),
    .ready_const3(const3_ready),
    .valid_const3(fsm_enable_y0_const3_valid_const3),
    .valid_y0(fsm_enable_y0_const3_valid_y0),
    .reset(reset),
    .valid(fsm_if_0_valid_t_fsm_enable_y0_const3),
    .ready_y0(y0_ready),
    .ready(fsm_enable_y0_const3_ready)
);

fsm_enable_z0_const4 #() fsm_enable_z0_const4 (
    .reset(reset),
    .valid_z0(fsm_enable_z0_const4_valid_z0),
    .ready(fsm_enable_z0_const4_ready),
    .valid(fsm_if_0_valid_f_fsm_enable_z0_const4),
    .valid_const4(fsm_enable_z0_const4_valid_const4),
    .ready_const4(const4_ready),
    .clk(clk),
    .ready_z0(z0_ready)
);

fsm_if_0 #() fsm_if_0 (
    .condition(gt0_out),
    .clk(clk),
    .valid_f_fsm_enable_z0_const4(fsm_if_0_valid_f_fsm_enable_z0_const4),
    .valid(fsm_seq_1_valid_fsm_if_0),
    .valid_t_fsm_enable_y0_const3(fsm_if_0_valid_t_fsm_enable_y0_const3),
    .ready_t_fsm_enable_y0_const3(fsm_enable_y0_const3_ready),
    .reset(reset),
    .ready(fsm_if_0_ready),
    .ready_f_fsm_enable_z0_const4(fsm_enable_z0_const4_ready)
);

fsm_par_0 #() fsm_par_0 (
    .ready(fsm_par_0_ready),
    .reset(reset),
    .ready_fsm_enable_a0_const0(fsm_enable_a0_const0_ready),
    .valid(fsm_seq_1_valid_fsm_par_0),
    .valid_fsm_enable_a0_const0(fsm_par_0_valid_fsm_enable_a0_const0),
    .valid_fsm_enable_b0_const1(fsm_par_0_valid_fsm_enable_b0_const1),
    .ready_fsm_enable_b0_const1(fsm_enable_b0_const1_ready),
    .clk(clk)
);

fsm_seq_1 #() fsm_seq_1 (
    .ready_fsm_par_0(fsm_par_0_ready),
    .reset(reset),
    .valid_fsm_enable_gt0const5_const2(fsm_seq_1_valid_fsm_enable_gt0const5_const2),
    .valid_fsm_if_0(fsm_seq_1_valid_fsm_if_0),
    .clk(clk),
    .valid(valid),
    .ready(),
    .valid_fsm_par_0(fsm_seq_1_valid_fsm_par_0),
    .ready_fsm_enable_gt0const5_const2(fsm_enable_gt0const5_const2_ready),
    .ready_fsm_if_0(fsm_if_0_ready)
);
std_reg #(32, 0) a0 (
    .valid(fsm_enable_a0_const0_valid_a0),
    .clk(clk),
    .out(),
    .ready(a0_ready),
    .in(const0_out),
    .reset()
);

std_const #(32, 0) const0 (
    .out(const0_out),
    .ready(const0_ready),
    .valid(fsm_enable_a0_const0_valid_const0),
    .reset()
);

std_reg #(32, 0) b0 (
    .clk(clk),
    .ready(b0_ready),
    .valid(fsm_enable_b0_const1_valid_b0),
    .reset(),
    .in(const1_out),
    .out()
);

std_const #(32, 1) const1 (
    .valid(fsm_enable_b0_const1_valid_const1),
    .out(const1_out),
    .ready(const1_ready),
    .reset()
);

std_gt #(32) gt0 (
    .right(const2_out),
    .ready(gt0_ready),
    .valid(fsm_enable_gt0const5_const2_valid_gt0),
    .left(const5_out),
    .out(gt0_out),
    .reset()
);

std_const #(32, 1) const2 (
    .out(const2_out),
    .ready(const2_ready),
    .valid(fsm_enable_gt0const5_const2_valid_const2),
    .reset()
);

std_reg #(32, 0) y0 (
    .in(const3_out),
    .reset(),
    .clk(clk),
    .ready(y0_ready),
    .out(),
    .valid(fsm_enable_y0_const3_valid_y0)
);

std_const #(32, 2) const3 (
    .reset(),
    .valid(fsm_enable_y0_const3_valid_const3),
    .out(const3_out),
    .ready(const3_ready)
);

std_reg #(32, 0) z0 (
    .ready(z0_ready),
    .valid(fsm_enable_z0_const4_valid_z0),
    .out(),
    .reset(),
    .in(const4_out),
    .clk(clk)
);

std_const #(32, 4) const4 (
    .ready(const4_ready),
    .reset(),
    .out(const4_out),
    .valid(fsm_enable_z0_const4_valid_const4)
);

std_const #(32, 2) const5 (
    .out(const5_out),
    .reset(),
    .ready(const5_ready),
    .valid(fsm_enable_gt0const5_const2_valid_const5)
);

endmodule
module fsm_enable_a0_const0 (
    input logic valid,
    input logic reset,
    input logic ready_a0,
    input logic ready_const0,
    input logic clk,
    output logic ready,
    output logic valid_a0,
    output logic valid_const0
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
                        next_state = 2'd1;
                    else
                        next_state = 2'd0;
                end
                2'd2: begin
                    if ( reset == 1'd1 )
                        next_state = 2'd0;
                    else
                        next_state = 2'd2;
                end
                2'd1: begin
                    if ( ready_a0 == 1'd1 && ready_const0 == 1'd1 )
                        next_state = 2'd2;
                    else
                        next_state = 2'd1;
                end
            
            default: 
        next_state = 2'd0;
    endcase
end
always_comb begin
    case (state)
        2'd0: begin
            ready = 1'd0;
            valid_a0 = 1'd0;
            valid_const0 = 1'd0;
        end
        2'd2: begin
            ready = 1'd1;
            valid_a0 = 1'd0;
            valid_const0 = 1'd0;
        end
        2'd1: begin
            valid_a0 = 1'd1;
            valid_const0 = 1'd1;
            ready = 1'd0;
        end
    
        default: begin
            ready = 1'd0;
            valid_a0 = 1'd0;
            valid_const0 = 1'd0;
        end
        endcase
end
endmodule

module fsm_enable_b0_const1 (
    input logic valid,
    input logic reset,
    input logic ready_b0,
    input logic ready_const1,
    input logic clk,
    output logic ready,
    output logic valid_b0,
    output logic valid_const1
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
                        next_state = 2'd1;
                    else
                        next_state = 2'd0;
                end
                2'd1: begin
                    if ( ready_b0 == 1'd1 && ready_const1 == 1'd1 )
                        next_state = 2'd2;
                    else
                        next_state = 2'd1;
                end
                2'd2: begin
                    if ( reset == 1'd1 )
                        next_state = 2'd0;
                    else
                        next_state = 2'd2;
                end
            
            default: 
        next_state = 2'd0;
    endcase
end
always_comb begin
    case (state)
        2'd0: begin
            valid_b0 = 1'd0;
            valid_const1 = 1'd0;
            ready = 1'd0;
        end
        2'd1: begin
            valid_b0 = 1'd1;
            valid_const1 = 1'd1;
            ready = 1'd0;
        end
        2'd2: begin
            ready = 1'd1;
            valid_b0 = 1'd0;
            valid_const1 = 1'd0;
        end
    
        default: begin
            valid_b0 = 1'd0;
            valid_const1 = 1'd0;
            ready = 1'd0;
        end
        endcase
end
endmodule

module fsm_enable_gt0const5_const2 (
    input logic valid,
    input logic reset,
    input logic ready_gt0,
    input logic ready_const5,
    input logic ready_const2,
    input logic clk,
    output logic ready,
    output logic valid_gt0,
    output logic valid_const5,
    output logic valid_const2
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
                        next_state = 2'd1;
                    else
                        next_state = 2'd0;
                end
                2'd1: begin
                    if ( ready_gt0 == 1'd1 && ready_const5 == 1'd1
                    && ready_const2 == 1'd1 )
                        next_state = 2'd2;
                    else
                        next_state = 2'd1;
                end
                2'd2: begin
                    if ( reset == 1'd1 )
                        next_state = 2'd0;
                    else
                        next_state = 2'd2;
                end
            
            default: 
        next_state = 2'd0;
    endcase
end
always_comb begin
    case (state)
        2'd0: begin
            valid_gt0 = 1'd0;
            valid_const5 = 1'd0;
            valid_const2 = 1'd0;
            ready = 1'd0;
        end
        2'd1: begin
            valid_gt0 = 1'd1;
            valid_const5 = 1'd1;
            valid_const2 = 1'd1;
            ready = 1'd0;
        end
        2'd2: begin
            ready = 1'd1;
            valid_gt0 = 1'd0;
            valid_const5 = 1'd0;
            valid_const2 = 1'd0;
        end
    
        default: begin
            valid_gt0 = 1'd0;
            valid_const5 = 1'd0;
            valid_const2 = 1'd0;
            ready = 1'd0;
        end
        endcase
end
endmodule

module fsm_enable_y0_const3 (
    input logic valid,
    input logic reset,
    input logic ready_y0,
    input logic ready_const3,
    input logic clk,
    output logic ready,
    output logic valid_y0,
    output logic valid_const3
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
                2'd2: begin
                    if ( reset == 1'd1 )
                        next_state = 2'd0;
                    else
                        next_state = 2'd2;
                end
                2'd1: begin
                    if ( ready_y0 == 1'd1 && ready_const3 == 1'd1 )
                        next_state = 2'd2;
                    else
                        next_state = 2'd1;
                end
                2'd0: begin
                    if ( valid == 1'd1 )
                        next_state = 2'd1;
                    else
                        next_state = 2'd0;
                end
            
            default: 
        next_state = 2'd0;
    endcase
end
always_comb begin
    case (state)
        2'd2: begin
            ready = 1'd1;
            valid_y0 = 1'd0;
            valid_const3 = 1'd0;
        end
        2'd1: begin
            valid_y0 = 1'd1;
            valid_const3 = 1'd1;
            ready = 1'd0;
        end
        2'd0: begin
            ready = 1'd0;
            valid_y0 = 1'd0;
            valid_const3 = 1'd0;
        end
    
        default: begin
            ready = 1'd0;
            valid_y0 = 1'd0;
            valid_const3 = 1'd0;
        end
        endcase
end
endmodule

module fsm_enable_z0_const4 (
    input logic valid,
    input logic reset,
    input logic ready_z0,
    input logic ready_const4,
    input logic clk,
    output logic ready,
    output logic valid_z0,
    output logic valid_const4
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
                        next_state = 2'd1;
                    else
                        next_state = 2'd0;
                end
                2'd2: begin
                    if ( reset == 1'd1 )
                        next_state = 2'd0;
                    else
                        next_state = 2'd2;
                end
                2'd1: begin
                    if ( ready_z0 == 1'd1 && ready_const4 == 1'd1 )
                        next_state = 2'd2;
                    else
                        next_state = 2'd1;
                end
            
            default: 
        next_state = 2'd0;
    endcase
end
always_comb begin
    case (state)
        2'd0: begin
            ready = 1'd0;
            valid_z0 = 1'd0;
            valid_const4 = 1'd0;
        end
        2'd2: begin
            ready = 1'd1;
            valid_z0 = 1'd0;
            valid_const4 = 1'd0;
        end
        2'd1: begin
            valid_z0 = 1'd1;
            valid_const4 = 1'd1;
            ready = 1'd0;
        end
    
        default: begin
            ready = 1'd0;
            valid_z0 = 1'd0;
            valid_const4 = 1'd0;
        end
        endcase
end
endmodule

module fsm_if_0 (
    input logic condition,
    input logic valid,
    input logic reset,
    input logic ready_t_fsm_enable_y0_const3,
    input logic ready_f_fsm_enable_z0_const4,
    input logic clk,
    output logic ready,
    output logic valid_t_fsm_enable_y0_const3,
    output logic valid_f_fsm_enable_z0_const4
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
                2'd2: begin
                    if ( ready_f_fsm_enable_z0_const4 == 1'd1 )
                        next_state = 2'd1;
                    else
                        next_state = 2'd2;
                end
                2'd1: begin
                    if ( reset == 1'd1 )
                        next_state = 2'd0;
                    else
                        next_state = 2'd1;
                end
                2'd3: begin
                    if ( ready_t_fsm_enable_y0_const3 == 1'd1 )
                        next_state = 2'd1;
                    else
                        next_state = 2'd3;
                end
                2'd0: begin
                    if ( valid == 1'd1 && condition == 1'd0 )
                        next_state = 2'd2;
                    else if ( valid == 1'd1 && condition == 1'd1 )
                        next_state = 2'd3;
                    else
                        next_state = 2'd0;
                end
            
            default: 
        next_state = 2'd0;
    endcase
end
always_comb begin
    case (state)
        2'd2: begin
            valid_f_fsm_enable_z0_const4 = 1'd1;
            ready = 1'd0;
            valid_t_fsm_enable_y0_const3 = 1'd0;
        end
        2'd1: begin
            ready = 1'd1;
            valid_f_fsm_enable_z0_const4 = 1'd0;
            valid_t_fsm_enable_y0_const3 = 1'd0;
        end
        2'd3: begin
            valid_t_fsm_enable_y0_const3 = 1'd1;
            valid_f_fsm_enable_z0_const4 = 1'd0;
            ready = 1'd0;
        end
        2'd0: begin
            valid_f_fsm_enable_z0_const4 = 1'd0;
            ready = 1'd0;
            valid_t_fsm_enable_y0_const3 = 1'd0;
        end
    
        default: begin
            valid_f_fsm_enable_z0_const4 = 1'd0;
            ready = 1'd0;
            valid_t_fsm_enable_y0_const3 = 1'd0;
        end
        endcase
end
endmodule

module fsm_par_0 (
    input logic valid,
    input logic reset,
    input logic ready_fsm_enable_a0_const0,
    input logic ready_fsm_enable_b0_const1,
    input logic clk,
    output logic ready,
    output logic valid_fsm_enable_a0_const0,
    output logic valid_fsm_enable_b0_const1
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
                    if ( ready_fsm_enable_a0_const0 == 1'd1
                    && ready_fsm_enable_b0_const1 == 1'd1 )
                        next_state = 2'd2;
                    else
                        next_state = 2'd1;
                end
                2'd0: begin
                    if ( valid == 1'd1 )
                        next_state = 2'd1;
                    else
                        next_state = 2'd0;
                end
                2'd2: begin
                    if ( reset == 1'd1 )
                        next_state = 2'd0;
                    else
                        next_state = 2'd2;
                end
            
            default: 
        next_state = 2'd0;
    endcase
end
always_comb begin
    case (state)
        2'd1: begin
            valid_fsm_enable_a0_const0 = 1'd1;
            valid_fsm_enable_b0_const1 = 1'd1;
            ready = 1'd0;
        end
        2'd0: begin
            valid_fsm_enable_a0_const0 = 1'd0;
            valid_fsm_enable_b0_const1 = 1'd0;
            ready = 1'd0;
        end
        2'd2: begin
            ready = 1'd1;
            valid_fsm_enable_a0_const0 = 1'd0;
            valid_fsm_enable_b0_const1 = 1'd0;
        end
    
        default: begin
            valid_fsm_enable_a0_const0 = 1'd0;
            valid_fsm_enable_b0_const1 = 1'd0;
            ready = 1'd0;
        end
        endcase
end
endmodule

module fsm_seq_1 (
    input logic valid,
    input logic reset,
    input logic ready_fsm_par_0,
    input logic ready_fsm_enable_gt0const5_const2,
    input logic ready_fsm_if_0,
    input logic clk,
    output logic ready,
    output logic valid_fsm_par_0,
    output logic valid_fsm_enable_gt0const5_const2,
    output logic valid_fsm_if_0
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
                3'd3: begin
                    if ( ready_fsm_if_0 == 1'd1 )
                        next_state = 3'd4;
                    else
                        next_state = 3'd3;
                end
                3'd0: begin
                    if ( valid == 1'd1 )
                        next_state = 3'd1;
                    else
                        next_state = 3'd0;
                end
                3'd4: begin
                    if ( reset == 1'd1 )
                        next_state = 3'd0;
                    else
                        next_state = 3'd4;
                end
                3'd1: begin
                    if ( ready_fsm_par_0 == 1'd1 )
                        next_state = 3'd2;
                    else
                        next_state = 3'd1;
                end
                3'd2: begin
                    if ( ready_fsm_enable_gt0const5_const2 == 1'd1 )
                        next_state = 3'd3;
                    else
                        next_state = 3'd2;
                end
            
            default: 
        next_state = 3'd0;
    endcase
end
always_comb begin
    case (state)
        3'd3: begin
            valid_fsm_if_0 = 1'd1;
            ready = 1'd0;
            valid_fsm_par_0 = 1'd0;
            valid_fsm_enable_gt0const5_const2 = 1'd0;
        end
        3'd0: begin
            valid_fsm_if_0 = 1'd0;
            ready = 1'd0;
            valid_fsm_par_0 = 1'd0;
            valid_fsm_enable_gt0const5_const2 = 1'd0;
        end
        3'd4: begin
            ready = 1'd1;
            valid_fsm_if_0 = 1'd0;
            valid_fsm_par_0 = 1'd0;
            valid_fsm_enable_gt0const5_const2 = 1'd0;
        end
        3'd1: begin
            valid_fsm_par_0 = 1'd1;
            valid_fsm_if_0 = 1'd0;
            ready = 1'd0;
            valid_fsm_enable_gt0const5_const2 = 1'd0;
        end
        3'd2: begin
            valid_fsm_enable_gt0const5_const2 = 1'd1;
            valid_fsm_if_0 = 1'd0;
            ready = 1'd0;
            valid_fsm_par_0 = 1'd0;
        end
    
        default: begin
            valid_fsm_if_0 = 1'd0;
            ready = 1'd0;
            valid_fsm_par_0 = 1'd0;
            valid_fsm_enable_gt0const5_const2 = 1'd0;
        end
        endcase
end
endmodule

