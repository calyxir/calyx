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
logic a0_ready;
logic const0_ready;
logic fsm_enable_a0_const0_valid_const0;
logic b0_ready;
logic const1_ready;
logic fsm_enable_b0_const1_valid_const1;
logic gt0_ready;
logic const2_ready;
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
logic gt0_out_read_out;
logic gt0_out;
logic fsm_enable_a0_const0_ready;
logic fsm_par_0_valid_fsm_enable_a0_const0;
logic fsm_enable_b0_const1_ready;
logic fsm_par_0_valid_fsm_enable_b0_const1;
logic fsm_par_0_ready;
logic fsm_seq_1_valid_fsm_par_0;
logic fsm_enable_gt0a0_const2_ready;
logic fsm_seq_1_valid_fsm_enable_gt0a0_const2;
logic fsm_if_0_ready;
logic fsm_seq_1_valid_fsm_if_0;
logic fsm_enable_b0_const1_valid_b0;
logic fsm_if_0_cond_val_b0;
logic lut_control_0_valid;
logic fsm_enable_a0_const0_valid_a0;
logic fsm_enable_gt0a0_const2_valid_a0;
logic lut_control_1_valid;
logic fsm_enable_gt0a0_const2_valid_const2;
logic fsm_if_0_cond_val_const2;
logic lut_control_2_valid;
logic fsm_enable_gt0a0_const2_valid_gt0;
logic fsm_if_0_cond_val_gt0;
logic lut_control_3_valid;

// Subcomponent Instances
fsm_enable_a0_const0 #() fsm_enable_a0_const0 (
    .clk(clk),
    .valid_a0(fsm_enable_a0_const0_valid_a0),
    .ready(fsm_enable_a0_const0_ready),
    .valid(fsm_par_0_valid_fsm_enable_a0_const0),
    .ready_a0(a0_ready),
    .ready_const0(const0_ready),
    .valid_const0(fsm_enable_a0_const0_valid_const0)
);

fsm_enable_b0_const1 #() fsm_enable_b0_const1 (
    .ready_b0(b0_ready),
    .valid_const1(fsm_enable_b0_const1_valid_const1),
    .valid(fsm_par_0_valid_fsm_enable_b0_const1),
    .clk(clk),
    .valid_b0(fsm_enable_b0_const1_valid_b0),
    .ready_const1(const1_ready),
    .ready(fsm_enable_b0_const1_ready)
);

fsm_enable_gt0a0_const2 #() fsm_enable_gt0a0_const2 (
    .ready_a0(a0_ready),
    .clk(clk),
    .ready_gt0(gt0_ready),
    .valid(fsm_seq_1_valid_fsm_enable_gt0a0_const2),
    .ready(fsm_enable_gt0a0_const2_ready),
    .valid_gt0(fsm_enable_gt0a0_const2_valid_gt0),
    .ready_const2(const2_ready),
    .valid_a0(fsm_enable_gt0a0_const2_valid_a0),
    .valid_const2(fsm_enable_gt0a0_const2_valid_const2)
);

fsm_enable_y0_const3 #() fsm_enable_y0_const3 (
    .clk(clk),
    .valid_const3(fsm_enable_y0_const3_valid_const3),
    .ready(fsm_enable_y0_const3_ready),
    .ready_const3(const3_ready),
    .ready_y0(y0_ready),
    .valid_y0(fsm_enable_y0_const3_valid_y0),
    .valid(fsm_if_0_valid_t_fsm_enable_y0_const3)
);

fsm_enable_z0_const4 #() fsm_enable_z0_const4 (
    .ready(fsm_enable_z0_const4_ready),
    .clk(clk),
    .ready_const4(const4_ready),
    .valid_const4(fsm_enable_z0_const4_valid_const4),
    .ready_z0(z0_ready),
    .valid(fsm_if_0_valid_f_fsm_enable_z0_const4),
    .valid_z0(fsm_enable_z0_const4_valid_z0)
);

