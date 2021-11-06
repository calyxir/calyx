#include "Vmain.h"
#include "verilated.h"
#include "verilated_vcd_c.h"
#include <cstdint>
#include <iostream>
#include <memory>

// Keep track of time:
// https://www.veripool.org/wiki/verilator/Manual-verilator#CONNECTING-TO-C
vluint64_t MainTime = 0;
// Accessed by the underlying test bench.
double sc_time_stamp() { return MainTime; }

// Expected program arguments:
// argv[1]: Input file path for the trace file.
// argv[2]: Number of cycles.
// argv[3]: `--trace` if the trace is requested for VCD dump.
int main(int argc, char **argv, char **env) {

  Verilated::commandArgs(argc, argv);
  // Initialize top Verilog instance.
  auto top = std::make_unique<Vmain>();

  // Number of cycles for simulation. Defaulted to 5e8 if none provided.
  const int64_t n_cycles = argc >= 3 ? std::stoi(argv[2]) : 5e8;

  // Initialize trace dump, used for VCD output.
  const bool trace_requested =
      argc >= 4 && std::strcmp(argv[3], "--trace") == 0;
  std::unique_ptr<VerilatedVcdC> tfp;
  if (trace_requested) {
    std::cout << "[VCD] trace turned on.\n";
    Verilated::traceEverOn(true);
    tfp = std::make_unique<VerilatedVcdC>();
    top->trace(tfp.get(), 99);
    tfp->open(argv[1]);
  }

  // Initialize simulation.
  std::cout << "[Verilator] Simulation begin\n";
  top->go = 0;
  top->clk = 0;
  top->reset = 1;
  top->eval();

  // Do nothing for 5 cycles to avoid zero-time reset bug:
  // (https://github.com/verilator/verilator/issues/2661)
  constexpr int8_t IgnoreCycles = 5;
  for (int8_t i = 0; i < IgnoreCycles; ++i)
    top->reset = 1;

  // Start the top-level module.
  top->reset = 0;
  top->go = 1;

  int64_t cycles = 0;
  // Check for 3 conditions:
  //   1. Number of cycles less than the upper limit.
  //   2. The top component is not marked done.
  //   3. Verilator simulator has not received a $finish.
  for (; cycles < n_cycles && top->done == 0 && !Verilated::gotFinish();
       ++cycles, ++MainTime) {
    // Toggle the clock and evaluate twice per cycle.
    if (trace_requested)
      // Dump variables into VCD file.
      tfp->dump(static_cast<vluint64_t>(2 * cycles + 0));
    top->clk = !top->clk;
    top->eval();

    if (trace_requested)
      tfp->dump(static_cast<vluint64_t>(2 * cycles + 1));
    top->clk = !top->clk;
    top->eval();
  }

  std::cout << "[Verilator] Simulated " << cycles << " cycles\n";
  top->final();
  if (trace_requested)
    tfp->close();
}
