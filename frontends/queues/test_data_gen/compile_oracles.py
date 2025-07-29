import sys
import queues
import util
import json
import os


def create(data, lower, upper, length):
    """
    Assumes boundary is calculated by evenly dividing the flows. This is not the
    case for the 6 and 7 flows in the existing oracles, which were different in
    order to demonstrate the customizable capability of our queues.
    """
    for key, val in data.items():
        if key == "FIFO":
            return queues.Fifo(length)
        elif key == "RR" or key == "Strict":
            num_children = len(val)
            if num_children == 1:
                return create(val[0], lower, upper, length)
            else:
                interval = (upper - lower) // num_children
                boundaries = []
                for i in range(1, num_children):
                    boundaries.append(lower + (interval * i))
                boundaries.append(upper)

                children = []
                lo = lower
                u = upper
                for i in range(num_children):
                    u = lo + interval
                    if i == num_children - 1:
                        u = upper
                    children.append(create(val[i], lo, u, length))
                    lo = u
                if key == "RR":
                    return queues.RRPifo(num_children, boundaries, children, length)
                elif key == "Strict":
                    order = [n for n in range(num_children)]
                    return queues.StrictPifo(
                        num_children, boundaries, order, children, length
                    )


if __name__ == "__main__":
    num_cmds, length = int(sys.argv[1]), int(sys.argv[2])
    json_file = sys.argv[3]
    keepgoing = "--keepgoing" in sys.argv
    order_override = "--order" in sys.argv
    commands, values, _, _ = util.parse_json()

    base_dir = os.path.dirname(__file__)
    json_subdir = "../tests/compiler/jsons"
    file_path = os.path.join(base_dir, json_subdir, json_file)
    with open(file_path) as f:
        data = json.load(f)

    pifo = create(data, 0, 400, length)

    ans = queues.operate_queue(pifo, num_cmds, commands, values, keepgoing=keepgoing)
    util.dump_json(commands, values, ans)
