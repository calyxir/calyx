import sys
import json

def get_memory_cells(calyx_program):
     memory_cell_dict = {}
     with open(calyx_program) as f:
          for line in f:
               if "ref" in line and "comb_mem_d1" in line:
                    # parse line that contains a ref cell
                    memory_cell_dict["cell-name"] = line.lstrip().split()[1]
                    # we're assuming that comb_mem_d1 for now
                    args = line.split("(")[1].split(")")[0].split(",")
                    memory_cell_dict["WIDTH"] = args[0]
                    memory_cell_dict["SIZE"] = args[1]
                    memory_cell_dict["IDX_SIZE"] = args[2]

               elif "comb_mem_d" in line:
                    print(f"Multidimensional memory found: {line}")
                    print("Aborting.")
                    sys.exit(1)

     return memory_cell_dict


def generate(calyx_program):
     print(get_memory_cells(calyx_program))

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
