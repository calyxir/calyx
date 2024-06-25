import calyx.queues as queues
from calyx import queue_util

if __name__ == "__main__":
    commands, values = queue_util.parse_json()
    pcq = queues.PCQ([])
    ans = queues.operate_queue(pcq, 200, commands, values)
    queue_util.dump_json(commands, values, ans)