import os
import sys
import json
import numpy as np
import matplotlib.pyplot as plt

stat = sys.argv[1]
path = sys.argv[2]

def append_path_prefix(file):
    path_to_script = os.path.dirname(__file__)
    path_to_file = os.path.join(path_to_script, file)
    return path_to_file

def draw(specialized, binheap, name, filename):
    fig, ax = plt.subplots(1, 1)
    fig.set_size_inches(20, 10, forward=True)
    ax.set_title(name, 
                 fontweight='bold',
                 fontsize=20)
    ax.set_xlabel("number of flows",
                  fontsize=20)
    ax.set_ylabel(stat,
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

# Parse data for round_robin and strict queues
binheap_rr_counts = []
specialized_rr_counts = []
binheap_strict_counts = []
specialized_strict_counts = []
if stat == "cycles":
    with open(path) as file:
        for line in file:
            split = line.strip().split(":")
            tup = (split[0].split("flow")[0][-1], int(split[1]))
            if "round_robin" in line:
                if "binheap" in line:
                    binheap_rr_counts.append(tup)
                else:
                    specialized_rr_counts.append(tup)
            if "strict" in line:
                if "binheap" in line:
                    binheap_strict_counts.append(tup)
                else:
                    specialized_strict_counts.append(tup)
else:
    with open(path) as file:
        data = file.read().strip()
        for d in data.split("\n\n"):
            split = d.split("\n")
            name = split[0]
            stats = json.loads("\n".join(split[1:]))
            tup = (name.split("flow")[0][-1], stats[stat])
            if "round_robin" in name:
                if "binheap" in name:
                    binheap_rr_counts.append(tup)
                else:
                    specialized_rr_counts.append(tup)
            if "strict" in name:
                if "binheap" in name:
                    binheap_strict_counts.append(tup)
                else:
                    specialized_strict_counts.append(tup)

# Draw results
draw(specialized_rr_counts, 
     binheap_rr_counts, 
     "Round Robin Queues", 
     "round_robin")

draw(specialized_strict_counts, 
     binheap_strict_counts, 
     "Strict Queues", 
     "strict")
