import os
import sys

template_file = os.path.join(sys.path[0], "custom_tb_template.sv")

def generate_addr_field(idx_size, name, index):
     if (idx_size > 1):
          return f"wire [{idx_size-1}:0] {name}_addr{index};\n"
     else:
          return f"wire {name}_addr{index};\n"

def generate_fields(memory_cell_dicts):
    out_str = ""
    for memory_cell_dict in memory_cell_dicts:
         name = memory_cell_dict['cell-name']
         width = memory_cell_dict['WIDTH']
         module_name = memory_cell_dict['module-name']
         if "d1" in module_name:
              out_str += generate_addr_field(memory_cell_dict['IDX_SIZE'], name, 0)
         elif "d2" in module_name:
              out_str += generate_addr_field(memory_cell_dict['D0_IDX_SIZE'], name, 0)
              out_str += generate_addr_field(memory_cell_dict['D1_IDX_SIZE'], name, 1)
         
         out_str += f"wire [{width-1}:0] {name}_write_data;\n"
         out_str += f"wire [{width-1}:0] {name}_read_data;\n"
         out_str += f"wire {name}_write_en;\n"
         if "comb" in module_name:
              out_str += f"wire {name}_done;\n\n"
         else: # seq has more ports.
              out_str += f'''wire {name}_read_en;
wire {name}_write_done;
wire {name}_read_done;\n\n'''

    return out_str

def generate_memory_dec(memory_cell_dicts):
     out_str = ""
     for memory_cell_dict in memory_cell_dicts:
          module_name = memory_cell_dict['module-name']
          name = memory_cell_dict['cell-name']
          out_str += f"{module_name} # (\n"
          if "d1" in module_name:
               out_str += f'''    .IDX_SIZE({memory_cell_dict["IDX_SIZE"]}),
    .SIZE({memory_cell_dict["SIZE"]}),
    .WIDTH({memory_cell_dict["WIDTH"]})
    ) {name} (\n'''
          elif "d2" in module_name:
               out_str += f'''    .D0_IDX_SIZE({memory_cell_dict["D0_IDX_SIZE"]}),
    .D1_IDX_SIZE({memory_cell_dict["D1_IDX_SIZE"]}),
    .D0_SIZE({memory_cell_dict["D0_SIZE"]}),
    .D1_SIZE({memory_cell_dict["D1_SIZE"]}),
    .WIDTH({memory_cell_dict["WIDTH"]})
    ) {name} (
    .addr1({name}_addr1),\n'''
          out_str += f'''    .addr0({name}_addr0),
    .clk(clk),
    .read_data({name}_read_data),
    .reset(reset),
    .write_data({name}_write_data),
    .write_en({name}_write_en),
'''
          if "comb" in module_name:
               out_str += f"    .done({name}_done)\n"
          else:
               out_str += f'''    .read_en({name}_read_en),
    .read_done({name}_read_done),
    .write_done({name}_write_done)
'''
          out_str += ");\n\n"
     return out_str

def generate_main_decl(memory_cell_dicts):
     out_str = '''main #() main (
  .go(go),
  .clk(clk),
  .reset(reset),
  .done(done)'''
     # Documentation: Need to connect to the wires we initialized above (ex. {name}_addr0)
     # and not the ports of the memory (ex. {name},addr0) because verilator would error
     # out due to some of the memory ports being input ports.
     for memory_cell_dict in memory_cell_dicts:
          name = memory_cell_dict["cell-name"]
          module_name = memory_cell_dict["module-name"]
          out_str += ",\n"
          if "d2" in module_name:
               out_str += f"  .{name}_addr1({name}_addr1),\n"
          out_str += f'''  .{name}_addr0({name}_addr0),
  .{name}_write_data({name}_write_data),
  .{name}_read_data({name}_read_data),
  .{name}_write_en({name}_write_en),
'''
          if "comb" in module_name:
               out_str += f"  .{name}_done({name}_done)"
          else:
               out_str += f'''  .{name}_read_en({name}_read_en),
  .{name}_write_done({name}_write_done),
  .{name}_read_done({name}_read_done)'''
     out_str += "\n);"
     return out_str

