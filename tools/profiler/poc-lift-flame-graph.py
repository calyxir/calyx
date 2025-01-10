# a super simple script to show ADL level profiling proof of concept/experimentation
import sys

class Component:
    def __init__(self, name, pos_id):
        self.name = name
        self.position_id = pos_id
        self.position = None
        self.cells = {} # cell_name --> pos_id
        self.groups = {} # group_name --> pos_id

    def rewrite(self, position_map):
        # replace position IDs with the filename, linenum pair.
        self.position = position_map[self.position_id]
        for cell in self.cells:
            pos_id = self.cells[cell]
            self.cells[cell] = position_map[pos_id]
        for group in self.groups:
            pos_id = self.groups[group]
            self.groups[group] = position_map[pos_id]

    def __str__(self):
        s = f"Component {self.name}:\n"
        s += f"\tpos: {self.position_id}\n"
        s += f"\tcells:\n"
        for cell in self.cells:
            s += f"\t\t{cell}: {self.cells[cell]}\n"
        s += f"\tgroups:\n"
        for group in self.groups:
            s += f"\t\t{group}: {self.groups[group]}\n"
        return s

def parse(calyx_file):
    # a really hacky parser.
    metadata = False
    file_map = {}
    position_map = {}
    components = {} # name --> Component
    curr_component = None
    with open(calyx_file, "r") as r:
        for line in r:
            line_strip = line.strip()
            # start metadata
            if line_strip.startswith("metadata #{"):
                metadata = True
                continue
            if metadata:
                if line_strip.startswith("file-"):
                    line_split = line_strip.split("file-")[1].split(":")
                    file_id = line_split[0]
                    filename = line_split[1].strip()
                    file_map[file_id] = filename
                if line_strip.startswith("pos-"):
                    line_split = line_strip.split("pos-")[1].split(":")
                    position_id = line_split[0]
                    rest_split = line_split[1].strip().strip("(").strip(")").replace(" ", "").split(",")
                    file_id = rest_split[0]
                    line_num = rest_split[1]
                    position_map[position_id] = (file_map[file_id], line_num) # if the query fails sth went wrong
            else:
                # shoddy attempt at parsing an eDSL-generated Calyx file
                if line_strip.startswith("component"):
                    if curr_component is not None: # store the previous component
                        components[curr_component.name] = curr_component
                    name = line_strip.split(" ")[1].split("<")[0]
                    pos_id = line_strip.split('"pos"=')[1].split(">")[0]
                    curr_component = Component(name, pos_id)
                elif line_strip.startswith("group "):
                    groupname = line_strip.split("<")[0].split(" ")[-1]
                    pos_id = line_strip.split('"pos"=')[1].split(">")[0]
                    if groupname in curr_component.groups:
                        raise RuntimeError(f"Group {groupname} already recorded in component {curr_component.name}")
                    curr_component.groups[groupname] = pos_id
                elif "@pos" in line_strip: # currently the other thing that has positions is cells. probably will change in the future
                    cellname = line_strip.split("=")[0].strip().split(" ")[-1]
                    pos_id = line_strip.split("@pos(")[1].split(")")[0]
                    if cellname in curr_component.cells:
                        raise RuntimeError(f"Cell {cellname} already recorded in component {curr_component.name}")
                    curr_component.cells[cellname] = pos_id

    return components, position_map

def main(calyx_file):
    components, position_map = parse(calyx_file)
    for component_name in components:
        component = components[component_name]
        component.rewrite(position_map)
        print(component)
        print()

if __name__ == "__main__":
    if len(sys.argv) > 1:
        calyx_file = sys.argv[1]
        main(calyx_file)
    else:
        args_desc = [
            "CALYX_FILE"
        ]
        print(f"Usage: {sys.argv[0]} {' '.join(args_desc)}")
        sys.exit(-1)
