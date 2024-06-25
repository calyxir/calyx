import calyx.queues as queues
from calyx import queue_util

if __name__ == "__main__":
    commands, values, ranks, times = queue_util.parse_json(True, True)
    pcq = queues.PCQ(200)
    ans = queues.operate_queue(pcq, 200, commands, values, ranks, times=times)
    queue_util.dump_json(commands, values, ans)