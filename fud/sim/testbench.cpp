#include "Vmain.h"
#include "verilated.h"
#include "verilated_vcd_c.h"
#include <stdio.h>
#include <memory>

// Keep track of time: https://www.veripool.org/wiki/verilator/Manual-verilator#CONNECTING-TO-C
vluint64_t main_time = 0;
double sc_time_stamp() { return main_time; }

int main(int argc, char **argv, char **env) {
  int i = 0;
  int clk;
  Verilated::commandArgs(argc, argv);

  // init top verilog instance
  std::unique_ptr<Vmain> top = std::make_unique<Vmain>();

  // get cycles to simulate
  int n_cycles = 5e8;
  if (argc >= 3) {
    n_cycles = std::stoi(argv[2]);
  }

  // init trace dump
  bool trace = false;
  if (argc >= 4) {
    trace = std::strcmp(argv[3], "--trace") == 0;
  }

  std::unique_ptr<VerilatedVcdC> tfp;
  if (trace) {
    Verilated::traceEverOn(true);
    tfp = std::make_unique<VerilatedVcdC>();
    top->trace(tfp.get(), 99);
    tfp->open(argv[1]);
  }

  // initialize simulation inputs and eval once to avoid zero-time reset bug
  // (https://github.com/verilator/verilator/issues/2661)
  top->go = 0;
  top->clk = 0;
  top->reset = 1;
  top->eval();

  int done = 0;
  int ignore_cycles = 5;
  //printf("Starting simulation\n");
  while (done == 0 && i < n_cycles) {
    done = top->done;
    // Do nothing for a few cycles to avoid zero-time reset bug
    if (ignore_cycles == 0) {
      top->go = 1;
      top->reset = 0;
    } else {
      top->reset = 1;
      ignore_cycles--;
    }
    // dump variables into VCD file and toggle clock
    for (clk = 0; clk < 2; clk++) {
      if (trace && ignore_cycles == 0) {
        tfp->dump(2 * i + clk);
      }
      top->clk = !top->clk;
      top->eval();
    }

    // Time passes
    main_time++;

    if (Verilated::gotFinish())
      exit(0);

    i++;
  }

  printf("Simulated %i cycles\n", i - ignore_cycles);
  top->final();
  if (trace) {
    tfp->close();
  }

  exit(0);
}
