module dual_port_mem_d1 #(
  parameter WIDTH=32,
  parameter SIZE=16,
  parameter IDX_SIZE=4
) (
  // Common signals
   input wire logic clk,
   input wire logic reset,
  
  // Read signal 
  input wire logic read_en,
  input wire logic addr0_r,
  output logic [ WIDTH-1:0] read_data,

  // Write signal
  input wire logic write_en,
  input wire logic [ WIDTH-1: 0] write_data,
  input wire logic addr0_w

);

// Internal memory
  (* ram_style = "ultra" *)  logic [WIDTH-1:0] mem[SIZE-1:0];

// Register for the read output
  logic [WIDTH-1:0] read_out;
  assign read_data = read_out;

// Read value from the memory
  always_ff @(posedge clk) begin
    if (reset) begin
      read_out <= '0;
    end else if (read_en) begin
      /* verilator lint_off WIDTH */
      read_out <= mem[addr0_r];
    end else begin
      read_out <= read_out;
    end
  end

// Write value to the memory
  always_ff @(posedge clk) begin
    if (!reset && write_en)
      mem[addr0_w] <= write_data;
  end

// Check for out of bounds access
  `ifdef VERILATOR
    always_comb begin
      if (read_en)
        if (addr0_r >= SIZE)
          $error(
            "dual_port_mem_d1: Out of bounds access\n",
            "addr0_r: %0d\n", addr0_r,
            "SIZE: %0d", SIZE
          );
    end
    always_comb begin
      if (write_en)
        if(addr0_w >= SIZE)
          $error(
            "dual_port_mem_d1: Out of bounds access\n",
            "addr0_w: %0d\n", addr0_w,
            "SIZE: %0d", SIZE
          );
    end
  `endif
  
endmodule

module dual_port_mem_d2 #(
  parameter WIDTH = 32,
  parameter D0_SIZE = 16,
  parameter D1_SIZE = 16,
  parameter D0_IDX_SIZE = 4,
  parameter D1_IDX_SIZE = 4
) (
  // Common signals
   input wire logic clk,
   input wire logic reset,

  // Read signal
   input wire logic read_en,
   output logic [WIDTH-1:0] read_data,
   input wire logic [D0_IDX_SIZE-1:0] addr0_r,
   input wire logic [D1_IDX_SIZE-1:0] addr1_r,

  // Write signals
   input wire logic write_en,
   input wire logic [ WIDTH-1:0] write_data,
   input wire logic [D0_IDX_SIZE-1:0] addr0_w,
   input wire logic [D1_IDX_SIZE-1:0] addr1_w


);

  wire [D0_IDX_SIZE+D1_IDX_SIZE-1:0] addr_r;
  assign addr_r = addr0_r * D1_SIZE + addr1_r;
  wire [D0_IDX_SIZE+D1_IDX_SIZE-1:0] addr_w;
  assign addr_w = addr0_w * D1_SIZE + addr1_w;

  dual_port_mem_d1 #(.WIDTH(WIDTH), .SIZE(D0_SIZE * D1_SIZE), .IDX_SIZE(D0_IDX_SIZE+D1_IDX_SIZE)) mem
     (.clk(clk), .reset(reset),
    .read_en(read_en), .read_data(read_data), .addr0_r(addr_r), .write_data(write_data), .write_en(write_en),
    .addr0_w(addr0_w));

endmodule