fsm_if_0 #() fsm_if_0 (
    .condition_read_in(gt0_out_read_out),
    .valid_f_fsm_enable_z0_const4(fsm_if_0_valid_f_fsm_enable_z0_const4),
    .ready(fsm_if_0_ready),
    .cond_val_b0(fsm_if_0_cond_val_b0),
    .cond_val_const2(fsm_if_0_cond_val_const2),
    .condition(gt0_out),
    .cond_rdy_gt0(gt0_ready),
    .valid_t_fsm_enable_y0_const3(fsm_if_0_valid_t_fsm_enable_y0_const3),
    .ready_f_fsm_enable_z0_const4(fsm_enable_z0_const4_ready),
    .cond_val_gt0(fsm_if_0_cond_val_gt0),
    .valid(fsm_seq_1_valid_fsm_if_0),
    .clk(clk),
    .cond_rdy_const2(const2_ready),
    .cond_rdy_b0(b0_ready),
    .ready_t_fsm_enable_y0_const3(fsm_enable_y0_const3_ready)
);

fsm_par_0 #() fsm_par_0 (
    .valid_fsm_enable_b0_const1(fsm_par_0_valid_fsm_enable_b0_const1),
    .ready(fsm_par_0_ready),
    .clk(clk),
    .ready_fsm_enable_a0_const0(fsm_enable_a0_const0_ready),
    .valid_fsm_enable_a0_const0(fsm_par_0_valid_fsm_enable_a0_const0),
    .ready_fsm_enable_b0_const1(fsm_enable_b0_const1_ready),
    .valid(fsm_seq_1_valid_fsm_par_0)
);

fsm_seq_1 #() fsm_seq_1 (
    .ready_fsm_enable_gt0a0_const2(fsm_enable_gt0a0_const2_ready),
    .valid_fsm_enable_gt0a0_const2(fsm_seq_1_valid_fsm_enable_gt0a0_const2),
    .clk(clk),
    .valid(valid),
    .ready_fsm_par_0(fsm_par_0_ready),
    .valid_fsm_par_0(fsm_seq_1_valid_fsm_par_0),
    .ready(),
    .ready_fsm_if_0(fsm_if_0_ready),
    .valid_fsm_if_0(fsm_seq_1_valid_fsm_if_0)
);

lut_control_0 #() lut_control_0 (
    .valid(lut_control_0_valid),
    .valid0(fsm_enable_b0_const1_valid_b0),
    .valid1(fsm_if_0_cond_val_b0)
);

lut_control_1 #() lut_control_1 (
    .valid(lut_control_1_valid),
    .valid3(fsm_enable_gt0a0_const2_valid_a0),
    .valid2(fsm_enable_a0_const0_valid_a0)
);

lut_control_2 #() lut_control_2 (
    .valid5(fsm_if_0_cond_val_const2),
    .valid4(fsm_enable_gt0a0_const2_valid_const2),
    .valid(lut_control_2_valid)
);

lut_control_3 #() lut_control_3 (
    .valid6(fsm_enable_gt0a0_const2_valid_gt0),
    .valid7(fsm_if_0_cond_val_gt0),
    .valid(lut_control_3_valid)
);
std_reg #(32, 0) a0 (
    .clk(clk),
    .valid(lut_control_1_valid),
    .out_read_out(a0_out_read_out),
    .out(a0_out),
    .in_read_in(const0_out_read_out),
    .in(const0_out),
    .ready(a0_ready)
);

std_const #(32, 10) const0 (
    .ready(const0_ready),
    .out(const0_out),
    .valid(fsm_enable_a0_const0_valid_const0),
    .out_read_out(const0_out_read_out)
);

std_reg #(32, 0) b0 (
    .clk(clk),
    .in_read_in(const1_out_read_out),
    .valid(lut_control_0_valid),
    .out(),
    .ready(b0_ready),
    .out_read_out(),
    .in(const1_out)
);

