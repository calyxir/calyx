import json
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

def create_memory_cell_dict(memory):
     memory_cell_dict = {}
     memory_cell_dict["cell-name"] = memory["name"]
     dimensions = memory["dimensions"]
     if memory["memory_type"] == "Sequential":
          memory_cell_dict["module-name"] = f"seq_mem_d{dimensions}"
     else:
          memory_cell_dict["module-name"] = f"comb_mem_d{dimensions}"

     memory_cell_dict["WIDTH"] = memory["data_width"]
     if dimensions == 1:
          memory_cell_dict["SIZE"] = memory["dimension_sizes"][0]
          memory_cell_dict["IDX_SIZE"] = memory["idx_sizes"][0]
     elif dimensions > 2:
          print(f"Multidimensional memory yet to be supported found: {memory['name']}")
          print("Aborting.")
          sys.exit(1)
     else:
          for i in range(dimensions):
               memory_cell_dict[f"D{i}_SIZE"] = memory["dimension_sizes"][i]
               memory_cell_dict[f"D{i}_IDX_SIZE"] = memory["idx_sizes"][i]

     return memory_cell_dict

def get_memory_cells(yxi_json_filepath):
     memory_cell_dicts = []
     yxi_json = json.load(open(yxi_json_filepath))
     for memory in yxi_json["memories"]:
          memory_cell_dict = create_memory_cell_dict(memory)
          memory_cell_dicts.append(memory_cell_dict)

     return memory_cell_dicts


def generate(yxi_json):
     memory_cell_dicts = get_memory_cells(yxi_json)
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
            "YXI_JSON"
        ]
        print(f"Usage: {sys.argv[0]} {' '.join(args_desc)}")                                                                                                                
        return 1
    generate(sys.argv[1])


if __name__ == '__main__':
        main()
