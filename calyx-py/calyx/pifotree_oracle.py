import queues


if __name__ == "__main__":
    commands, values = queues.parse_json()

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
        queues.Pifo(queues.Fifo([]), queues.Fifo([]), 100), queues.Fifo([]), 200
    )

    ans = queues.operate_queue(commands, values, pifo)
    queues.dump_json(commands, values, ans)
