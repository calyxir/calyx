# For usage, see gen_queue_data_expect.sh
import sys
import calyx.queues as queues
from calyx import queue_util

if __name__ == "__main__":
    max_cmds, len, numflows = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
    #keepgoing = "--keepgoing" in sys.argv
    commands, values, _ = queue_util.parse_json()

    if numflows == 2:
        boundaries = [200, 400] 
    elif numflows == 3:
        boundaries = [133, 266, 400]
    elif numflows == 4:
        boundaries = [100, 200, 300, 400]
    elif numflows == 5:
        boundaries = [80, 160, 240, 320, 400]
    elif numflows ==6:
        boundaries = [66, 100, 200, 220, 300, 400]
    elif numflows == 7:
        boundaries = [50, 100, 150, 200, 250, 300, 400]


    # Our Round Robin Queue (formerly known as generalized pifo) is simple: it 
    # just orchestrates n FIFOs, in this case 3. It takes in a list of
    # boundaries of length n (in this case 3).
    pifo = queues.RRQueue(numflows, boundaries, len)

    ans = queues.operate_queue(pifo, max_cmds, commands, values, keepgoing=True)

    queue_util.dump_json(ans, commands, values)
