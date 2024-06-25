# For usage, see gen_queue_data_expect.sh
import sys
import calyx.queues as queues
from calyx import queue_util

if __name__ == "__main__":
    max_cmds, len = int(sys.argv[1]), int(sys.argv[2])
    keepgoing = "--keepgoing" in sys.argv
    commands, values = queue_util.parse_json()

    # Our Round Robin Queue (formerly known as generalized pifo) is simple: it 
    # just orchestrates n FIFOs, in this case 3. It takes in a list of
    # boundaries of length n (in this case 3).
    pifo = queues.RRQueue(3, [133, 266, 400], len)

    ans = queues.operate_queue(commands, values, pifo, max_cmds, keepgoing=True)
    queue_util.dump_json(commands, values, ans)