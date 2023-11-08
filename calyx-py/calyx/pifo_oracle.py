import queues


if __name__ == "__main__":
    commands, values = queues.parse_json()

    # Our PIFO is simple: it just orchestrates two FIFOs. The boundary is 200.
    pifo = queues.Pifo(queues.Fifo([]), queues.Fifo([]), 200)

    ans = queues.operate_queue(commands, values, pifo)
    queues.dump_json(commands, values, ans)