std_const #(32, 1) const1 (
    .out(const1_out),
    .ready(const1_ready),
    .valid(fsm_enable_b0_const1_valid_const1),
    .out_read_out(const1_out_read_out)
);

std_gt #(32) gt0 (
    .right(const2_out),
    .right_read_in(const2_out_read_out),
    .left(a0_out),
    .left_read_in(a0_out_read_out),
    .ready(gt0_ready),
    .out_read_out(gt0_out_read_out),
    .out(gt0_out),
    .valid(lut_control_3_valid)
);

std_const #(32, 5) const2 (
    .out_read_out(const2_out_read_out),
    .valid(lut_control_2_valid),
    .out(const2_out),
    .ready(const2_ready)
);

std_reg #(32, 0) y0 (
    .in_read_in(const3_out_read_out),
    .in(const3_out),
    .out(),
    .out_read_out(),
    .valid(fsm_enable_y0_const3_valid_y0),
    .clk(clk),
    .ready(y0_ready)
);

std_const #(32, 20) const3 (
    .out_read_out(const3_out_read_out),
    .ready(const3_ready),
    .out(const3_out),
    .valid(fsm_enable_y0_const3_valid_const3)
);

std_reg #(32, 0) z0 (
    .out(),
    .out_read_out(),
    .in(const4_out),
    .in_read_in(const4_out_read_out),
    .valid(fsm_enable_z0_const4_valid_z0),
    .clk(clk),
    .ready(z0_ready)
);

std_const #(32, 40) const4 (
    .valid(fsm_enable_z0_const4_valid_const4),
    .out(const4_out),
    .out_read_out(const4_out_read_out),
    .ready(const4_ready)
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
        default: valid= 0;
    endcase
end
endmodule

module lut_control_1 (
    input logic valid2,
    input logic valid3,
    output logic valid
);
always_comb begin
    case ({valid2, valid3})
            2'b10: valid = 1;
            2'b01: valid = 1;
        default: valid= 0;
    endcase
end
endmodule

module lut_control_2 (
    input logic valid4,
    input logic valid5,
    output logic valid
);
always_comb begin
    case ({valid4, valid5})
            2'b10: valid = 1;
            2'b01: valid = 1;
        default: valid= 0;
    endcase
end
endmodule

