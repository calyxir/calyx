/**
* Signed arthimetic primitives for FuTIL.
*/

module std_slsh #(
    parameter width = 32
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed [width-1:0] out
);
  assign out = left <<< right;
endmodule

module std_srsh #(
    parameter width = 32
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed [width-1:0] out
);
  assign out = left >>> right;
endmodule

module std_sadd #(
    parameter width = 32
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed [width-1:0] out
);
  assign out = $signed(left + right);
endmodule

module std_ssub #(
    parameter width = 32
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed [width-1:0] out
);
  assign out = $signed(left - right);
endmodule

module std_smult_pipe #(
    parameter width = 32
) (
    input  logic                    go,
    input  logic                    clk,
    input  signed       [width-1:0] left,
    input  signed       [width-1:0] right,
    output logic signed [width-1:0] out,
    output logic                    done
);
  logic signed [width-1:0] rtmp;
  logic signed [width-1:0] ltmp;
  logic signed [width-1:0] out_tmp;
  logic done_buf[1:0];

  always_ff @(posedge clk) begin
    if (go) begin
      rtmp <= right;
      ltmp <= left;
      out_tmp <= $signed(rtmp * ltmp);
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

module std_sgt #(
    parameter width = 32
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed             out
);
  assign out = $signed(left > right);
endmodule

module std_slt #(
    parameter width = 32
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed             out
);
  assign out = $signed(left < right);
endmodule

module std_seq #(
    parameter width = 32
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed             out
);
  assign out = $signed(left == right);
endmodule

module std_sneq #(
    parameter width = 32
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed             out
);
  assign out = $signed(left != right);
endmodule

module std_sge #(
    parameter width = 32
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed             out
);
  assign out = $signed(left >= right);
endmodule

module std_sle #(
    parameter width = 32
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed             out
);
  assign out = $signed(left <= right);
endmodule

module std_sdiv #(
    parameter width = 32
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed [width-1:0] out
);
  assign out = $signed(left / right);
endmodule

module std_smod #(
    parameter width = 32
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed [width-1:0] out
);
  assign out = $signed(left % right);
endmodule


module std_smult #(
    parameter width = 32
) (
    input  signed [width-1:0] left,
    input  signed [width-1:0] right,
    output signed [width-1:0] out
);
  assign out = $signed(left * right);
endmodule

