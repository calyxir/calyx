import calyx.queues as queues
from calyx import queue_util

if __name__ == "__main__":
    commands, values = queue_util.parse_json()
    pcq = queues.CalendarQueue([], 0, False, 200)
    ans = queues.operate_queue(commands, values, pcq)
    queue_util.dump_json(commands, values, ans)