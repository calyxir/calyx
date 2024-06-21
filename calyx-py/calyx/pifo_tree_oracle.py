# For usage, see gen_queue_data_expect.sh

import sys
import calyx.queues as queues
from calyx import queue_util


if __name__ == "__main__":
    max_cmds, len = int(sys.argv[1]), int(sys.argv[2])
    keepgoing = "--keepgoing" in sys.argv
    commands, values, _ = queue_util.parse_json()

    # Our PIFO is a little complicated: it is a tree of queues.
    # The root has two children, which are PIFOs.
    #   - PIFO_red is the left child.
    #     + PIFO_red itself has two children, which are FIFOs.
    #       * FIFO_purple is the left child.
    #       * FIFO_tangerine is the right child.
    #       * The boundary for this is 100.
    #   - FIFO_blue is the right child.
    #   - The boundary for this is 200.

    pifo = queues.Pifo(
        queues.Pifo(queues.Fifo(len), queues.Fifo(len), 100, len),
        queues.Fifo(len),
        200,
        len,
    )

    ans = queues.operate_queue(pifo, max_cmds, commands, values, keepgoing=keepgoing)
    queue_util.dump_json(ans, commands, values)
