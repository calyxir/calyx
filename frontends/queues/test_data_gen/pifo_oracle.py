# For usage, see gen_queue_data_expect.sh

import sys
import queues
import util


if __name__ == "__main__":
    max_cmds, len = int(sys.argv[1]), int(sys.argv[2])
    keepgoing = "--keepgoing" in sys.argv
    commands, values, _, _ = util.parse_json()

    # Our PIFO is simple: it just orchestrates two FIFOs. The boundary is 200.
    pifo = queues.Pifo(queues.Fifo(len), queues.Fifo(len), 200, len)

    ans = queues.operate_queue(pifo, max_cmds, commands, values, keepgoing=keepgoing)
    util.dump_json(commands, values, ans)
