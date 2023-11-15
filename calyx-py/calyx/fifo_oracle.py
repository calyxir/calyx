import calyx.queues as queues
import calyx.queue_util as queue_util

if __name__ == "__main__":
    commands, values = queue_util.parse_json()
    fifo = queues.Fifo([], queue_util.QUEUE_SIZE)
    ans = queues.operate_queue(commands, values, fifo)
    queue_util.dump_json(commands, values, ans)
