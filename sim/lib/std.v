module std_const
  #(parameter width = 32,
    parameter value = 0)
   (input logic valid,
    output logic               ready,
    output logic [width - 1:0] out,
   output logic out_read_out);
   assign out = value;
   assign ready = valid;
   assign out_read_out = valid;
endmodule

module std_reg
  #(parameter width = 32)
   (input logic  [width-1:0] in,
    input logic                write_en,
    input logic                clk,
    // output
    output logic [width - 1:0] out);

   logic [width-1:0]           register;
   always_ff @(posedge clk) begin
      if (write_en) begin
         register <= in;
      end
   end

   assign out = register;
endmodule

module std_add
  #(parameter width = 32)
   (input logic [width-1:0] left,
    input logic              left_read_in,
    input logic [width-1:0]  right,
    input logic              right_read_in,
    input logic              valid,
    output logic             ready,
    output logic [width-1:0] out,
    output logic             out_read_out);
   always_comb begin
      if (valid && left_read_in && right_read_in) begin
         out = left + right;
         out_read_out = 1'd1;
         ready = 1'd1;
      end else begin
         out_read_out = 1'd0;
         ready = 1'd0;
      end
   end
endmodule

module std_sub
  #(parameter width = 32)
   (input logic [width-1:0] left,
    input logic [width-1:0]  right,
    input logic              valid,
    output logic             ready,
    output logic [width-1:0] out);
   assign out = left - right;
   assign ready = 1'd1;
endmodule

module std_mul
  #(parameter width = 32)
   (input logic [width-1:0] left,
    input logic [width-1:0]  right,
    input logic              valid,
    output logic             ready,
    output logic [width-1:0] out);
   assign out = left * right;
   assign ready = 1'd1;
endmodule

module std_div
  #(parameter width = 32)
   (input logic [width-1:0] left,
    input logic [width-1:0]  right,
    input logic              valid,
    output logic             ready,
    output logic [width-1:0] out);
   assign out = left / right;
   assign ready = 1'd1;
endmodule

module std_not
  #(parameter width = 32)
   (input logic [width-1:0] in,
    input logic             valid,
    output logic            ready,
    output logic [width-1:0] out);
   assign out = ~in;
   assign ready = 1'd1;
endmodule

module std_and
  #(parameter width = 32)
   (input logic [width-1:0] left,
    input logic [width-1:0]  right,
    input logic              valid,
    output logic             ready,
    output logic [width-1:0] out);
   assign out = left & right;
   assign ready = 1'd1;
endmodule

module std_or
  #(parameter width = 32)
   (input logic [width-1:0] left,
    input logic [width-1:0]  right,
    input logic              valid,
    output logic             ready,
    output logic [width-1:0] out);
   assign out = left | right;
   assign ready = 1'd1;
endmodule

module std_gt
  #(parameter width = 32)
   (input logic [width-1:0] left,
    input logic             left_read_in,
    input logic [width-1:0] right,
    input logic             right_read_in,
    input logic             valid,
    output logic            ready,
    output logic            out,
    output logic            out_read_out);
   always_comb begin
      if (valid && left_read_in && right_read_in) begin
         out = left > right;
         out_read_out = 1'd1;
         ready = 1'd1;
      end else begin
         out_read_out = 1'd0;
         ready = 1'd0;
      end
   end
endmodule

module std_lt
  #(parameter width = 32)
   (input logic [width-1:0] left,
    input logic             left_read_in,
    input logic [width-1:0] right,
    input logic             right_read_in,
    input logic             valid,
    output logic            ready,
    output logic            out,
    output logic            out_read_out);
   always_comb begin
      if (valid && left_read_in && right_read_in) begin
         out = left < right;
         out_read_out = 1'd1;
         ready = 1'd1;
      end else begin
         out_read_out = 1'd0;
         ready = 1'd0;
      end
   end
endmodule

module std_eq
  #(parameter width = 32)
   (input logic [width-1:0] left,
    input logic [width-1:0] right,
    input logic             valid,
    output logic            ready,
    output logic            out);
   assign out = left == right;
   assign ready = 1'd1;
endmodule

module std_neq
  #(parameter width = 32)
   (input logic [width-1:0] left,
    input logic [width-1:0] right,
    input logic             valid,
    output logic            ready,
    output logic            out);
   assign out = left != right;
   assign ready = 1'd1;
endmodule

module std_ge
  #(parameter width = 32)
   (input logic [width-1:0] left,
    input logic [width-1:0] right,
    input logic             valid,
    output logic            ready,
    output logic            out);
   assign out = left >= right;
   assign ready = 1'd1;
endmodule

module std_le
  #(parameter width = 32)
   (input logic [width-1:0] left,
    input logic [width-1:0] right,
    input logic             valid,
    output logic            ready,
    output logic            out);
   assign out = left <= right;
   assign ready = 1'd1;
endmodule

module std_start_fsm
  (input logic  valid,
   input logic  reset,
   input logic  clk,
   output logic out);
   logic        state;
   always_ff @(posedge clk) begin
      if (reset) begin
         out <= 1'b0;
         state <= 1'b0;
      end else
        case ({valid, state})
          2'b00: out <= 1'b0;
          2'b10: begin
             state <= 1'b1;
             out <= 1'b1;
          end
          2'b01: out <= 1'b0;
          2'b11: out <= 1'b0;
        endcase
   end
endmodule

module std_fsm_state
  (input logic  in,
   input logic  reset,
   input logic  clk,
   output logic out);

   logic        state;

   always_ff @(posedge clk) begin
      if (reset) state <= 1'b0;
      else begin
         state <= in;
      end
   end

   always_comb
     out = state;
endmodule
