# For usage, see gen_queue_data_expect.sh

import sys
import calyx.queues as queues
from calyx import queue_util

if __name__ == "__main__":
    max_cmds, len = int(sys.argv[1]), int(sys.argv[2])
    keepgoing = "--keepgoing" in sys.argv
    commands, values, _ = queue_util.parse_json()

    # Our PIFO is simple: it just orchestrates two FIFOs. The boundary is 200.
    pifo = queues.Pifo(queues.Fifo(len), queues.Fifo(len), 200, len)

    ans = queues.operate_queue(pifo, max_cmds, commands, values, keepgoing=keepgoing)
    queue_util.dump_json(ans, commands, values)