module lut_control_3 (
    input logic valid6,
    input logic valid7,
    output logic valid
);
always_comb begin
    case ({valid6, valid7})
            2'b10: valid = 1;
            2'b01: valid = 1;
        default: valid= 0;
    endcase
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
    
        default: begin
            valid_a0 = 1'd0;
            valid_const0 = 1'd0;
            ready = 1'd0;
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
                2'd1: begin
                    if ( ready_b0 == 1'd1 && ready_const1 == 1'd1 )
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
    
        default: begin
            valid_b0 = 1'd0;
            valid_const1 = 1'd0;
            ready = 1'd0;
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
                2'd0: begin
                    if ( valid == 1'd1 )
                        next_state = 2'd1;
                    else
                        next_state = 2'd0;
                end
                2'd1: begin
                    if ( ready_gt0 == 1'd1 && ready_a0 == 1'd1
                    && ready_const2 == 1'd1 )
                        next_state = 2'd2;
                    else
                        next_state = 2'd1;
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
        2'd0: begin
            valid_gt0 = 1'd0;
            valid_a0 = 1'd0;
            valid_const2 = 1'd0;
            ready = 1'd0;
        end
        2'd1: begin
            valid_gt0 = 1'd1;
            valid_a0 = 1'd1;
            valid_const2 = 1'd1;
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
                2'd0: begin
                    if ( valid == 1'd1 )
                        next_state = 2'd1;
                    else
                        next_state = 2'd0;
                end
                2'd1: begin
                    if ( ready_y0 == 1'd1 && ready_const3 == 1'd1 )
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
            valid_y0 = 1'd0;
            valid_const3 = 1'd0;
        end
        2'd0: begin
            ready = 1'd0;
            valid_y0 = 1'd0;
            valid_const3 = 1'd0;
        end
        2'd1: begin
            valid_y0 = 1'd1;
            valid_const3 = 1'd1;
            ready = 1'd0;
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
        2'd0: begin
            valid_z0 = 1'd0;
            valid_const4 = 1'd0;
            ready = 1'd0;
        end
        2'd1: begin
            valid_z0 = 1'd1;
            valid_const4 = 1'd1;
            ready = 1'd0;
        end
        2'd2: begin
            ready = 1'd1;
            valid_z0 = 1'd0;
            valid_const4 = 1'd0;
        end
    
        default: begin
            valid_z0 = 1'd0;
            valid_const4 = 1'd0;
            ready = 1'd0;
        end
        endcase
end
endmodule

module fsm_if_0 (
    input logic condition,
    input logic condition_read_in,
    input logic valid,
    input logic cond_rdy_gt0,
    input logic cond_rdy_b0,
    input logic cond_rdy_const2,
    input logic ready_t_fsm_enable_y0_const3,
    input logic ready_f_fsm_enable_z0_const4,
    input logic clk,
    output logic ready,
    output logic cond_val_gt0,
    output logic cond_val_b0,
    output logic cond_val_const2,
    output logic valid_t_fsm_enable_y0_const3,
    output logic valid_f_fsm_enable_z0_const4
);
logic [2:0] state, next_state;
always_ff @(posedge clk) begin
    state <= next_state;
end
always_comb begin
    case (state)
                3'd0: begin
                    if ( valid == 1'd1 )
                        next_state = 3'd1;
                    else
                        next_state = 3'd0;
                end
                3'd2: begin
                    if ( valid == 1'd0 )
                        next_state = 3'd0;
                    else
                        next_state = 3'd2;
                end
                3'd1: begin
                    if ( cond_rdy_gt0 == 1'd1 && cond_rdy_b0 == 1'd1
                    && cond_rdy_const2 == 1'd1 && condition == 1'd0
                    && condition_read_in == 1'd0 )
                        next_state = 3'd3;
                    else if ( cond_rdy_gt0 == 1'd1 && cond_rdy_b0 == 1'd1
                    && cond_rdy_const2 == 1'd1 && condition == 1'd1
                    && condition_read_in == 1'd1 )
                        next_state = 3'd4;
                    else
                        next_state = 3'd1;
                end
                3'd3: begin
                    if ( ready_f_fsm_enable_z0_const4 == 1'd1 )
                        next_state = 3'd2;
                    else
                        next_state = 3'd3;
                end
                3'd4: begin
                    if ( ready_t_fsm_enable_y0_const3 == 1'd1 )
                        next_state = 3'd2;
                    else
                        next_state = 3'd4;
                end
            
            default: 
        next_state = 3'd0;
    endcase
end
always_comb begin
    case (state)
        3'd0: begin
            ready = 1'd0;
            cond_val_gt0 = 1'd0;
            cond_val_b0 = 1'd0;
            cond_val_const2 = 1'd0;
            valid_f_fsm_enable_z0_const4 = 1'd0;
            valid_t_fsm_enable_y0_const3 = 1'd0;
        end
        3'd2: begin
            ready = 1'd1;
            cond_val_gt0 = 1'd0;
            cond_val_b0 = 1'd0;
            cond_val_const2 = 1'd0;
            valid_f_fsm_enable_z0_const4 = 1'd0;
            valid_t_fsm_enable_y0_const3 = 1'd0;
        end
        3'd1: begin
            cond_val_gt0 = 1'd1;
            cond_val_b0 = 1'd1;
            cond_val_const2 = 1'd1;
            ready = 1'd0;
            valid_f_fsm_enable_z0_const4 = 1'd0;
            valid_t_fsm_enable_y0_const3 = 1'd0;
        end
        3'd3: begin
            valid_f_fsm_enable_z0_const4 = 1'd1;
            ready = 1'd0;
            cond_val_gt0 = 1'd0;
            cond_val_b0 = 1'd0;
            cond_val_const2 = 1'd0;
            valid_t_fsm_enable_y0_const3 = 1'd0;
        end
        3'd4: begin
            valid_t_fsm_enable_y0_const3 = 1'd1;
            ready = 1'd0;
            cond_val_gt0 = 1'd0;
            cond_val_b0 = 1'd0;
            cond_val_const2 = 1'd0;
            valid_f_fsm_enable_z0_const4 = 1'd0;
        end
    
        default: begin
            ready = 1'd0;
            cond_val_gt0 = 1'd0;
            cond_val_b0 = 1'd0;
            cond_val_const2 = 1'd0;
            valid_f_fsm_enable_z0_const4 = 1'd0;
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
        2'd2: begin
            ready = 1'd1;
            valid_fsm_enable_a0_const0 = 1'd0;
            valid_fsm_enable_b0_const1 = 1'd0;
        end
        2'd0: begin
            valid_fsm_enable_a0_const0 = 1'd0;
            valid_fsm_enable_b0_const1 = 1'd0;
            ready = 1'd0;
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
    input logic ready_fsm_par_0,
    input logic ready_fsm_enable_gt0a0_const2,
    input logic ready_fsm_if_0,
    input logic clk,
    output logic ready,
    output logic valid_fsm_par_0,
    output logic valid_fsm_enable_gt0a0_const2,
    output logic valid_fsm_if_0
);
logic [2:0] state, next_state;
always_ff @(posedge clk) begin
    state <= next_state;
end
always_comb begin
    case (state)
                3'd1: begin
                    if ( ready_fsm_par_0 == 1'd1 )
                        next_state = 3'd2;
                    else
                        next_state = 3'd1;
                end
                3'd3: begin
                    if ( ready_fsm_if_0 == 1'd1 )
                        next_state = 3'd4;
                    else
                        next_state = 3'd3;
                end
                3'd4: begin
                    if ( valid == 1'd0 )
                        next_state = 3'd0;
                    else
                        next_state = 3'd4;
                end
                3'd0: begin
                    if ( valid == 1'd1 )
                        next_state = 3'd1;
                    else
                        next_state = 3'd0;
                end
                3'd2: begin
                    if ( ready_fsm_enable_gt0a0_const2 == 1'd1 )
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
        3'd1: begin
            valid_fsm_par_0 = 1'd1;
            valid_fsm_if_0 = 1'd0;
            ready = 1'd0;
            valid_fsm_enable_gt0a0_const2 = 1'd0;
        end
        3'd3: begin
            valid_fsm_if_0 = 1'd1;
            valid_fsm_par_0 = 1'd0;
            ready = 1'd0;
            valid_fsm_enable_gt0a0_const2 = 1'd0;
        end
        3'd4: begin
            ready = 1'd1;
            valid_fsm_par_0 = 1'd0;
            valid_fsm_if_0 = 1'd0;
            valid_fsm_enable_gt0a0_const2 = 1'd0;
        end
        3'd0: begin
            valid_fsm_par_0 = 1'd0;
            valid_fsm_if_0 = 1'd0;
            ready = 1'd0;
            valid_fsm_enable_gt0a0_const2 = 1'd0;
        end
        3'd2: begin
            valid_fsm_enable_gt0a0_const2 = 1'd1;
            valid_fsm_par_0 = 1'd0;
            valid_fsm_if_0 = 1'd0;
            ready = 1'd0;
        end
    
        default: begin
            valid_fsm_par_0 = 1'd0;
            valid_fsm_if_0 = 1'd0;
            ready = 1'd0;
            valid_fsm_enable_gt0a0_const2 = 1'd0;
        end
        endcase
end
endmodule

