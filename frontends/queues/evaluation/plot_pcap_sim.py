import os
import sys
import json
import matplotlib.pyplot as plt
from matplotlib.patches import Patch

CLOCK_PERIOD = 7  # in ns
NUM_FLOWS = 2


class OutOfColors(Exception):
    pass


class Packet:
    def __init__(self, id, flow, punch_in, punch_out=None):
        self.id = id
        self.flow = flow
        self.punch_in = punch_in
        self.punch_out = punch_out

    def color(self):
        colors = [
            "red",
            "skyblue",
            "forestgreen",
            "lightsalmon",
            "dodgerblue",
            "darkseagreen",
            "orchid",
        ]
        if self.flow < len(colors):
            return colors[self.flow]

        raise OutOfColors(f"No color for flow {self.flow}; Extend the colors list!")

    def __str__(self):
        return f"({self.id}, {self.flow}, {self.punch_in}, {self.punch_out})"


def append_path_prefix(file):
    path_to_script = os.path.dirname(__file__)
    path_to_file = os.path.join(path_to_script, file)
    return path_to_file


def parse(file):
    data = json.load(file)
    packets = []

    for i, cmd in enumerate(data["commands"]):
        if cmd == 0:
            continue

        id = data["values"][i]
        flow = data["flows"][i]
        punch_in = data["arrival_cycles"][i] * CLOCK_PERIOD
        if id in data["ans_mem"]:
            j = data["ans_mem"].index(id)
            punch_out = data["departure_cycles"][j] * CLOCK_PERIOD
            pkt = Packet(id, flow, punch_in, punch_out)
        else:
            pkt = Packet(id, flow, punch_in)
        packets += [pkt]

    packets.sort(
        key=lambda p: float("inf") if p.punch_out is None else float(p.punch_out)
    )

    return packets


def draw(packets, name):
    fig, ax = plt.subplots(1, 1)
    fig.set_size_inches(20, 10, forward=True)
    ax.set_ylim(0, len(packets))
    ax.axes.yaxis.set_visible(False)

    patches = []
    labels = []
    for i, pkt in enumerate(packets):
        color = pkt.color()
        if pkt.punch_out is not None:
            treetime = pkt.punch_out - pkt.punch_in
            _handle = ax.broken_barh(
                [(pkt.punch_in, treetime)], (i, 1), facecolors=color
            )

            label = f"Flow {pkt.flow}"
            if label not in labels:
                patches += [Patch(color=color)]
                labels += [label]
        else:
            treetime = 0
            ax.broken_barh([(pkt.punch_in, treetime)], (i, 1), facecolors=color)
            ax.text(
                x=pkt.punch_in + 0.2,
                y=i + 0.7,
                s="OVERFLOW",
                color="black",
                fontsize="x-small",
            )
    ax.invert_yaxis()
    ax.legend(handles=patches, labels=labels)

    file = append_path_prefix(name)
    plt.savefig(file)
    print(f"Generated {file}")


if __name__ == "__main__":
    file = sys.argv[1]
    basename = os.path.basename(file)
    with open(file) as file:
        packets = parse(file)
        name = basename.split(".")[0] + ".png"
        draw(packets, name)
