import calyx.queues as queues
import calyx.queue_util as queue_util

if __name__ == "__main__":
    commands, values = queue_util.parse_json()

    # Our PIFO is simple: it just orchestrates two FIFOs. The boundary is 200.
    pifo = queues.Pifo(
        queues.Fifo([], queue_util.QUEUE_SIZE),
        queues.Fifo([], queue_util.QUEUE_SIZE),
        200,
        queue_util.QUEUE_SIZE,
    )

    ans = queues.operate_queue(commands, values, pifo)
    queue_util.dump_json(commands, values, ans)
