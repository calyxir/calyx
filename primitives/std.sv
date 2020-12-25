/**
 * The FuTIL standard library.
 * Implement verilog primitives that are unrepresentable in FuTIL.
 * Conventions:
 * - All parameter names must be SNAKE_CASE and all caps.
 * - Port names must be snake_case, no caps.
 */
`default_nettype none

module std_mem_d1
#(parameter width = 32,
  parameter size = 16,
  parameter idx_size = 4)
  (input logic [idx_size-1:0] addr0,
    input logic [width-1:0]   write_data,
    input logic               write_en,
    input logic               clk,
    output logic [width-1:0]  read_data,
    output logic done);

  logic [width-1:0]  mem[size-1:0];

  /* verilator lint_off WIDTH */
  assign read_data = mem[addr0];
  always_ff @(posedge clk) begin
    if (write_en) begin
      mem[addr0] <= write_data;
      done <= 1'd1;
    end else
      done <= 1'd0;
  end
endmodule

module std_mem_d2
#(parameter width = 32,
  parameter d0_size = 16,
  parameter d1_size = 16,
  parameter d0_idx_size = 4,
  parameter d1_idx_size = 4)
  (input logic [d0_idx_size-1:0] addr0,
    input logic [d1_idx_size-1:0] addr1,
    input logic [width-1:0]   write_data,
    input logic               write_en,
    input logic               clk,
    output logic [width-1:0]  read_data,
    output logic done);

  /* verilator lint_off WIDTH */
  logic [width-1:0]  mem[d0_size-1:0][d1_size-1:0];

  assign read_data = mem[addr0][addr1];
  always_ff @(posedge clk) begin
    if (write_en) begin
      mem[addr0][addr1] <= write_data;
      done <= 1'd1;
    end else
      done <= 1'd0;
  end
endmodule

module std_mem_d3
#(parameter width = 32,
  parameter d0_size = 16,
  parameter d1_size = 16,
  parameter d2_size = 16,
  parameter d0_idx_size = 4,
  parameter d1_idx_size = 4,
  parameter d2_idx_size = 4)
  (input logic [d0_idx_size-1:0] addr0,
    input logic [d1_idx_size-1:0] addr1,
    input logic [d2_idx_size-1:0] addr2,
    input logic [width-1:0]   write_data,
    input logic               write_en,
    input logic               clk,
    output logic [width-1:0]  read_data,
    output logic done);

  /* verilator lint_off WIDTH */
  logic [width-1:0]  mem[d0_size-1:0][d1_size-1:0][d2_size-1:0];

  assign read_data = mem[addr0][addr1][addr2];
  always_ff @(posedge clk) begin
    if (write_en) begin
      mem[addr0][addr1][addr2] <= write_data;
      done <= 1'd1;
    end else
      done <= 1'd0;
  end
endmodule

module std_mem_d4
#(parameter width = 32,
  parameter d0_size = 16,
  parameter d1_size = 16,
  parameter d2_size = 16,
  parameter d3_size = 16,
  parameter d0_idx_size = 4,
  parameter d1_idx_size = 4,
  parameter d2_idx_size = 4,
  parameter d3_idx_size = 4)
  (input logic [d0_idx_size-1:0] addr0,
   input logic [d1_idx_size-1:0] addr1,
   input logic [d2_idx_size-1:0] addr2,
   input logic [d3_idx_size-1:0] addr3,
   input logic [width-1:0]   write_data,
   input logic               write_en,
   input logic               clk,
   output logic [width-1:0]  read_data,
   output logic done);

  /* verilator lint_off WIDTH */
  logic [width-1:0]  mem[d0_size-1:0][d1_size-1:0][d2_size-1:0][d3_size-1:0];

  assign read_data = mem[addr0][addr1][addr2][addr3];
  always_ff @(posedge clk) begin
    if (write_en) begin
      mem[addr0][addr1][addr2][addr3] <= write_data;
      done <= 1'd1;
    end else
      done <= 1'd0;
  end
endmodule

module std_reg
#(parameter width = 32)
 (input wire [width-1:0] in,
  input wire write_en,
  input wire clk,
  // output
  output logic [width - 1:0] out,
  output logic done);

  always_ff @(posedge clk) begin
    if (write_en) begin
      out <= in;
      done <= 1'd1;
    end else
      done <= 1'd0;
  end
endmodule

module std_const
  #(parameter width = 32,
    parameter value = 0)
   (output logic [width - 1:0] out);
  assign out = value;
endmodule

module std_slice
  #(parameter in_width = 32,
    parameter out_width = 32)
  (input  logic [in_width-1:0] in,
   output logic [out_width-1:0] out);
  assign out = in[out_width-1:0];
endmodule

module std_lsh
  #(parameter width = 32)
  (input  logic [width-1:0] left,
    input  logic [width-1:0] right,
    output logic [width-1:0] out);
  assign out = left << right;
endmodule

module std_rsh
  #(parameter width = 32)
  (input  logic [width-1:0] left,
    input  logic [width-1:0] right,
    output logic [width-1:0] out);
  assign out = left >> right;
endmodule

module std_add
  #(parameter width = 32)
  (input  logic [width-1:0] left,
    input  logic [width-1:0] right,
    output logic [width-1:0] out);
  assign out = left + right;
endmodule

module std_sub
  #(parameter width = 32)
  (input  logic [width-1:0] left,
    input  logic [width-1:0] right,
    output logic [width-1:0] out);
  assign out = left - right;
endmodule

module std_mod
  #(parameter width = 32)
  (input  logic [width-1:0] left,
    input  logic [width-1:0] right,
    output logic [width-1:0] out);
  assign out = left % right;
endmodule

module std_mod_pipe
          #(parameter width = 32)
            (input            clk, reset,
            input                  go,
            input [width-1:0]      left,
            input [width-1:0]      right,
            output reg [width-1:0] out,
            output reg             done);

  wire start = go && !running && !reset;

  reg [width-1:0]     dividend;
  reg [(width-1)*2:0] divisor;
  reg [width-1:0]     quotient;
  reg [width-1:0]     quotient_msk;
  reg                 running;

  always @(posedge clk) begin
      if (reset || !go) begin
        running <= 0;
        done <= 0;
        out <= 0;
      end else
       if (start && left == 0) begin
          out <= 0;
          done <= 1;
       end if (start) begin
          running <= 1;
          dividend <= left;
          divisor <= right << width-1;
          quotient <= 0;
          quotient_msk <= 1 << width-1;
        end else
          if (!quotient_msk && running) begin
            running <= 0;
            done <= 1;
            out <= dividend;
          end else begin
            if (divisor <= dividend) begin
                dividend <= dividend - divisor;
                quotient <= quotient | quotient_msk;
            end
            divisor <= divisor >> 1;
            quotient_msk <= quotient_msk >> 1;
          end
  end
endmodule

module std_mult
  #(parameter width = 32)
  (input logic  [width-1:0] left,
    input logic  [width-1:0] right,
    output logic [width-1:0] out);
  assign out = left * right;
endmodule

module std_mult_pipe
  #(parameter width = 32)
   (input logic [width-1:0] left,
    input logic [width-1:0] right,
    input logic go,
    input logic clk,
    output logic [width-1:0] out,
    output logic done);
   logic [width-1:0] rtmp;
   logic [width-1:0] ltmp;
   logic [width-1:0] out_tmp;
   reg done_buf[1:0];
   always_ff @(posedge clk) begin
     if (go) begin
       rtmp <= right;
       ltmp <= left;
       out_tmp <= rtmp * ltmp;
       out <= out_tmp;

       done <= done_buf[1];
       done_buf[0] <= 1'b1;
       done_buf[1] <= done_buf[0];
     end else begin
       rtmp <= 0;
       ltmp <= 0;
       out_tmp <= 0;
       out <= 0;

       done <= 0;
       done_buf[0] <= 0;
       done_buf[1] <= 0;
     end
   end
 endmodule

module std_div
  #(parameter width = 32)
  (input logic  [width-1:0] left,
    input logic  [width-1:0] right,
    output logic [width-1:0] out);
  assign out = left / right;
endmodule

/* verilator lint_off WIDTH */
module std_div_pipe
          #(parameter width = 32)
            (input             clk, reset,
            input                  go,
            input [width-1:0]      left,
            input [width-1:0]      right,
            output reg [width-1:0] out,
            output reg             done);

  wire start = go && !running && !reset;

  reg [width-1:0]     dividend;
  reg [(width-1)*2:0] divisor;
  reg [width-1:0]     quotient;
  reg [width-1:0]     quotient_msk;
  reg                 running;

  always @(posedge clk) begin
      if (reset || !go) begin
        running <= 0;
        done <= 0;
        out <= 0;
      end else
       if (start && left == 0) begin
          out <= 0;
          done <= 1;
       end if (start) begin
          running <= 1;
          dividend <= left;
          divisor <= right << width-1;
          quotient <= 0;
          quotient_msk <= 1 << width-1;
        end else
          if (!quotient_msk && running) begin
            running <= 0;
            done <= 1;
            out <= quotient;
          end else begin
            if (divisor <= dividend) begin
                dividend <= dividend - divisor;
                quotient <= quotient | quotient_msk;
            end
            divisor <= divisor >> 1;
            quotient_msk <= quotient_msk >> 1;
          end
  end
endmodule

module std_not
  #(parameter width = 32)
  (input logic [width-1:0] in,
    output logic [width-1:0] out);
  assign out = ~in;
endmodule

module std_and
  #(parameter width = 32)
  (input logic  [width-1:0] left,
    input logic  [width-1:0] right,
    output logic [width-1:0] out);
  assign out = left & right;
endmodule

module std_or
  #(parameter width = 32)
  (input logic  [width-1:0] left,
    input logic  [width-1:0] right,
    output logic [width-1:0] out);
  assign out = left | right;
endmodule

module std_xor
  #(parameter width = 32)
  (input logic  [width-1:0] left,
    input logic  [width-1:0] right,
    output logic [width-1:0] out);
  assign out = left ^ right;
endmodule

module std_gt
  #(parameter width = 32)
  (input logic [width-1:0] left,
    input logic [width-1:0] right,
    output logic            out);
  assign out = left > right;
endmodule

module std_lt
  #(parameter width = 32)
  (input logic [width-1:0] left,
    input logic [width-1:0] right,
    output logic            out);
  assign out = left < right;
endmodule

module std_eq
  #(parameter width = 32)
  (input logic [width-1:0] left,
    input logic [width-1:0] right,
    output logic            out);
  assign out = left == right;
endmodule

module std_neq
  #(parameter width = 32)
  (input logic [width-1:0] left,
    input logic [width-1:0] right,
    output logic            out);
  assign out = left != right;
endmodule

module std_ge
  #(parameter width = 32)
  (input logic [width-1:0] left,
    input logic [width-1:0] right,
    output logic            out);
  assign out = left >= right;
endmodule

module std_le
  #(parameter width = 32)
  (input logic [width-1:0] left,
   input logic [width-1:0] right,
   output logic            out);
  assign out = left <= right;
endmodule

module std_exp
  (input  logic [31:0]  exponent,
   input  logic        go,
   input  logic        clk,
   output logic [31:0] out,
   output logic        done);
   always_ff @(posedge clk) begin
     if (go) begin
       /* verilator lint_off REALCVT */
       out <= 2.718281 ** exponent;
       done <= 1;
     end else begin
       out <= 0;
       done <= 0;
     end
   end
endmodule

module std_sqrt
  (input logic [31:0]  in,
   input logic         go,
   input logic         clk,
   output logic [31:0] out,
   output logic        done);
  // declare the variables
  logic [31:0] a;
  logic [15:0] q;
  logic [17:0] left,right,r;
  integer i;
  always_ff @(posedge clk) begin
    if (go && i == 0) begin
      // initialize all the variables.
      a     <= in;
      q     <= 0;
      i     <= 1;
      left  <= 0;  // input to adder/sub
      right <= 0;  // input to adder/sub
      r     <= 0;  // remainder
    // run the calculations for 16 iterations.
    end else if (go && i <= 16) begin
      right <= { q, r[17], 1'b1 };
      left  <= { r[15:0], a[31:30] };
      a     <= { a[29:0], 2'b00 };    //left shift by 2 bits.
      if (r[17] == 1) //add if r is negative
          r <= left + right;
      else    //subtract if r is positive
          r <= left - right;
      q <= {q[14:0],!r[17]};

      if (i == 16) begin
        out <= {16'd0,q};   //final assignment of output.
        i <= 0;
        done <= 1;
      end else
        i <= i + 1;
    end else begin
      // initialize all the variables.
      a <= in;
      q <= 0;
      i <= 0;
      left <= 0;   // input to adder/sub
      right <= 0;  // input to adder/sub
      r <= 0;      // remainder
      done <= 0;
    end
  end
endmodule

/////// fixed_point primitive ///////////

module fixed_p_std_const
  #(parameter width = 32,
    parameter int_width = 8,
    parameter fract_width = 24,
    parameter [int_width:0] value1 = 0,
    parameter [fract_width:0] value2 = 0)
  (output logic [width-1:0] out);

  assign out = {value1, value2};
endmodule


module fixed_p_std_add
  #(parameter width= 32,
  parameter int_width= 8,
  parameter fract_width= 24)

  (input logic [width-1:0] left,
  input logic [width-1:0] right,
  output logic [width-1:0] out);

  assign out = left + right;
endmodule

module fixed_p_std_sub
  #(parameter width= 32,
  parameter int_width= 8,
  parameter fract_width= 24)

  (input logic [width-1:0] left,
  input logic [width-1:0] right,
  output logic [width-1:0] out);

  assign out = left -right;
endmodule

module fixed_p_std_mult
  #(parameter width= 32,
  parameter int_width= 8,
  parameter fract_width= 24)

  (input logic [width-1:0] left,
  input logic [width-1:0] right,
  output logic [width-1:0] out);

  logic [2*width-2:0] result;

  assign result = left * right;
  assign out = result[width + fract_width - 1: fract_width];
endmodule

module fixed_p_std_div
  #(parameter width= 32,
    parameter int_width= 8,
    parameter fract_width= 24)
  (input logic [width-1:0] left,
   input logic [width-1:0] right,
   output logic [width-1:0] out);

  logic [2*width-2:0] result;

  assign result = left / right;
  assign out = result[width+fract_width-1:fract_width];
endmodule

module fixed_p_std_gt
 #(parameter width = 32,
   parameter int_width = 8,
   parameter fract_width = 24)
   (input  logic [width-1:0] left,
    input  logic [width-1:0] right,
    output logic             out);
  assign out = left > right;
endmodule

module fixed_p_std_add_dbit
  #(parameter width1= 32,
  parameter width2= 32,
  parameter int_width1 = 8,
  parameter fract_width1 = 24,
  parameter int_width2 = 4,
  parameter fract_width2 =28,
  parameter out_width = 36)

  (input logic [width1-1:0] left,
  input logic [width2-1:0] right,
  output logic [out_width-1:0] out);

  logic [int_width1-1:0] left_int;
  logic [int_width2-1:0] right_int;
  logic [fract_width1-1:0] left_fract;
  logic [fract_width2-1:0] right_fract;

  localparam bigint = (int_width1 >= int_width2) ? int_width1 : int_width2;
  localparam bigfract = (fract_width1 >= fract_width2) ? fract_width1 : fract_width2;

  logic [bigint-1:0]  mod_right_int;
  logic [bigfract-1:0] mod_left_fract;

  logic [bigint-1:0]  whole_int;
  logic [ bigfract-1:0] whole_fract;

  assign {left_int, left_fract} = left;
  assign {right_int, right_fract} = right;

  assign mod_left_fract = left_fract * (2**(fract_width2-fract_width1));

  always_comb begin
    if ((mod_left_fract + right_fract) >= 2**fract_width2) begin
      whole_int = left_int + right_int+1;
      whole_fract =mod_left_fract + right_fract-2**fract_width2;
    end
    else begin
      whole_int = left_int + right_int;
      whole_fract =mod_left_fract + right_fract;
    end
  end

  assign out = {whole_int, whole_fract};
endmodule

/////// signed primitives ///////
module std_slsh
  #(parameter width = 32)
  (input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed [width-1:0] out);
  assign out = left << right;
endmodule

module std_srsh
  #(parameter width = 32)
  (input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed [width-1:0] out);
  assign out = left >> right;
endmodule

module std_sadd
  #(parameter width = 32)
  (input  signed  [width-1:0] left,
    input  signed  [width-1:0] right,
    output signed  [width-1:0] out);
  assign out = left + right;
endmodule

module std_ssub
  #(parameter width = 32)
  (input  signed  [width-1:0] left,
    input  signed  [width-1:0] right,
    output signed  [width-1:0] out);
  assign out = left - right;
endmodule

module std_smod
  #(parameter width = 32)
  (input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed [width-1:0] out);
  assign out = left % right;
endmodule

module std_smod_pipe
          #(parameter width = 32)
            (input            clk, reset,
            input                  go,
            input  signed [width-1:0]      left,
            input  signed [width-1:0]      right,
            output reg [width-1:0] out,
            output reg             done);

  wire start = go && !running && !reset;

  reg [width-1:0]     dividend;
  reg [(width-1)*2:0] divisor;
  reg [width-1:0]     quotient;
  reg [width-1:0]     quotient_msk;
  reg                 running;

  always @(posedge clk) begin
      if (reset || !go) begin
        running <= 0;
        done <= 0;
        out <= 0;
      end else
       if (start && left == 0) begin
          out <= 0;
          done <= 1;
       end if (start) begin
          running <= 1;
          dividend <= left;
          divisor <= right << width-1;
          quotient <= 0;
          quotient_msk <= 1 << width-1;
        end else
          if (!quotient_msk && running) begin
            running <= 0;
            done <= 1;
            out <= dividend;
          end else begin
            if (divisor <= dividend) begin
                dividend <= dividend - divisor;
                quotient <= quotient | quotient_msk;
            end
            divisor <= divisor >> 1;
            quotient_msk <= quotient_msk >> 1;
          end
  end
endmodule

module std_smult
  #(parameter width = 32)
  (input  signed  [width-1:0] left,
    input  signed  [width-1:0] right,
    output signed  [width-1:0] out);
  assign out = left * right;
endmodule

module std_smult_pipe
  #(parameter width = 32)
   (input signed [width-1:0] left,
    input signed [width-1:0] right,
    input logic go,
    input logic clk,
    output signed [width-1:0] out,
    output logic done);
   logic signed [width-1:0] rtmp;
   logic signed [width-1:0] ltmp;
   logic signed [width-1:0] out_tmp;
   reg done_buf[1:0];
   always_ff @(posedge clk) begin
     if (go) begin
       rtmp <= right;
       ltmp <= left;
       out_tmp <= rtmp * ltmp;
       out <= out_tmp;

       done <= done_buf[1];
       done_buf[0] <= 1'b1;
       done_buf[1] <= done_buf[0];
     end else begin
       rtmp <= 0;
       ltmp <= 0;
       out_tmp <= 0;
       out <= 0;

       done <= 0;
       done_buf[0] <= 0;
       done_buf[1] <= 0;
     end
   end
endmodule

module std_sdiv
  #(parameter width = 32)
  (input  signed  [width-1:0] left,
    input  signed  [width-1:0] right,
    output signed  [width-1:0] out);
  assign out = left / right;
endmodule

/* verilator lint_off WIDTH */
module std_sdiv_pipe
          #(parameter width = 32)
            (input             clk, reset,
            input                  go,
            input signed [width-1:0]      left,
            input signed [width-1:0]      right,
            output reg [width-1:0] out,
            output reg             done);

  wire start = go && !running && !reset;

  reg [width-1:0]     dividend;
  reg [(width-1)*2:0] divisor;
  reg [width-1:0]     quotient;
  reg [width-1:0]     quotient_msk;
  reg                 running;

  always @(posedge clk) begin
      if (reset || !go) begin
        running <= 0;
        done <= 0;
        out <= 0;
      end else
       if (start && left == 0) begin
          out <= 0;
          done <= 1;
       end if (start) begin
          running <= 1;
          dividend <= left;
          divisor <= right << width-1;
          quotient <= 0;
          quotient_msk <= 1 << width-1;
        end else
          if (!quotient_msk && running) begin
            running <= 0;
            done <= 1;
            out <= quotient;
          end else begin
            if (divisor <= dividend) begin
                dividend <= dividend - divisor;
                quotient <= quotient | quotient_msk;
            end
            divisor <= divisor >> 1;
            quotient_msk <= quotient_msk >> 1;
          end
  end
endmodule

module std_snot
  #(parameter width = 32)
  (input signed [width-1:0] in,
    output signed [width-1:0] out);
  assign out = ~in;
endmodule

module std_sand
  #(parameter width = 32)
  (input signed  [width-1:0] left,
    input signed  [width-1:0] right,
    output signed [width-1:0] out);
  assign out = left & right;
endmodule

module std_sor
  #(parameter width = 32)
  (input signed  [width-1:0] left,
    input signed  [width-1:0] right,
    output signed [width-1:0] out);
  assign out = left | right;
endmodule

module std_sgt
  #(parameter width = 32)
  (input signed [width-1:0] left,
    input signed [width-1:0] right,
    output signed            out);
  assign out = left > right;
endmodule

module std_slt
  #(parameter width = 32)
  (input signed [width-1:0] left,
    input signed [width-1:0] right,
    output signed            out);
  assign out = left < right;
endmodule

module std_seq
  #(parameter width = 32)
  (input signed [width-1:0] left,
    input signed [width-1:0] right,
    output signed            out);
  assign out = left == right;
endmodule

module std_sneq
  #(parameter width = 32)
  (input signed [width-1:0] left,
    input signed [width-1:0] right,
    output signed            out);
  assign out = left != right;
endmodule

module std_sge
  #(parameter width = 32)
  (input signed [width-1:0] left,
    input signed [width-1:0] right,
    output signed            out);
  assign out = left >= right;
endmodule

module std_sle
  #(parameter width = 32)
  (input signed [width-1:0] left,
   input signed [width-1:0] right,
   output signed            out);
  assign out = left <= right;
endmodule

module std_ssqrt
  (input signed [31:0]  in,
   input logic         go,
   input logic         clk,
   output signed [31:0] out,
   output logic        done);
  // declare the variables
  reg [31:0] a;
  reg [15:0] q;
  reg [17:0] left,right,r;
  integer i;

  always_ff @(posedge clk) begin
    if (go && i == 0) begin
      // initialize all the variables.
      a <= in;
      q <= 0;
      i <= 1;
      left <= 0;   // input to adder/sub
      right <= 0;  // input to adder/sub
      r <= 0;      // remainder
    // run the calculations for 16 iterations.
    end else if (go && i <= 16) begin
      right <= {q,r[17],1'b1};
      left  <= {r[15:0],a[31:30]};
      a <= {a[29:0], 2'b00};    //left shift by 2 bits.

      if (r[17] == 1) //add if r is negative
          r <= left + right;
      else    //subtract if r is positive
          r <= left - right;

      q <= {q[14:0],!r[17]};
      if (i == 16) begin
        out <= {16'd0,q};   //final assignment of output.
        i <= 0;
        done <= 1;
      end else
        i <= i + 1;
    end else begin
      // initialize all the variables.
      a <= in;
      q <= 0;
      i <= 0;
      left <= 0;   // input to adder/sub
      right <= 0;  // input to adder/sub
      r <= 0;      // remainder
      done <= 0;
    end
  end
endmodule

///signed fixedpoint
  module fixed_p_std_sadd
    #(parameter width= 32,
    parameter int_width= 8,
    parameter fract_width= 24)

    (input signed [width-1:0] left,
    input signed [width-1:0] right,
    output signed [width-1:0] out);

    assign out = left + right;
  endmodule

module fixed_p_std_ssub
  #(parameter width= 32,
    parameter int_width= 8,
    parameter fract_width= 24)

   (input signed [width-1:0] left,
    input signed [width-1:0] right,
    output signed [width-1:0] out);

  assign out = left -right;
endmodule

module fixed_p_std_smult
  #(parameter width= 32,
  parameter int_width= 8,
  parameter fract_width= 24)

  (input signed [width-1:0] left,
  input signed [width-1:0] right,
  output signed [width-1:0] out);

  logic [2*width-2:0] result;

  assign result = left * right;
  assign out = result[width+fract_width-1:fract_width];
endmodule

module fixed_p_std_sdiv
  #(parameter width= 32,
    parameter int_width= 8,
    parameter fract_width= 24)
  (input signed [width-1:0] left,
   input signed [width-1:0] right,
   output signed [width-1:0] out);

  logic [2*width-2:0] result;

  assign result = left / right;
  assign out = result[width+fract_width-1:fract_width];
endmodule

module sfixed_p_std_add_dbit
  #(parameter width1= 32,
  parameter width2= 32,
  parameter int_width1 = 8,
  parameter fract_width1 = 24,
  parameter int_width2 = 4,
  parameter fract_width2 =28,
  parameter out_width = 36)

  (input logic [width1-1:0] left,
  input logic [width2-1:0] right,
  output logic [out_width-1:0] out);

  logic [int_width1-1:0] left_int;
  logic [int_width2-1:0] right_int;
  logic [fract_width1-1:0] left_fract;
  logic [fract_width2-1:0] right_fract;

  localparam bigint = (int_width1 >= int_width2) ? int_width1 : int_width2;
  localparam bigfract = (fract_width1 >= fract_width2) ? fract_width1 : fract_width2;

  logic [bigint-1:0]  mod_right_int;
  logic [bigfract-1:0] mod_left_fract;

  logic [bigint-1:0]  whole_int;
  logic [ bigfract-1:0] whole_fract;

  assign {left_int, left_fract} = left;
  assign {right_int, right_fract} = right;

  assign mod_left_fract = left_fract * (2**(fract_width2-fract_width1));

  always_comb begin
    if ((mod_left_fract + right_fract) >= 2**fract_width2) begin
      whole_int = left_int + right_int+1;
      whole_fract =mod_left_fract + right_fract-2**fract_width2;
    end
    else begin
      whole_int = left_int + right_int;
      whole_fract =mod_left_fract + right_fract;
    end
  end

  assign out = {whole_int, whole_fract};
endmodule

module fixed_p_std_sgt
 #(parameter width = 32,
   parameter int_width = 8,
   parameter fract_width = 24)
   (input  logic signed [width-1:0] left,
    input  logic signed [width-1:0] right,
    output logic signed             out);
  assign out = left > right;
endmodule
