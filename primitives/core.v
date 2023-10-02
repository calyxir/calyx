`default_nettype none
module std_slice (
	in,
	out
);
	parameter IN_WIDTH = 32;
	parameter OUT_WIDTH = 32;
	input wire [IN_WIDTH - 1:0] in;
	output wire [OUT_WIDTH - 1:0] out;
	assign out = in[OUT_WIDTH - 1:0];
endmodule
module std_pad (
	in,
	out
);
	parameter IN_WIDTH = 32;
	parameter OUT_WIDTH = 32;
	input wire [IN_WIDTH - 1:0] in;
	output wire [OUT_WIDTH - 1:0] out;
	localparam EXTEND = OUT_WIDTH - IN_WIDTH;
	assign out = {{EXTEND {1'b0}}, in};
endmodule
module std_cat (
	left,
	right,
	out
);
	parameter LEFT_WIDTH = 32;
	parameter RIGHT_WIDTH = 32;
	parameter OUT_WIDTH = 64;
	input wire [LEFT_WIDTH - 1:0] left;
	input wire [RIGHT_WIDTH - 1:0] right;
	output wire [OUT_WIDTH - 1:0] out;
	assign out = {left, right};
endmodule
module std_not (
	in,
	out
);
	parameter WIDTH = 32;
	input wire [WIDTH - 1:0] in;
	output wire [WIDTH - 1:0] out;
	assign out = ~in;
endmodule
module std_and (
	left,
	right,
	out
);
	parameter WIDTH = 32;
	input wire [WIDTH - 1:0] left;
	input wire [WIDTH - 1:0] right;
	output wire [WIDTH - 1:0] out;
	assign out = left & right;
endmodule
module std_or (
	left,
	right,
	out
);
	parameter WIDTH = 32;
	input wire [WIDTH - 1:0] left;
	input wire [WIDTH - 1:0] right;
	output wire [WIDTH - 1:0] out;
	assign out = left | right;
endmodule
module std_xor (
	left,
	right,
	out
);
	parameter WIDTH = 32;
	input wire [WIDTH - 1:0] left;
	input wire [WIDTH - 1:0] right;
	output wire [WIDTH - 1:0] out;
	assign out = left ^ right;
endmodule
module std_sub (
	left,
	right,
	out
);
	parameter WIDTH = 32;
	input wire [WIDTH - 1:0] left;
	input wire [WIDTH - 1:0] right;
	output wire [WIDTH - 1:0] out;
	assign out = left - right;
endmodule
module std_gt (
	left,
	right,
	out
);
	parameter WIDTH = 32;
	input wire [WIDTH - 1:0] left;
	input wire [WIDTH - 1:0] right;
	output wire out;
	assign out = left > right;
endmodule
module std_lt (
	left,
	right,
	out
);
	parameter WIDTH = 32;
	input wire [WIDTH - 1:0] left;
	input wire [WIDTH - 1:0] right;
	output wire out;
	assign out = left < right;
endmodule
module std_eq (
	left,
	right,
	out
);
	parameter WIDTH = 32;
	input wire [WIDTH - 1:0] left;
	input wire [WIDTH - 1:0] right;
	output wire out;
	assign out = left == right;
endmodule
module std_neq (
	left,
	right,
	out
);
	parameter WIDTH = 32;
	input wire [WIDTH - 1:0] left;
	input wire [WIDTH - 1:0] right;
	output wire out;
	assign out = left != right;
endmodule
module std_ge (
	left,
	right,
	out
);
	parameter WIDTH = 32;
	input wire [WIDTH - 1:0] left;
	input wire [WIDTH - 1:0] right;
	output wire out;
	assign out = left >= right;
endmodule
module std_le (
	left,
	right,
	out
);
	parameter WIDTH = 32;
	input wire [WIDTH - 1:0] left;
	input wire [WIDTH - 1:0] right;
	output wire out;
	assign out = left <= right;
endmodule
module std_lsh (
	left,
	right,
	out
);
	parameter WIDTH = 32;
	input wire [WIDTH - 1:0] left;
	input wire [WIDTH - 1:0] right;
	output wire [WIDTH - 1:0] out;
	assign out = left << right;
endmodule
module std_rsh (
	left,
	right,
	out
);
	parameter WIDTH = 32;
	input wire [WIDTH - 1:0] left;
	input wire [WIDTH - 1:0] right;
	output wire [WIDTH - 1:0] out;
	assign out = left >> right;
endmodule
module std_mux (
	cond,
	tru,
	fal,
	out
);
	parameter WIDTH = 32;
	input wire cond;
	input wire [WIDTH - 1:0] tru;
	input wire [WIDTH - 1:0] fal;
	output wire [WIDTH - 1:0] out;
	assign out = (cond ? tru : fal);
endmodule
module std_mem_d1 (
	addr0,
	write_data,
	write_en,
	clk,
	reset,
	read_data,
	done
);
	parameter WIDTH = 32;
	parameter SIZE = 16;
	parameter IDX_SIZE = 4;
	input wire [IDX_SIZE - 1:0] addr0;
	input wire [WIDTH - 1:0] write_data;
	input wire write_en;
	input wire clk;
	input wire reset;
	output wire [WIDTH - 1:0] read_data;
	output reg done;
	reg [WIDTH - 1:0] mem [SIZE - 1:0];
	assign read_data = mem[addr0];
	always @(posedge clk)
		if (reset)
			done <= 1'sb0;
		else if (write_en)
			done <= 1'sb1;
		else
			done <= 1'sb0;
	always @(posedge clk)
		if (!reset && write_en)
			mem[addr0] <= write_data;
endmodule
module std_mem_d2 (
	addr0,
	addr1,
	write_data,
	write_en,
	clk,
	reset,
	read_data,
	done
);
	parameter WIDTH = 32;
	parameter D0_SIZE = 16;
	parameter D1_SIZE = 16;
	parameter D0_IDX_SIZE = 4;
	parameter D1_IDX_SIZE = 4;
	input wire [D0_IDX_SIZE - 1:0] addr0;
	input wire [D1_IDX_SIZE - 1:0] addr1;
	input wire [WIDTH - 1:0] write_data;
	input wire write_en;
	input wire clk;
	input wire reset;
	output wire [WIDTH - 1:0] read_data;
	output reg done;
	reg [WIDTH - 1:0] mem [D0_SIZE - 1:0][D1_SIZE - 1:0];
	assign read_data = mem[addr0][addr1];
	always @(posedge clk)
		if (reset)
			done <= 1'sb0;
		else if (write_en)
			done <= 1'sb1;
		else
			done <= 1'sb0;
	always @(posedge clk)
		if (!reset && write_en)
			mem[addr0][addr1] <= write_data;
endmodule
module std_mem_d3 (
	addr0,
	addr1,
	addr2,
	write_data,
	write_en,
	clk,
	reset,
	read_data,
	done
);
	parameter WIDTH = 32;
	parameter D0_SIZE = 16;
	parameter D1_SIZE = 16;
	parameter D2_SIZE = 16;
	parameter D0_IDX_SIZE = 4;
	parameter D1_IDX_SIZE = 4;
	parameter D2_IDX_SIZE = 4;
	input wire [D0_IDX_SIZE - 1:0] addr0;
	input wire [D1_IDX_SIZE - 1:0] addr1;
	input wire [D2_IDX_SIZE - 1:0] addr2;
	input wire [WIDTH - 1:0] write_data;
	input wire write_en;
	input wire clk;
	input wire reset;
	output wire [WIDTH - 1:0] read_data;
	output reg done;
	reg [WIDTH - 1:0] mem [D0_SIZE - 1:0][D1_SIZE - 1:0][D2_SIZE - 1:0];
	assign read_data = mem[addr0][addr1][addr2];
	always @(posedge clk)
		if (reset)
			done <= 1'sb0;
		else if (write_en)
			done <= 1'sb1;
		else
			done <= 1'sb0;
	always @(posedge clk)
		if (!reset && write_en)
			mem[addr0][addr1][addr2] <= write_data;
endmodule
module std_mem_d4 (
	addr0,
	addr1,
	addr2,
	addr3,
	write_data,
	write_en,
	clk,
	reset,
	read_data,
	done
);
	parameter WIDTH = 32;
	parameter D0_SIZE = 16;
	parameter D1_SIZE = 16;
	parameter D2_SIZE = 16;
	parameter D3_SIZE = 16;
	parameter D0_IDX_SIZE = 4;
	parameter D1_IDX_SIZE = 4;
	parameter D2_IDX_SIZE = 4;
	parameter D3_IDX_SIZE = 4;
	input wire [D0_IDX_SIZE - 1:0] addr0;
	input wire [D1_IDX_SIZE - 1:0] addr1;
	input wire [D2_IDX_SIZE - 1:0] addr2;
	input wire [D3_IDX_SIZE - 1:0] addr3;
	input wire [WIDTH - 1:0] write_data;
	input wire write_en;
	input wire clk;
	input wire reset;
	output wire [WIDTH - 1:0] read_data;
	output reg done;
	reg [WIDTH - 1:0] mem [D0_SIZE - 1:0][D1_SIZE - 1:0][D2_SIZE - 1:0][D3_SIZE - 1:0];
	assign read_data = mem[addr0][addr1][addr2][addr3];
	always @(posedge clk)
		if (reset)
			done <= 1'sb0;
		else if (write_en)
			done <= 1'sb1;
		else
			done <= 1'sb0;
	always @(posedge clk)
		if (!reset && write_en)
			mem[addr0][addr1][addr2][addr3] <= write_data;
endmodule
`default_nettype wire