def generate_readmemh(memory_cell_dicts):
     out_str = ""
     for memory_cell_dict in memory_cell_dicts:
          name = memory_cell_dict["cell-name"]
          if "d2" in memory_cell_dict["module-name"]:
               mem = "mem.mem"
          else:
               mem = "mem"
          out_str += f"  $readmemh({{DATA, \"/{name}.dat\"}}, {name}.{mem});\n"
     return out_str

def generate_writememh(memory_cell_dicts):
     out_str = ""
     if len(memory_cell_dicts) > 0:
          out_str += "final begin\n"
          for memory_cell_dict in memory_cell_dicts:
               name = memory_cell_dict["cell-name"]
               if "d2" in memory_cell_dict["module-name"]:
                    mem = "mem.mem"
               else:
                    mem = "mem"
               out_str += f"    $writememh({{DATA, \"/{name}.out\"}}, {name}.{mem});\n"
          out_str += "end"
     return out_str

def create_memory_cell_dict(line, module_name):
     memory_cell_dict = {}
     # parse line that contains a ref cell
     memory_cell_dict["cell-name"] = line.lstrip().split()[1]
     memory_cell_dict["module-name"] = module_name
     args = line.split(f"{module_name}(")[1].split(")")[0].split(",")
     if "d1" in module_name:
          memory_cell_dict["WIDTH"] = int(args[0])
          memory_cell_dict["SIZE"] = int(args[1])
          memory_cell_dict["IDX_SIZE"] = int(args[2])
     elif "d2" in module_name:
          memory_cell_dict["WIDTH"] = int(args[0])
          memory_cell_dict["D0_SIZE"] = int(args[1])
          memory_cell_dict["D1_SIZE"] = int(args[2])
          memory_cell_dict["D0_IDX_SIZE"] = int(args[3])
          memory_cell_dict["D1_IDX_SIZE"] = int(args[4])
     return memory_cell_dict

def get_memory_cells(calyx_program):
     memory_cell_dicts = []
     with open(calyx_program) as f:
          for line in f:
               if not("ref") in line:
                    continue
               elif "comb_mem_d1" in line:
                    memory_cell_dict = create_memory_cell_dict(line, "comb_mem_d1")
                    memory_cell_dicts.append(memory_cell_dict)
               elif "seq_mem_d1" in line:
                    memory_cell_dict = create_memory_cell_dict(line, "seq_mem_d1")
                    memory_cell_dicts.append(memory_cell_dict)
               elif "seq_mem_d2" in line:
                    memory_cell_dict = create_memory_cell_dict(line, "seq_mem_d2")
                    memory_cell_dicts.append(memory_cell_dict)
               elif "comb_mem_d" in line or "seq_mem_d" in line:
                    print(f"Multidimensional memory yet to be supported found: {line}")
                    print("Aborting.")
                    sys.exit(1)

     return memory_cell_dicts


def generate(calyx_program):
     memory_cell_dicts = get_memory_cells(calyx_program)
     with open(template_file) as t:
          for line in t:
               if line.strip() == "MEMORY_FIELDS":
                    print(generate_fields(memory_cell_dicts))
               elif line.strip() == "MEMORY_DECLS":
                    print(generate_memory_dec(memory_cell_dicts))
               elif line.strip() == "MAIN_DECL":
                    print(generate_main_decl(memory_cell_dicts))
               elif line.strip() == "READMEMH_STATEMENTS":
                    print(generate_readmemh(memory_cell_dicts))
               elif line.strip() == "WRITEMEMH_STATEMENTS":
                    print(generate_writememh(memory_cell_dicts))
               else:
                    print(line.rstrip())

def main():
    if len(sys.argv) != 2:
        args_desc = [                                                                                                                                                                   
            "CALYX_PROGRAM"
        ]
        print(f"Usage: {sys.argv[0]} {' '.join(args_desc)}")                                                                                                                           
        return 1
    generate(sys.argv[1])


if __name__ == '__main__':
        main()
