# For usage, see gen_queue_data_expect.sh

import sys
import calyx.queues as queues
from calyx import queue_util

if __name__ == "__main__":
    max_cmds, len = int(sys.argv[1]), int(sys.argv[2])
    keepgoing = "--keepgoing" in sys.argv
    commands, values, ranks = queue_util.parse_json()
    binheap = queues.Binheap(len)
    ans = queues.operate_queue(binheap, max_cmds, commands, values, ranks=ranks, keepgoing=keepgoing)
    queue_util.dump_json(ans, commands, values, ranks=ranks)
