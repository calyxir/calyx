import os
import sys

template_file = os.path.join(sys.path[0], "custom_tb_template.sv")

def generate_fields(memory_cell_dicts):
    out_str = ""
    for memory_cell_dict in memory_cell_dicts:
         name = memory_cell_dict['cell-name']
         width = memory_cell_dict['WIDTH']
         idx_size = memory_cell_dict['IDX_SIZE']
         if (idx_size > 1):
              out_str += f"wire [{idx_size-1}:0] {name}_addr0;\n"
         else:
              out_str += f"wire {name}_addr0;\n"
         out_str += f"wire [{width-1}:0] {name}_write_data;\n"
         out_str += f"wire {name}_write_en;\n"
         out_str += f"wire [{width-1}:0] {name}_read_data;\n"
         out_str += f"wire {name}_done;\n\n"

    return out_str

def generate_memory_dec(memory_cell_dicts):
     out_str = ""
     for memory_cell_dict in memory_cell_dicts:
          name = memory_cell_dict['cell-name']
          out_str += f'''comb_mem_d1 # (
    .IDX_SIZE({memory_cell_dict["IDX_SIZE"]}),
    .SIZE({memory_cell_dict["SIZE"]}),
    .WIDTH({memory_cell_dict["WIDTH"]})
) {name} (
    .addr0({name}_addr0),
    .clk(clk),
    .done({name}_done),
    .read_data({name}_read_data),
    .reset(reset),
    .write_data({name}_write_data),
    .write_en({name}_write_en)
);
'''
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
          out_str += ",\n"
          out_str += f'''  .{name}_addr0({name}_addr0),
  .{name}_write_data({name}_write_data),
  .{name}_write_en({name}_write_en),
  .{name}_read_data({name}_read_data),
  .{name}_done({name}_done)
'''
     out_str += "\n);"
     return out_str

def generate_readmemh(memory_cell_dicts):
     out_str = ""
     for memory_cell_dict in memory_cell_dicts:
          name = memory_cell_dict["cell-name"]
          out_str += f"  $readmemh({{DATA, \"/{name}.dat\"}}, {name}.mem);\n"
     return out_str

def generate_writememh(memory_cell_dicts):
     out_str = ""
     if len(memory_cell_dicts) > 0:
          out_str += "final begin\n"
          for memory_cell_dict in memory_cell_dicts:
               name = memory_cell_dict["cell-name"]
               out_str += f"    $writememh({{DATA, \"/{name}.out\"}}, {name}.mem);\n"
          out_str += "end"
     return out_str
         
def get_memory_cells(calyx_program):
     memory_cell_dicts = []
     with open(calyx_program) as f:
          for line in f:
               if "comb_mem_d1" in line:
                    memory_cell_dict = {}
                    # parse line that contains a ref cell
                    memory_cell_dict["cell-name"] = line.lstrip().split()[1]
                    # we're assuming that comb_mem_d1 for now
                    args = line.split("comb_mem_d1(")[1].split(")")[0].split(",")
                    memory_cell_dict["WIDTH"] = int(args[0])
                    memory_cell_dict["SIZE"] = int(args[1])
                    memory_cell_dict["IDX_SIZE"] = int(args[2])
                    memory_cell_dicts.append(memory_cell_dict)

               elif "comb_mem_d" in line:
                    print(f"Multidimensional memory found: {line}")
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
