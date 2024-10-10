# For usage, see gen_queue_data_expect.sh

import sys
import queues
import util


if __name__ == "__main__":
    max_cmds, len = int(sys.argv[1]), int(sys.argv[2])
    keepgoing = "--keepgoing" in sys.argv
    commands, values, _, _ = util.parse_json()
    fifo = queues.Fifo(len)
    ans = queues.operate_queue(fifo, max_cmds, commands, values, keepgoing=keepgoing)
    util.dump_json(commands, values, ans)
