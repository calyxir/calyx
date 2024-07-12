import sys
import calyx.queues as queues
from calyx import queue_util

if __name__ == "__main__":
    commands, values, ranks, times = queue_util.parse_json(True, True)
    max_cmds, len = int(sys.argv[1]), int(sys.argv[2])
    keepgoing = "--keepgoing" in sys.argv
    pcq = queues.PCQ(len)
    ans = queues.operate_queue(pcq, max_cmds, commands, values, ranks, times=times, keepgoing=keepgoing)
    queue_util.dump_json(commands, values, ans, ranks, times)