import queues


if __name__ == "__main__":
    commands, values = queues.parse_json()

    pifo = queues.Pifo(queues.Fifo([]), queues.Fifo([]), 200)
    # Our PIFO is simple: it just orchestrates two FIFOs.

    ans = queues.operate_queue(commands, values, pifo)
    queues.dump_json(commands, values, ans)
