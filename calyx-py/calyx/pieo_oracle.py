import sys
import calyx.queues as queues
from calyx import queue_util

if __name__ == "__main__":
    commands, values, ranks, times = queue_util.parse_json(parse_ranks=True, parse_times=True)
    keepgoing = "--keepgoing" in sys.argv
    pieo = queues.Pieo(200)
    ans = queues.operate_queue(pieo, 200, commands, values, ranks, keepgoing=keepgoing, times=times)
    queue_util.dump_json(commands, values, ans, ranks, times)