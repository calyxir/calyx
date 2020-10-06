#include "Vmain.h"
#include "verilated.h"
#include "verilated_vcd_c.h"
#include <stdio.h>
int main(int argc, char **argv, char **env) {
  int i = 0;
  int clk;
  Verilated::commandArgs(argc, argv);
  // init top verilog instance
  Vmain *top = new Vmain;
  // init trace dump
  bool trace = false;
  if (argc >= 3) {
    trace = std::strcmp(argv[2], "--trace") == 0;
  }
  printf("Tracing: %d\n", trace);
  VerilatedVcdC *tfp;
  if (trace) {
    Verilated::traceEverOn(true);
    tfp = new VerilatedVcdC;
    top->trace(tfp, 99);
    tfp->open(argv[1]);
  }
  // initialize simulation inputs
  top->clk = 0;
  top->go = 1;
  int done = 0;
  printf("Starting simulation\n");
  while (done == 0 && i < 5e8) {
    done = top->done;
    // dump variables into VCD file and toggle clock
    for (clk = 0; clk < 2; clk++) {
      if (trace) {
        tfp->dump(2 * i + clk);
      }
      top->clk = !top->clk;
      top->eval();
    }

    if (Verilated::gotFinish())
      exit(0);

    i++;
  }
  printf("Simulated %i cycles\n", i);
  top->final();
  if (trace) {
    tfp->close();
  }
  exit(0);
}
