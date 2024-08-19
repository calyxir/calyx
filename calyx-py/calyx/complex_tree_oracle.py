# For usage, see gen_queue_data_expect.sh

import sys
import calyx.queues as queues
from calyx import queue_util


if __name__ == "__main__":
    max_cmds, len = int(sys.argv[1]), int(sys.argv[2])
    keepgoing = "--keepgoing" in sys.argv
    commands, values, _, _ = queue_util.parse_json()

    # Our complex PIFO is a tree of queues. It has the shape
    # rr(strict(A, B, C), rr(D, E, F), strict(G, H)).

    subqueues3 = [queues.Fifo(len) for _ in range(3)]
    # a second subqueue copy is required, we cannot pass the same subqueue across more than one function call
    subqueues3s = [queues.Fifo(len) for _ in range(3)] 
    subqueues2 = [queues.Fifo(len) for _ in range(2)]

    pifo = queues.RRQueue(
        3,
        [133, 266, 400],
        [
            queues.StrictPifo(3, [44, 88, 133], [0, 1, 2], subqueues3s, len),
            queues.RRQueue(3, [177, 221, 266], subqueues3, len),
            queues.StrictPifo(2, [333, 400], [0, 1], subqueues2, len),
        ],
        len,
    )

    ans = queues.operate_queue(pifo, max_cmds, commands, values, keepgoing=keepgoing)
    queue_util.dump_json(commands, values, ans)
