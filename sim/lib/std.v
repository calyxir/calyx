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
  #(parameter width = 32,
    parameter reset_val = 0)
   (input logic  [width-1:0] in,
    input logic                in_read_in,
    input logic                valid,
    input logic                clk,
    // output
    output logic [width - 1:0] out,
    output logic out_read_out,
    output logic               ready);

   logic [width-1:0]           register;
   always_ff @(posedge clk) begin
      if (valid && in_read_in) begin
         register <= in;
      end
   end

   assign out = register;
   always_comb begin
      if (valid) begin
         ready = 1'd1;
         out_read_out = 1'd1;
      end else begin
         ready = 1'd0;
         out_read_out = 1'd0;
      end
   end
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
