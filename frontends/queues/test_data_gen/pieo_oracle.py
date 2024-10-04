# For usage, see gen_queue_data_expect.sh

import sys
import queues
import util


if __name__ == "__main__":
    commands, values, ranks, times = util.parse_json(True, True)
    max_cmds, len = int(sys.argv[1]), int(sys.argv[2])
    keepgoing = "--keepgoing" in sys.argv
    pieo = queues.Pieo(len)
    ans = queues.operate_queue(
        pieo, max_cmds, commands, values, ranks, times=times, keepgoing=keepgoing
    )
    util.dump_json(commands, values, ans, ranks, times)
