#include "header_fixed.h"
#include <iostream>
#include <vector>
#include <fstream>

int main(int argc, char* argv[]) {
  if (argc < 2) {
    std::cerr << "Error: Please provide the flat memory data file." << std::endl;
    return 1;
  }

  class main hardware; 

  // Initialize Memory
  std::ifstream file(argv[1]);
  std::vector<uint32_t> mem = {};
  uint32_t val;
  while (file >> std::hex >> val) {
    mem.push_back(val);
  }

  // Initial Ports
  hardware.view.go = 1;
  hardware.view.reset = 0;

  uint8_t delayed_mem_done = 0;

  int max_cycles = 1500;
  int cycle = 1;
  for (; cycle < max_cycles; cycle++) {
    
    // Clock Low
    hardware.view.clk = 0;
    hardware.eval(); 

    uint8_t addr = hardware.view.mem_addr0;
    
    // Handle Reads
    if (addr < mem.size()) {
      hardware.view.mem_read_data = mem[addr];
    }

    // Use the 'done' signal from the previous cycle
    hardware.view.mem_done = delayed_mem_done;
    hardware.eval(); 

    bool is_writing = hardware.view.mem_write_en;
    uint32_t w_data = hardware.view.mem_write_data;

    // Handle write
    if (is_writing && addr < mem.size()) {
      mem[addr] = w_data;
    }
    
    // Clock High
    hardware.view.clk = 1;       
    hardware.eval();

    delayed_mem_done = is_writing ? 1 : 0;

    if (hardware.view.done == 1) {
      break;
    }
  }

  std::cout << "{\n";
  std::cout << "  \"cycles\": " << cycle << ",\n";
  std::cout << "  \"memories\": {\n";
  std::cout << "    \"mem\": [\n";
  for (size_t i = 0; i < mem.size(); ++i) {
    std::cout << "      " << mem[i] << (i == mem.size() - 1 ? "" : ",") << "\n";
  }
  std::cout << "    ]\n";
  std::cout << "  }\n";
  std::cout << "}\n";

  return 0;
}