# For usage, see gen_queue_data_expect.sh
import sys
import calyx.queues as queues
from calyx import queue_util

if __name__ == "__main__":
    num_cmds, len, numflows = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
    keepgoing = "--keepgoing" in sys.argv
    commands, values, _, _ = queue_util.parse_json()

    # In reality, we would allow the user to specify the boundaries via
    # command line arguments or a configuration file. For now, we hardcode them
    # as a function of the number of flows.
    if numflows == 2:
        boundaries = [200, 400]
    elif numflows == 3:
        boundaries = [133, 266, 400]
    elif numflows == 4:
        boundaries = [100, 200, 300, 400]
    elif numflows == 5:
        boundaries = [80, 160, 240, 320, 400]
    elif numflows == 6:
        boundaries = [66, 100, 200, 220, 300, 400]
    elif numflows == 7:
        boundaries = [50, 100, 150, 200, 250, 300, 400]
    else:
        raise ValueError("Unsupported number of flows")
    
    subqueues = [queues.Fifo(len) for _ in range(numflows)]

    # Our Round Robin Queue orchestrates n subqueues, in this case provided as
    # a command line argument. It orchestrates the subqueues in a round-robin fashion.
    pifo = queues.RRQueue(numflows, boundaries, subqueues, len)

    ans = queues.operate_queue(pifo, num_cmds, commands, values, keepgoing=keepgoing)

    queue_util.dump_json(commands, values, ans)
