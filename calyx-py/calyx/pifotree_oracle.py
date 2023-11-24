# Usage:
# To make a .data file:
# python calyx-py/calyx/queue_data_gen.py --piezo > calyx-py/test/correctness/sdn.data
# To then make a .expect file:
# cat calyx-py/test/correctness/sdn.data |
# python calyx-py/calyx/pifotree_oracle.py > calyx-py/test/correctness/sdn.expect

import calyx.queues as queues
from calyx import queue_util


if __name__ == "__main__":
    commands, values = queue_util.parse_json()

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
        queues.Pifo(queues.Fifo([]), queues.Fifo([]), 100), queues.Fifo([]), 200, False
    )

    ans = queues.operate_queue(commands, values, pifo)
    queue_util.dump_json(commands, values, ans)
