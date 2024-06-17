import calyx.queues as queues
from calyx import queue_util

if __name__ == "__main__":
    commands, values = queue_util.parse_json()
    fifo = queues.Fifo(16)
    ans = queues.operate_queue(commands, values, fifo)
    queue_util.dump_json(commands, values, ans)
