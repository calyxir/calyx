import argparse
import json


def get_flat_index(dims, name):
    if len(dims) == 1:
        return f"hardware.view.{name}_addr0"

    addr_exprs = [f"hardware.view.{name}_addr{i}" for i in range(len(dims))]
    terms, stride = [], 1
    for i in reversed(range(len(dims))):
        terms.insert(
            0, addr_exprs[i] if stride == 1 else f"({addr_exprs[i]} * {stride})"
        )
        stride *= dims[i]
    return " + ".join(terms)


def generate_driver(yxi_path, template_out):
    with open(yxi_path, "r") as f:
        yxi = json.load(f)

    memories = yxi.get("memories", [])

    cpp = [
        """#include "header.h"
#include "json.hpp"
#include <iostream>
#include <vector>
#include <fstream>

using json = nlohmann::ordered_json;

static json reshape(
    const std::vector<uint32_t>& flat,
    const std::vector<size_t>& dims,
    size_t dim_idx = 0,
    size_t offset = 0
) {
  if (dims.empty()) {
    return json::array();
  }
  if (dim_idx >= dims.size() - 1) {
    json arr = json::array();
    for (size_t i = 0; i < dims[dim_idx]; ++i) {
      size_t idx = offset + i;
      arr.push_back(idx < flat.size() ? flat[idx] : 0);
    }
    return arr;
  }
  size_t stride = 1;
  for (size_t k = dim_idx + 1; k < dims.size(); ++k) stride *= dims[k];
  json arr = json::array();
  for (size_t i = 0; i < dims[dim_idx]; ++i) {
    arr.push_back(reshape(flat, dims, dim_idx + 1, offset + i * stride));
  }
  return arr;
}

int main(int argc, char* argv[]) {
  class main hardware;

  if (argc < 2) {
    std::cerr << "Error: Please provide the directory containing the .dat files.\\n";
    return 1;
  }
  std::string data_dir = argv[1];
"""
    ]

    # 1. Initialization and State Variables
    for mem in memories:
        name = mem["name"]
        is_seq = mem["memory_type"] == "Sequential"
        total_size = mem["total_size"]

        read_data_decl = (
            f"uint32_t {name}_read_data_reg = 0;" if is_seq else ""
        )
        cpp.append(
            f"""
  // Initialize {name}
  std::vector<uint32_t> mem_{name}({total_size}, 0);
  std::string path_{name} = data_dir + "/{name}.dat";
  std::ifstream file_{name}(path_{name});
  if (file_{name}.is_open()) {{
    uint32_t val_{name};
    int idx_{name} = 0;
    while (file_{name} >> std::hex >> val_{name} && idx_{name} < {total_size}) {{
      mem_{name}[idx_{name}++] = val_{name};
    }}
  }} else {{
    std::cerr << "WARNING: Could not open "
              << path_{name} << "! Initializing empty.\\n";
  }}
  uint8_t {name}_done_reg = 0;
  {read_data_decl}
"""
        )

    # 2. Main Loop Setup
    cpp.append(
        """
  // Initial Ports
  hardware.view.go = 1;
  hardware.view.reset = 0;

  int max_cycles = 1500;
  int cycle = 1;
  for (; cycle < max_cycles; cycle++) {
    hardware.view.clk = 0;
    hardware.eval();

    // Apply Outputs Phase
"""
    )

    # Apply Outputs Phase
    for mem in memories:
        name = mem["name"]
        is_seq = mem["memory_type"] == "Sequential"
        dims = mem["dimension_sizes"]
        flat_idx = get_flat_index(dims, name)

        if is_seq:
            cpp.append(
                f"""    hardware.view.{name}_read_data = {name}_read_data_reg;
    hardware.view.{name}_done = {name}_done_reg;"""
            )
        else:
            total = mem["total_size"]
            cpp.append(
                f"""    uint32_t addr_{name} = {flat_idx};
    hardware.view.{name}_read_data = (addr_{name} < {total})
        ? mem_{name}[addr_{name}] : 0;
    hardware.view.{name}_done = {name}_done_reg;"""
            )

    cpp.append(
        """
    hardware.eval();

    // Capture Inputs Phase
"""
    )

    # Capture Inputs Phase
    for mem in memories:
        name = mem["name"]
        if mem["memory_type"] == "Sequential":
            flat_idx = get_flat_index(mem["dimension_sizes"], name)
            cpp.append(
                f"""    uint32_t addr_{name} = {flat_idx};
    bool content_en_{name} = hardware.view.{name}_content_en;"""
            )

        cpp.append(
            f"""    bool write_en_{name} = hardware.view.{name}_write_en;
    uint32_t write_data_{name} = hardware.view.{name}_write_data;"""
        )

    cpp.append(
        """
    // Clock High
    hardware.view.clk = 1;
    hardware.eval();

    // Update State Phase
"""
    )

    # Update State Phase
    for mem in memories:
        name = mem["name"]
        is_seq = mem["memory_type"] == "Sequential"
        total_size = mem["total_size"]

        if is_seq:
            cpp.append(
                f"""    if (content_en_{name}) {{
      if (write_en_{name} && addr_{name} < {total_size}) {{
        mem_{name}[addr_{name}] = write_data_{name};
      }}
      {name}_read_data_reg = (addr_{name} < {total_size})
          ? mem_{name}[addr_{name}] : 0;
      {name}_done_reg = 1;
    }} else {{
      {name}_done_reg = 0;
    }}"""
            )
        else:
            cpp.append(
                f"""    if (write_en_{name}) {{
      if (addr_{name} < {total_size}) {{
        mem_{name}[addr_{name}] = write_data_{name};
      }}
      {name}_done_reg = 1;
    }} else {{
      {name}_done_reg = 0;
    }}"""
            )

    cpp.append(
        """
    if (hardware.view.done == 1) break;
  }

  // Output JSON
  json root;
  root["cycles"] = cycle;
  root["memories"] = json::object();
"""
    )

    for mem in memories:
        name = mem["name"]
        dims = mem["dimension_sizes"]
        dim_str = ", ".join(str(d) for d in dims)
        cpp.append(
            f'  root["memories"]["{name}"] = reshape(mem_{name}, {{{dim_str}}});'
        )

    cpp.append(
        """
  std::cout << root.dump(2) << std::endl;

  return 0;
}
"""
    )

    with open(template_out, "w") as f:
        f.write("\n".join(cpp))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--yxi", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    generate_driver(args.yxi, args.out)
