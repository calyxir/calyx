import os
import numpy as np
import matplotlib.pyplot as plt

def append_path_prefix(file):
    path_to_script = os.path.dirname(__file__)
    path_to_file = os.path.join(path_to_script, file)
    return path_to_file

cycle_stats = append_path_prefix("cycles.txt")

# Parse data for round_robin and strict queues
binheap_rr_cycle_counts = []
specialized_rr_cycle_counts = []
binheap_strict_cycle_counts = []
specialized_strict_cycle_counts = []
with open(cycle_stats) as cycle_stats:
    for line in cycle_stats:
        if "round_robin" in line:
            split = line.strip().split(":")
            tup = (split[0].split("flow")[0][-1], int(split[1]))
            if "binheap" in line:
                binheap_rr_cycle_counts.append(tup)
            else:
                specialized_rr_cycle_counts.append(tup)
        if "strict" in line:
            split = line.strip().split(":")
            tup = (split[0].split("flow")[0][-1], int(split[1]))
            if "binheap" in line:
                binheap_strict_cycle_counts.append(tup)
            else:
                specialized_strict_cycle_counts.append(tup)

# Draw results
fig, ax = plt.subplots(1, 1)
fig.set_size_inches(20, 10, forward=True)
ax.set_title("Cycle Counts for Round Robin Queues", 
             fontweight='bold',
             fontsize=20)
ax.set_xlabel("number of flows",
              fontsize=20)
ax.set_ylabel("cycles",
              fontsize=20)
specialized = ax.scatter(
        list(map(lambda x: x[0], specialized_rr_cycle_counts)),
        list(map(lambda x: x[1], specialized_rr_cycle_counts)),
        c='b')
binheap = ax.scatter(
        list(map(lambda x: x[0], binheap_rr_cycle_counts)),
        list(map(lambda x: x[1], binheap_rr_cycle_counts)),
        c='g')
plt.legend((specialized, binheap),
           ("Specialized (i.e. Cassandra style PIFO)", "Binary Heap"),
           fontsize=12)
file = append_path_prefix("round_robin.png")
plt.savefig(file)

fig, ax = plt.subplots(1, 1)
fig.set_size_inches(20, 10, forward=True)
ax.set_title("Cycle Counts for Strict Queues", 
             fontweight='bold',
             fontsize=18)
ax.set_xlabel("number of flows",
              fontsize=16)
ax.set_ylabel("cycles",
              fontsize=16)
specialized = ax.scatter(
        list(map(lambda x: x[0], specialized_strict_cycle_counts)),
        list(map(lambda x: x[1], specialized_strict_cycle_counts)),
        c='b')
binheap = ax.scatter(
        list(map(lambda x: x[0], binheap_strict_cycle_counts)),
        list(map(lambda x: x[1], binheap_strict_cycle_counts)),
        c='g')
plt.legend((specialized, binheap),
           ("Specialized (i.e. Cassandra style PIFO)", "Binary Heap"),
           fontsize=12)
file = append_path_prefix("strict.png")
plt.savefig(file)

