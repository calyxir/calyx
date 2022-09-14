#include "Vmain.h"
#include "verilated.h"
#include "verilated_vcd_c.h"
#include <algorithm>
#include <array>
#include <cstdint>
#include <iostream>
#include <memory>

// Keep track of time:
// https://www.veripool.org/wiki/verilator/Manual-verilator#CONNECTING-TO-C
vluint64_t GLOBAL_sc_time = 0;
// Accessed by the underlying test bench.
double sc_time_stamp() { return GLOBAL_sc_time; }

// This shows simulation progress by printing the number of cycles at certain
// intervals.
void cycle_tracker(const uint64_t current_cycle, const uint64_t n_cycles) {
  auto print_simulation_at = [&](int64_t cycle) {
    std::cout << "[Verilator] In-progress: Simulated " << cycle << " cycles\n";
  };

  const std::array<uint64_t, 3> intervals = {n_cycles * 1 / 4, n_cycles * 2 / 4,
                                             n_cycles * 3 / 4};
  if (std::any_of(intervals.begin(), intervals.end(),
                  [&](uint64_t i) { return current_cycle == i; })) {
    print_simulation_at(current_cycle);
  }
}

// Expected program arguments:
// argv[1]: Input file path for the trace file.
// argv[2]: Number of cycles.
// argv[3]: `--trace` if the trace is requested for VCD dump.
int main(int argc, char **argv) {

  Verilated::commandArgs(argc, argv);
  // Initialize top Verilog instance.
  std::unique_ptr<Vmain> top = std::make_unique<Vmain>();

  // Number of cycles for simulation. Defaulted to 5e8 if none provided.
  const uint64_t n_cycles = argc >= 3 ? std::stoull(argv[2]) : 5e8;

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

  // Do nothing for 4 cycles to avoid zero-time reset bug:
  // https://github.com/verilator/verilator/issues/2661
  constexpr int8_t ResetCycles = 4;
  for (uint8_t i = 0; i < ResetCycles; ++i)
    top->reset = 1;

  // Drive the top-level module.
  top->reset = 0;
  top->go = 1;

  uint64_t cycles = 0;
  for (; cycles < n_cycles && top->done == 0; ++cycles, ++GLOBAL_sc_time) {
    cycle_tracker(cycles, n_cycles);

    if (trace_requested)
      tfp->dump(static_cast<vluint64_t>(2 * cycles + 0));
    // Toggle the clock (positive edge) and evaluate.
    top->clk = 1;
    top->eval();

    if (trace_requested)
      tfp->dump(static_cast<vluint64_t>(2 * cycles + 1));
    // Toggle the clock (negative edge) and evaluate.
    top->clk = 0;
    top->eval();
  }

  const bool is_unfinished = cycles == n_cycles && top->done == 0;
  std::cout << "[Verilator]"
            << (is_unfinished ? " ERROR: Program reached limit of "
                              : " Simulated ")
            << cycles << " cycles\n";
  top->final();
  if (trace_requested)
    tfp->close();
}
