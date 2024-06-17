# For usage, see gen_queue_data_expect.sh

import sys
import calyx.queues as queues
from calyx import queue_util

if __name__ == "__main__":
    len = int(sys.argv[1])
    commands, values = queue_util.parse_json()
    fifo = queues.Fifo(len)
    ans = queues.operate_queue(commands, values, fifo)
    queue_util.dump_json(commands, values, ans)
