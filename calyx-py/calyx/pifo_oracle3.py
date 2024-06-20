# For usage, see gen_queue_data_expect.sh

import sys
import calyx.queues as queues
from calyx import queue_util

if __name__ == "__main__":
    max_cmds, len = int(sys.argv[1]), int(sys.argv[2])
    commands, values = queue_util.parse_json()

    # Our PIFO is simple: it just orchestrates three FIFOs. The boundary is 200.
    pifo = queues.NPifo(3, 200, len)

    ans = queues.operate_queue(commands, values, pifo, max_cmds)
    queue_util.dump_json(commands, values, ans)
