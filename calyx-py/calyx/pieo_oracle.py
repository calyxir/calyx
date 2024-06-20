import calyx.queues as queues
from calyx import queue_util

if __name__ == "__main__":
    commands, values, ranks, bounds = queue_util.parse_json(parse_ranks=True, parse_bounds=True)
    pieo = queues.Pieo([], False, 200)
    ans = queues.operate_queue(commands, values, ranks, bounds, pieo)
    queue_util.dump_json(commands, values, ans, ranks, bounds)
