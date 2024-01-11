import calyx.queues as queues
from calyx import queue_util

if __name__ == "__main__":
    commands, values = queue_util.parse_json()

    # Our PIFO is simple: it just orchestrates two FIFOs. The boundary is 200.
    pifo = queues.Pifo(queues.Fifo([]), queues.Fifo([]), 200, False)

    ans = queues.operate_queue(commands, values, pifo)
    queue_util.dump_json(commands, values, ans)
