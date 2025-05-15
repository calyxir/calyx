# For usage, see gen_test_data.sh

import sys
import queues
import util


if __name__ == "__main__":
    num_cmds, len, numflows = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
    keepgoing = "--keepgoing" in sys.argv
    order_override = "--order" in sys.argv
    commands, values, _, _ = util.parse_json()

    if numflows == 2:
        boundaries = [200, 400]
        order = [0, 1]
    elif numflows == 3:
        boundaries = [133, 266, 400]
        order = [0, 1, 2]
    elif numflows == 4:
        boundaries = [100, 200, 300, 400]
        order = [0, 1, 2, 3]
    elif numflows == 5:
        boundaries = [80, 160, 240, 320, 400]
        order = [0, 1, 2, 3, 4]
    elif numflows == 6:
        boundaries = [66, 100, 200, 220, 300, 400]
        order = [0, 1, 2, 3, 4, 5]
    elif numflows == 7:
        boundaries = [50, 100, 150, 200, 250, 300, 400]
        order = [0, 1, 2, 3, 4, 5, 6]
    else:
        raise ValueError("Unsupported number of flows")

    if order_override:
        # --order expects an argument where the ints are comma-separated,
        # for example "2,3,1,0" for [2, 3, 1, 0]
        order_idx = sys.argv.index("--order") + 1
        lst = sys.argv[order_idx]
        order = list(map(int, lst.split(",")))

    subqueues = [queues.Fifo(len) for _ in range(numflows)]

    # Our Strict queue orchestrates n subqueues. It takes in a list of
    # boundaries of length n, as well as a list `order` which specifies the ranked
    # order of the flows.
    pifo = queues.StrictPifo(numflows, boundaries, order, subqueues, len)

    ans = queues.operate_queue(pifo, num_cmds, commands, values, keepgoing=keepgoing)

    util.dump_json(commands, values, ans)
