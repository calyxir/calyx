import sys
import calyx.queues as queues
from calyx import queue_util

if __name__ == "__main__":
    num_cmds, len, numflows = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
    keepgoing = "--keepgoing" in sys.argv
    commands, values, _, _ = queue_util.parse_json()

    if numflows == 2:
        boundaries = [200, 400]
        order = [1, 0]
    elif numflows == 3:
        boundaries = [133, 266, 400]
        order = [1, 2, 0]
    elif numflows == 4:
        boundaries = [100, 200, 300, 400]
        order = [3, 0, 2, 1]
    elif numflows == 5:
        boundaries = [80, 160, 240, 320, 400]
        order = [0, 1, 2, 3, 4]
    elif numflows == 6:
        boundaries = [66, 100, 200, 220, 300, 400]
        order = [3, 1, 5, 2, 4, 0]
    else:
        raise ValueError("Unsupported number of flows")

    subqueues = [queues.Fifo(len) for _ in range(numflows)]

    # Our Strict queue orchestrates n subqueues. It takes in a list of
    # boundaries of length n, as well as a list `order` which specifies the ranked
    # order of the flows.
    pifo = queues.StrictPifo(numflows, boundaries, order, subqueues, len)

    ans = queues.operate_queue(pifo, num_cmds, commands, values, keepgoing=keepgoing)

    queue_util.dump_json(commands, values, ans)
