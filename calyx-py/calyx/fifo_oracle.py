import queues


if __name__ == "__main__":
    commands, values = queues.parse_json()
    pifo = queues.Fifo([])
    ans = queues.operate_queue(commands, values, pifo)
    queues.dump_json(commands, values, ans)
