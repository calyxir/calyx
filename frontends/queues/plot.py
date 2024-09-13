import os
import sys
import json
import numpy as np
import matplotlib.pyplot as plt

stat = sys.argv[1]

def append_path_prefix(file):
    path_to_script = os.path.dirname(__file__)
    path_to_file = os.path.join(path_to_script, file)
    return path_to_file

def parse(stat, file):
    binheap_rr = []
    specialized_rr = []
    binheap_strict = []
    specialized_strict = []

    with open(file) as file:
        if stat == "cycles":
            for line in file:
                split = line.strip().split(":")
                tup = (split[0].split("flow")[0][-1], int(split[1]))
                if "round_robin" in line:
                    if "binheap" in line:
                        binheap_rr.append(tup)
                    else:
                        specialized_rr.append(tup)
                if "strict" in line:
                    if "binheap" in line:
                        binheap_strict.append(tup)
                    else:
                        specialized_strict.append(tup)
        else:
            data = file.read().strip()
            for d in data.split("\n\n"):
                split = d.split("\n")
                name = split[0]
                stats = json.loads("\n".join(split[1:]))
                tup = (name.split("flow")[0][-1], stats[stat])
                if "round_robin" in name:
                    if "binheap" in name:
                        binheap_rr.append(tup)
                    else:
                        specialized_rr.append(tup)
                if "strict" in name:
                    if "binheap" in name:
                        binheap_strict.append(tup)
                    else:
                        specialized_strict.append(tup)

    return (specialized_rr, 
            binheap_rr, 
            specialized_strict, 
            binheap_strict)

def draw(specialized, binheap, name, filename, details=None):
    fig, ax = plt.subplots(1, 1)
    fig.set_size_inches(20, 10, forward=True)
    ax.set_title(name, 
                 fontweight='bold',
                 fontsize=20)
    ax.set_xlabel("number of flows",
                  fontsize=20)
    if details is None:
        ax.set_ylabel(stat,
                      fontsize=20)
    else:
        ax.set_ylabel(f"{stat} ({details})",
                      fontsize=20)
    specialized = ax.scatter(
            list(map(lambda x: x[0], specialized)),
            list(map(lambda x: x[1], specialized)),
            c='b')
    binheap = ax.scatter(
            list(map(lambda x: x[0], binheap)),
            list(map(lambda x: x[1], binheap)),
            c='g')
    plt.legend((specialized, binheap),
               ("Specialized (i.e. Cassandra style PIFO)", "Binary Heap"),
               fontsize=12)
    file = append_path_prefix(f"{stat}_{filename}")
    plt.savefig(file)
    print(f"Generated {file}.png")

# Parse data for round_robin and strict queues
(specialized_rr, binheap_rr, specialized_strict, binheap_strict) = ([], [], [], [])
if stat == "total_time":
    file1 = sys.argv[2]
    file2 = sys.argv[3]

    (specialized_cycles_rr, 
     binheap_cycles_rr, 
     specialized_cycles_strict, 
     binheap_cycles_strict) = parse("cycles", file1)
    (specialized_slacks_rr, 
     binheap_slacks_rr, 
     specialized_slacks_strict, 
     binheap_slacks_strict) = parse("worst_slack", file2)

    def map2(cycles, slacks):
        cycles.sort(key=lambda c: c[0])
        slacks.sort(key=lambda s: s[0])

        def f(c,s):
            return (c[0], (1000*c[1])/(7 - s[1]))

        return list(map(f, cycles, slacks))

    specialized_rr = map2(specialized_cycles_rr, specialized_slacks_rr)
    binheap_rr = map2(binheap_cycles_rr, binheap_slacks_rr)
    specialized_strict = map2(specialized_cycles_strict, specialized_slacks_strict)
    binheap_strict = map2(binheap_cycles_strict, binheap_slacks_strict)
else:
    file = sys.argv[2]
    (specialized_rr, binheap_rr, specialized_strict, binheap_strict) = parse(stat, file)

# Draw results
details = "μs" if stat == "total_time" else None
draw(specialized_rr, binheap_rr, "Round Robin Queues", "round_robin", details)
draw(specialized_strict, binheap_strict, "Strict Queues", "strict", details)
