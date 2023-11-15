import calyx.queues as queues
import calyx.queue_util as queue_util


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
        queues.Pifo(
            queues.Fifo([], queue_util.QUEUE_SIZE),
            queues.Fifo([], queue_util.QUEUE_SIZE),
            100,
            queue_util.QUEUE_SIZE,
        ),
        queues.Fifo([], queue_util.QUEUE_SIZE),
        200,
        queue_util.QUEUE_SIZE,
    )

    ans = queues.operate_queue(commands, values, pifo)
    queue_util.dump_json(commands, values, ans)
