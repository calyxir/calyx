import queues
import queue_util

if __name__ == "__main__":
    commands, values = queue_util.parse_json()
    pifo = queues.Fifo([])
    ans = queues.operate_queue(commands, values, pifo)
    queue_util.dump_json(commands, values, ans)
