# For usage, see gen_test_data.sh

import sys
import queues
import util


if __name__ == "__main__":
    max_cmds, len = int(sys.argv[1]), int(sys.argv[2])
    keepgoing = "--keepgoing" in sys.argv
    commands, values, ranks, _ = util.parse_json(True)
    binheap = queues.Binheap(len)
    ans = queues.operate_queue(
        binheap, max_cmds, commands, values, ranks=ranks, keepgoing=keepgoing
    )
    util.dump_json(commands, values, ans, ranks=ranks)