module dual_port_mem_d3 #(
  parameter WIDTH = 32,
    parameter D0_SIZE = 16,
    parameter D1_SIZE = 16,
    parameter D2_SIZE = 16,
    parameter D0_IDX_SIZE = 4,
    parameter D1_IDX_SIZE = 4,
    parameter D2_IDX_SIZE = 4
) (
  // Common signals
   input wire logic clk,
   input wire logic reset,

  // Read signal
   input wire logic read_en,
   output logic [WIDTH-1:0] read_data,
   input wire logic [D0_IDX_SIZE-1:0] addr0_r,
   input wire logic [D1_IDX_SIZE-1:0] addr1_r,
   input wire logic [D2_IDX_SIZE-1:0] addr2_r,

  // Write signals
   input wire logic write_en,
   input wire logic [ WIDTH-1:0] write_data,
   input wire logic [D0_IDX_SIZE-1:0] addr0_w,
   input wire logic [D1_IDX_SIZE-1:0] addr1_w,
   input wire logic [D2_IDX_SIZE-1:0] addr2_w
);

  wire [D0_IDX_SIZE+D1_IDX_SIZE+D2_IDX_SIZE-1:0] addr_r;
  assign addr_r = addr0_r * (D1_SIZE * D2_SIZE) + addr1_r * (D2_SIZE) + addr2_r;

  wire [D0_IDX_SIZE+D1_IDX_SIZE+D2_IDX_SIZE-1:0] addr_w;
  assign addr_w = addr0_w * (D1_SIZE * D2_SIZE) + addr1_w * (D2_SIZE) + addr2_w;

  dual_port_mem_d1 #(.WIDTH(WIDTH), .SIZE(D0_SIZE * D1_SIZE * D2_SIZE), .IDX_SIZE(D0_IDX_SIZE+D1_IDX_SIZE+D2_IDX_SIZE)) mem
     (.clk(clk), .reset(reset),
    .read_en(read_en), .read_data(read_data), .addr0_r(addr_r), .write_data(write_data), .write_en(write_en),
    .addr0_w(addr_w));
  
endmodule

module dual_port_mem_d4 #(
    parameter WIDTH = 32,
    parameter D0_SIZE = 16,
    parameter D1_SIZE = 16,
    parameter D2_SIZE = 16,
    parameter D3_SIZE = 16,
    parameter D0_IDX_SIZE = 4,
    parameter D1_IDX_SIZE = 4,
    parameter D2_IDX_SIZE = 4,
    parameter D3_IDX_SIZE = 4
) (
   // Common signals
   input wire logic clk,
   input wire logic reset,

   // Read signal
   input wire logic read_en,
   output logic [WIDTH-1:0] read_data,
   input wire logic [D0_IDX_SIZE-1:0] addr0_r,
   input wire logic [D1_IDX_SIZE-1:0] addr1_r,
   input wire logic [D2_IDX_SIZE-1:0] addr2_r,
   input wire logic [D3_IDX_SIZE-1:0] addr3_r,


   // Write signals
   input wire logic write_en,
   input wire logic [ WIDTH-1:0] write_data,
   input wire logic [D0_IDX_SIZE-1:0] addr0_w,
   input wire logic [D1_IDX_SIZE-1:0] addr1_w,
   input wire logic [D2_IDX_SIZE-1:0] addr2_w,
   input wire logic [D3_IDX_SIZE-1:0] addr3_w

);
  wire [D0_IDX_SIZE+D1_IDX_SIZE+D2_IDX_SIZE+D3_IDX_SIZE-1:0] addr_r;
  assign addr_r = addr_r * (D1_SIZE * D2_SIZE * D3_SIZE) + addr1_r * (D2_SIZE * D3_SIZE) + addr2_r * (D3_SIZE) + addr3_r;

  wire [D0_IDX_SIZE+D1_IDX_SIZE+D2_IDX_SIZE+D3_IDX_SIZE-1:0] addr_w;
  assign addr_w = addr_w * (D1_SIZE * D2_SIZE * D3_SIZE) + addr1_w * (D2_SIZE * D3_SIZE) + addr2_w * (D3_SIZE) + addr3_w;

  dual_port_mem_d1 #(.WIDTH(WIDTH), .SIZE(D0_SIZE * D1_SIZE * D2_SIZE * D3_SIZE), .IDX_SIZE(D0_IDX_SIZE+D1_IDX_SIZE+D2_IDX_SIZE+D3_IDX_SIZE)) mem
     (.clk(clk), .reset(reset),
    .read_en(read_en), .read_data(read_data), .addr0_r(addr_r), .write_data(write_data), .write_en(write_en),
    .addr0_w(addr_w));
endmodule
