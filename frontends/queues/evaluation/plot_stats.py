import os
import sys
import json
from enum import Enum
import matplotlib.pyplot as plt


class Logic(Enum):
    RR = 1
    STRICT = 2


def append_path_prefix(file):
    path_to_script = os.path.dirname(__file__)
    path_to_file = os.path.join(path_to_script, file)
    return path_to_file


def parse(stat, file):
    out = {
        "binheap": {"round_robin": {}, "strict": {}},
        "specialized": {"round_robin": {}, "strict": {}},
    }

    with open(file) as file:
        data = json.load(file)
        for file, data in data.items():
            if isinstance(data, dict):
                data = data[stat]

            flow_no = file.split("flow")[0][-1]

            if "round_robin" in file:
                if "binheap" in file:
                    out["binheap"]["round_robin"][flow_no] = data
                else:
                    out["specialized"]["round_robin"][flow_no] = data
            if "strict" in file:
                if "binheap" in file:
                    out["binheap"]["strict"][flow_no] = data
                else:
                    out["specialized"]["strict"][flow_no] = data

    return out


def draw(data, stat, logic, unit):
    fig, ax = plt.subplots(1, 1)
    fig.set_size_inches(20, 10, forward=True)
    ax.set_xlabel("number of flows", fontsize=20)
    if unit is None:
        ax.set_ylabel(stat, fontsize=20)
    else:
        ax.set_ylabel(f"{stat} ({unit})", fontsize=20)

    if logic == Logic.RR:
        specialized = ax.scatter(
            data["specialized"]["round_robin"].keys(),
            data["specialized"]["round_robin"].values(),
            c="b",
        )
        binheap = ax.scatter(
            data["binheap"]["round_robin"].keys(),
            data["binheap"]["round_robin"].values(),
            c="g",
        )

        ax.set_title("Round Robin Queues", fontweight="bold", fontsize=20)
        file = append_path_prefix(f"{stat}_round_robin")

    elif logic == Logic.STRICT:
        specialized = ax.scatter(
            data["specialized"]["strict"].keys(),
            data["specialized"]["strict"].values(),
            c="b",
        )
        binheap = ax.scatter(
            data["binheap"]["strict"].keys(), data["binheap"]["strict"].values(), c="g"
        )

        ax.set_title("Strict Queues", fontweight="bold", fontsize=20)
        file = append_path_prefix(f"{stat}_strict")

    plt.legend((specialized, binheap), ("Specialized", "Binary Heap"), fontsize=12)

    plt.savefig(file)

    print(f"Generated {file}.png")


if __name__ == "__main__":
    # Parse data for round_robin and strict queues
    stat = sys.argv[1]
    data = {}
    if stat == "total_time":
        file1 = sys.argv[2]
        file2 = sys.argv[3]

        cycle_data = parse("cycles", file1)
        slack_data = parse("worst_slack", file2)

        data = cycle_data.copy()
        for impl in data.keys():
            for logic in data[impl].keys():
                for flow_no in data[impl][logic].keys():
                    cycles = cycle_data[impl][logic][flow_no]
                    slack = slack_data[impl][logic][flow_no]
                    data[impl][logic][flow_no] = (1000 * cycles) / (7 - slack)
    else:
        file = sys.argv[2]
        data = parse(stat, file)

    # Draw results
    unit = "μs" if stat == "total_time" else None
    draw(data, stat, Logic.RR, unit)
    draw(data, stat, Logic.STRICT, unit)
