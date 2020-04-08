#include "Vmain.h"
#include "verilated.h"
#include "verilated_vcd_c.h"
#include <stdio.h>
int main(int argc, char **argv, char **env) {
  int i;
  int clk;
  Verilated::commandArgs(argc, argv);
  // init top verilog instance
  Vmain *top = new Vmain;
  // init trace dump
  Verilated::traceEverOn(true);
  VerilatedVcdC *tfp = new VerilatedVcdC;
  top->trace(tfp, 99);
  tfp->open(argv[1]);
  // initialize simulation inputs
  top->clk = 1;
  top->valid = 1;
  int ready = 0;
  printf("Starting simulation\n");
  while (ready == 0) {
    ready = top->ready;
    // dump variables into VCD file and toggle clock
    for (clk = 0; clk < 2; clk++) {
      tfp->dump(2 * i + clk);
      top->clk = !top->clk;
      top->eval();
    }

    if (Verilated::gotFinish())
      exit(0);

    i++;
  }
  printf("Simulated %i cycles\n", i);
  top->final();
  tfp->close();
  exit(0);
}
