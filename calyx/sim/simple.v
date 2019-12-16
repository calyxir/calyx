`include "sim/lib/std.v"
// Component Signature
module main
(
    input logic valid,
    input logic clk,
    output logic ready
);

// Wire declarations
logic [31:0] const0_out;
logic [31:0] const1_out;
logic [31:0] a0_out;
logic [31:0] const2_out;
logic [31:0] const3_out;
logic [31:0] const4_out;
logic const0_out_read_out;
logic const1_out_read_out;
logic a0_out_read_out;
logic const2_out_read_out;
logic const3_out_read_out;
logic const4_out_read_out;
logic const0_ready;
logic fsm_enable_a0_const0_valid_const0;
logic b0_ready;
logic fsm_enable_b0_const1_valid_b0;
logic const1_ready;
logic fsm_enable_b0_const1_valid_const1;
logic gt0_ready;
logic fsm_enable_gt0a0_const2_valid_gt0;
logic a0_ready;
logic const2_ready;
logic fsm_enable_gt0a0_const2_valid_const2;
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
logic fsm_enable_gt0a0_const2_ready;
logic fsm_par_1_valid_fsm_enable_gt0a0_const2;
logic fsm_if_0_ready;
logic fsm_par_1_valid_fsm_if_0;
logic fsm_par_0_ready;
logic fsm_seq_1_valid_fsm_par_0;
logic fsm_par_1_ready;
logic fsm_seq_1_valid_fsm_par_1;
logic fsm_enable_a0_const0_valid_a0;
logic fsm_enable_gt0a0_const2_valid_a0;
logic lut_control_0_valid;

// Subcomponent Instances
fsm_enable_a0_const0 #() fsm_enable_a0_const0 (
    .valid_const0(fsm_enable_a0_const0_valid_const0),
    .ready_a0(a0_ready),
    .clk(clk),
    .ready_const0(const0_ready),
    .ready(fsm_enable_a0_const0_ready),
    .valid(fsm_par_0_valid_fsm_enable_a0_const0),
    .valid_a0(fsm_enable_a0_const0_valid_a0)
);

fsm_enable_b0_const1 #() fsm_enable_b0_const1 (
    .valid_b0(fsm_enable_b0_const1_valid_b0),
    .clk(clk),
    .valid_const1(fsm_enable_b0_const1_valid_const1),
    .ready_const1(const1_ready),
    .ready_b0(b0_ready),
    .ready(fsm_enable_b0_const1_ready),
    .valid(fsm_par_0_valid_fsm_enable_b0_const1)
);

fsm_enable_gt0a0_const2 #() fsm_enable_gt0a0_const2 (
    .valid_const2(fsm_enable_gt0a0_const2_valid_const2),
    .clk(clk),
    .valid_gt0(fsm_enable_gt0a0_const2_valid_gt0),
    .ready_gt0(gt0_ready),
    .valid_a0(fsm_enable_gt0a0_const2_valid_a0),
    .ready_const2(const2_ready),
    .valid(fsm_par_1_valid_fsm_enable_gt0a0_const2),
    .ready_a0(a0_ready),
    .ready(fsm_enable_gt0a0_const2_ready)
);

fsm_enable_y0_const3 #() fsm_enable_y0_const3 (
    .ready(fsm_enable_y0_const3_ready),
    .valid_y0(fsm_enable_y0_const3_valid_y0),
    .ready_const3(const3_ready),
    .ready_y0(y0_ready),
    .valid_const3(fsm_enable_y0_const3_valid_const3),
    .valid(fsm_if_0_valid_t_fsm_enable_y0_const3),
    .clk(clk)
);

fsm_enable_z0_const4 #() fsm_enable_z0_const4 (
    .ready(fsm_enable_z0_const4_ready),
    .valid(fsm_if_0_valid_f_fsm_enable_z0_const4),
    .clk(clk),
    .ready_const4(const4_ready),
    .ready_z0(z0_ready),
    .valid_z0(fsm_enable_z0_const4_valid_z0),
    .valid_const4(fsm_enable_z0_const4_valid_const4)
);

fsm_if_0 #() fsm_if_0 (
    .ready_f_fsm_enable_z0_const4(fsm_enable_z0_const4_ready),
    .valid(fsm_par_1_valid_fsm_if_0),
    .ready(fsm_if_0_ready),
    .condition(gt0_out),
    .valid_t_fsm_enable_y0_const3(fsm_if_0_valid_t_fsm_enable_y0_const3),
    .ready_t_fsm_enable_y0_const3(fsm_enable_y0_const3_ready),
    .valid_f_fsm_enable_z0_const4(fsm_if_0_valid_f_fsm_enable_z0_const4),
    .clk(clk)
);

fsm_par_0 #() fsm_par_0 (
    .clk(clk),
    .valid_fsm_enable_a0_const0(fsm_par_0_valid_fsm_enable_a0_const0),
    .valid_fsm_enable_b0_const1(fsm_par_0_valid_fsm_enable_b0_const1),
    .ready_fsm_enable_b0_const1(fsm_enable_b0_const1_ready),
    .ready(fsm_par_0_ready),
    .ready_fsm_enable_a0_const0(fsm_enable_a0_const0_ready),
    .valid(fsm_seq_1_valid_fsm_par_0)
);

fsm_par_1 #() fsm_par_1 (
    .valid_fsm_enable_gt0a0_const2(fsm_par_1_valid_fsm_enable_gt0a0_const2),
    .ready_fsm_if_0(fsm_if_0_ready),
    .clk(clk),
    .ready_fsm_enable_gt0a0_const2(fsm_enable_gt0a0_const2_ready),
    .valid_fsm_if_0(fsm_par_1_valid_fsm_if_0),
    .ready(fsm_par_1_ready),
    .valid(fsm_seq_1_valid_fsm_par_1)
);

fsm_seq_1 #() fsm_seq_1 (
    .valid_fsm_par_1(fsm_seq_1_valid_fsm_par_1),
    .valid_fsm_par_0(fsm_seq_1_valid_fsm_par_0),
    .ready(),
    .valid(valid),
    .ready_fsm_par_0(fsm_par_0_ready),
    .clk(clk),
    .ready_fsm_par_1(fsm_par_1_ready)
);

lut_control_0 #() lut_control_0 (
    .valid0(fsm_enable_a0_const0_valid_a0),
    .valid(lut_control_0_valid),
    .valid1(fsm_enable_gt0a0_const2_valid_a0)
);
std_reg #(32, 0) a0 (
    .in_read_in(const0_out_read_out),
    .out_read_out(a0_out_read_out),
    .in(const0_out),
    .clk(clk),
    .ready(a0_ready),
    .out(a0_out),
    .valid(lut_control_0_valid)
);

std_const #(32, 3) const0 (
    .valid(fsm_enable_a0_const0_valid_const0),
    .out(const0_out),
    .ready(const0_ready),
    .out_read_out(const0_out_read_out)
);

std_reg #(32, 0) b0 (
    .in(const1_out),
    .clk(clk),
    .out(),
    .out_read_out(),
    .ready(b0_ready),
    .valid(fsm_enable_b0_const1_valid_b0),
    .in_read_in(const1_out_read_out)
);

std_const #(32, 1) const1 (
    .valid(fsm_enable_b0_const1_valid_const1),
    .out_read_out(const1_out_read_out),
    .out(const1_out),
    .ready(const1_ready)
);

std_gt #(32) gt0 (
    .out(gt0_out),
    .out_read_out(),
    .left(a0_out),
    .left_read_in(a0_out_read_out),
    .right(const2_out),
    .ready(gt0_ready),
    .valid(fsm_enable_gt0a0_const2_valid_gt0),
    .right_read_in(const2_out_read_out)
);

std_const #(32, 1) const2 (
    .out_read_out(const2_out_read_out),
    .ready(const2_ready),
    .valid(fsm_enable_gt0a0_const2_valid_const2),
    .out(const2_out)
);

std_reg #(32, 0) y0 (
    .out_read_out(),
    .in_read_in(const3_out_read_out),
    .ready(y0_ready),
    .in(const3_out),
    .clk(clk),
    .valid(fsm_enable_y0_const3_valid_y0),
    .out()
);

std_const #(32, 2) const3 (
    .ready(const3_ready),
    .valid(fsm_enable_y0_const3_valid_const3),
    .out(const3_out),
    .out_read_out(const3_out_read_out)
);

std_reg #(32, 0) z0 (
    .in(const4_out),
    .in_read_in(const4_out_read_out),
    .ready(z0_ready),
    .valid(fsm_enable_z0_const4_valid_z0),
    .clk(clk),
    .out_read_out(),
    .out()
);

std_const #(32, 4) const4 (
    .out(const4_out),
    .ready(const4_ready),
    .out_read_out(const4_out_read_out),
    .valid(fsm_enable_z0_const4_valid_const4)
);

endmodule
module lut_control_0 (
    input logic valid0,
    input logic valid1,
    output logic valid
);
always_comb begin
    case ({valid0, valid1})
            2'b10: valid = 1;
            2'b01: valid = 1;
        
        default: valid= 0;endcase
end
endmodule

module fsm_enable_a0_const0 (
    input logic valid,
    input logic ready_a0,
    input logic ready_const0,
    input logic clk,
    output logic ready,
    output logic valid_a0,
    output logic valid_const0
);
logic [1:0] state, next_state;
always_ff @(posedge clk) begin
    state <= next_state;
end
always_comb begin
    case (state)
                2'd2: begin
                    if ( valid == 1'd0 )
                        next_state = 2'd0;
                    else
                        next_state = 2'd2;
                end
                2'd0: begin
                    if ( valid == 1'd1 )
                        next_state = 2'd1;
                    else
                        next_state = 2'd0;
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
        2'd2: begin
            ready = 1'd1;
            valid_a0 = 1'd0;
            valid_const0 = 1'd0;
        end
        2'd0: begin
            ready = 1'd0;
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
    input logic ready_b0,
    input logic ready_const1,
    input logic clk,
    output logic ready,
    output logic valid_b0,
    output logic valid_const1
);
logic [1:0] state, next_state;
always_ff @(posedge clk) begin
    state <= next_state;
end
always_comb begin
    case (state)
                2'd2: begin
                    if ( valid == 1'd0 )
                        next_state = 2'd0;
                    else
                        next_state = 2'd2;
                end
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
            
            default: 
        next_state = 2'd0;
    endcase
end
always_comb begin
    case (state)
        2'd2: begin
            ready = 1'd1;
            valid_b0 = 1'd0;
            valid_const1 = 1'd0;
        end
        2'd0: begin
            ready = 1'd0;
            valid_b0 = 1'd0;
            valid_const1 = 1'd0;
        end
        2'd1: begin
            valid_b0 = 1'd1;
            valid_const1 = 1'd1;
            ready = 1'd0;
        end
    
        default: begin
            ready = 1'd0;
            valid_b0 = 1'd0;
            valid_const1 = 1'd0;
        end
        endcase
end
endmodule

module fsm_enable_gt0a0_const2 (
    input logic valid,
    input logic ready_gt0,
    input logic ready_a0,
    input logic ready_const2,
    input logic clk,
    output logic ready,
    output logic valid_gt0,
    output logic valid_a0,
    output logic valid_const2
);
logic [1:0] state, next_state;
always_ff @(posedge clk) begin
    state <= next_state;
end
always_comb begin
    case (state)
                2'd1: begin
                    if ( ready_gt0 == 1'd1 && ready_a0 == 1'd1
                    && ready_const2 == 1'd1 )
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
                    if ( valid == 1'd0 )
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
            valid_gt0 = 1'd1;
            valid_a0 = 1'd1;
            valid_const2 = 1'd1;
            ready = 1'd0;
        end
        2'd0: begin
            valid_gt0 = 1'd0;
            valid_a0 = 1'd0;
            valid_const2 = 1'd0;
            ready = 1'd0;
        end
        2'd2: begin
            ready = 1'd1;
            valid_gt0 = 1'd0;
            valid_a0 = 1'd0;
            valid_const2 = 1'd0;
        end
    
        default: begin
            valid_gt0 = 1'd0;
            valid_a0 = 1'd0;
            valid_const2 = 1'd0;
            ready = 1'd0;
        end
        endcase
end
endmodule

module fsm_enable_y0_const3 (
    input logic valid,
    input logic ready_y0,
    input logic ready_const3,
    input logic clk,
    output logic ready,
    output logic valid_y0,
    output logic valid_const3
);
logic [1:0] state, next_state;
always_ff @(posedge clk) begin
    state <= next_state;
end
always_comb begin
    case (state)
                2'd2: begin
                    if ( valid == 1'd0 )
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
    input logic ready_z0,
    input logic ready_const4,
    input logic clk,
    output logic ready,
    output logic valid_z0,
    output logic valid_const4
);
logic [1:0] state, next_state;
always_ff @(posedge clk) begin
    state <= next_state;
end
always_comb begin
    case (state)
                2'd2: begin
                    if ( valid == 1'd0 )
                        next_state = 2'd0;
                    else
                        next_state = 2'd2;
                end
                2'd0: begin
                    if ( valid == 1'd1 )
                        next_state = 2'd1;
                    else
                        next_state = 2'd0;
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
        2'd2: begin
            ready = 1'd1;
            valid_z0 = 1'd0;
            valid_const4 = 1'd0;
        end
        2'd0: begin
            ready = 1'd0;
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
    input logic ready_t_fsm_enable_y0_const3,
    input logic ready_f_fsm_enable_z0_const4,
    input logic clk,
    output logic ready,
    output logic valid_t_fsm_enable_y0_const3,
    output logic valid_f_fsm_enable_z0_const4
);
logic [1:0] state, next_state;
always_ff @(posedge clk) begin
    state <= next_state;
end
always_comb begin
    case (state)
                2'd0: begin
                    if ( valid == 1'd1 && condition == 1'd0 )
                        next_state = 2'd2;
                    else if ( valid == 1'd1 && condition == 1'd1 )
                        next_state = 2'd3;
                    else
                        next_state = 2'd0;
                end
                2'd2: begin
                    if ( ready_f_fsm_enable_z0_const4 == 1'd1 )
                        next_state = 2'd1;
                    else
                        next_state = 2'd2;
                end
                2'd1: begin
                    if ( valid == 1'd0 )
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
            
            default: 
        next_state = 2'd0;
    endcase
end
always_comb begin
    case (state)
        2'd0: begin
            valid_f_fsm_enable_z0_const4 = 1'd0;
            ready = 1'd0;
            valid_t_fsm_enable_y0_const3 = 1'd0;
        end
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
    input logic ready_fsm_enable_a0_const0,
    input logic ready_fsm_enable_b0_const1,
    input logic clk,
    output logic ready,
    output logic valid_fsm_enable_a0_const0,
    output logic valid_fsm_enable_b0_const1
);
logic [1:0] state, next_state;
always_ff @(posedge clk) begin
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
                    if ( valid == 1'd0 )
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

module fsm_par_1 (
    input logic valid,
    input logic ready_fsm_enable_gt0a0_const2,
    input logic ready_fsm_if_0,
    input logic clk,
    output logic ready,
    output logic valid_fsm_enable_gt0a0_const2,
    output logic valid_fsm_if_0
);
logic [1:0] state, next_state;
always_ff @(posedge clk) begin
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
                    if ( valid == 1'd0 )
                        next_state = 2'd0;
                    else
                        next_state = 2'd2;
                end
                2'd1: begin
                    if ( ready_fsm_enable_gt0a0_const2 == 1'd1
                    && ready_fsm_if_0 == 1'd1 )
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
            valid_fsm_enable_gt0a0_const2 = 1'd0;
            valid_fsm_if_0 = 1'd0;
        end
        2'd2: begin
            ready = 1'd1;
            valid_fsm_enable_gt0a0_const2 = 1'd0;
            valid_fsm_if_0 = 1'd0;
        end
        2'd1: begin
            valid_fsm_enable_gt0a0_const2 = 1'd1;
            valid_fsm_if_0 = 1'd1;
            ready = 1'd0;
        end
    
        default: begin
            ready = 1'd0;
            valid_fsm_enable_gt0a0_const2 = 1'd0;
            valid_fsm_if_0 = 1'd0;
        end
        endcase
end
endmodule

module fsm_seq_1 (
    input logic valid,
    input logic ready_fsm_par_0,
    input logic ready_fsm_par_1,
    input logic clk,
    output logic ready,
    output logic valid_fsm_par_0,
    output logic valid_fsm_par_1
);
logic [1:0] state, next_state;
always_ff @(posedge clk) begin
    state <= next_state;
end
always_comb begin
    case (state)
                2'd3: begin
                    if ( valid == 1'd0 )
                        next_state = 2'd0;
                    else
                        next_state = 2'd3;
                end
                2'd2: begin
                    if ( ready_fsm_par_1 == 1'd1 )
                        next_state = 2'd3;
                    else
                        next_state = 2'd2;
                end
                2'd0: begin
                    if ( valid == 1'd1 )
                        next_state = 2'd1;
                    else
                        next_state = 2'd0;
                end
                2'd1: begin
                    if ( ready_fsm_par_0 == 1'd1 )
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
        2'd3: begin
            ready = 1'd1;
            valid_fsm_par_1 = 1'd0;
            valid_fsm_par_0 = 1'd0;
        end
        2'd2: begin
            valid_fsm_par_1 = 1'd1;
            ready = 1'd0;
            valid_fsm_par_0 = 1'd0;
        end
        2'd0: begin
            ready = 1'd0;
            valid_fsm_par_1 = 1'd0;
            valid_fsm_par_0 = 1'd0;
        end
        2'd1: begin
            valid_fsm_par_0 = 1'd1;
            ready = 1'd0;
            valid_fsm_par_1 = 1'd0;
        end
    
        default: begin
            ready = 1'd0;
            valid_fsm_par_1 = 1'd0;
            valid_fsm_par_0 = 1'd0;
        end
        endcase
end
endmodule

