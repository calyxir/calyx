# For usage, see gen_queue_data_expect.sh

import sys
import queues
import util


if __name__ == "__main__":
    max_cmds, len = int(sys.argv[1]), int(sys.argv[2])
    keepgoing = "--keepgoing" in sys.argv
    commands, values, _, _ = util.parse_json()

    # Our PIFO is a little complicated: it is a tree of queues.
    # The root has two children, which are PIFOs.
    #   - PIFO_red is the left child.
    #     + PIFO_red itself has two children, which are FIFOs.
    #       * FIFO_purple is the left child.
    #       * FIFO_tangerine is the right child.
    #       * The boundary for this is 100.
    #   - FIFO_blue is the right child.
    #   - The boundary for this is 200.

    pifo = queues.RRPifo(
        2,
        [200, 400],
        [
            queues.RRPifo(2, [100, 200], [queues.Fifo(len), queues.Fifo(len)], len),
            queues.Fifo(len),
        ],
        len,
    )

    ans = queues.operate_queue(pifo, max_cmds, commands, values, keepgoing=keepgoing)
    util.dump_json(commands, values, ans)
