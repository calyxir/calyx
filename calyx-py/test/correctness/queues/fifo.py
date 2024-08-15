# pylint: disable=import-error
import sys
import calyx.builder as cb
import calyx.queue_call as qc
from calyx.utils import bits_needed

# This determines the maximum possible length of the queue:
# The max length of the queue will be 2^QUEUE_LEN_FACTOR.
QUEUE_LEN_FACTOR = 4


def insert_fifo(prog, name, queue_len_factor=QUEUE_LEN_FACTOR, val_width=32):
    """Inserts the component `fifo` into the program.

    It has:
    - two inputs, `cmd` and `value`.
    - one memory, `mem`, of size 10.
    - two registers, `next_write` and `next_read`.
    - two ref registers, `ans` and `err`.
    """

    fifo: cb.ComponentBuilder = prog.component(name)
    cmd = fifo.input("cmd", 1)
    # If it is 0, we pop.
    # If it is 1, we push `value` to the queue.
    value = fifo.input("value", val_width)  # The value to push to the queue

    max_queue_len = 2**queue_len_factor
    mem = fifo.seq_mem_d1("mem", val_width, max_queue_len, queue_len_factor)
    write = fifo.reg(queue_len_factor)  # The next address to write to
    read = fifo.reg(queue_len_factor)  # The next address to read from
    # We will orchestrate `mem`, along with the two pointers above, to
    # simulate a circular queue of size 2^queue_len_factor.

    ans = fifo.reg(val_width, "ans", is_ref=True)
    # If the user wants to pop or peek, we will write the value to `ans`.
    err = fifo.reg(1, "err", is_ref=True)
    # We'll raise this as a general error flag for overflow and underflow.
    len = fifo.reg(bits_needed(max_queue_len))  # The active length of the FIFO.
    raise_err = fifo.reg_store(err, 1, "raise_err")  # err := 1

    # The user called pop.
    # If the queue is empty, we should raise an error.
    # Otherwise, we should proceed with the core logic
    pop_logic = cb.if_with(
        fifo.eq_use(len.out, 0),
        raise_err,
        [
            # `pop` has been called, and the queue is not empty.
            # Write the answer to the answer register, increment `read`, and decrement `len`.
            fifo.mem_load_d1(mem, read.out, ans, "read_payload_from_mem_pop"),
            fifo.incr(read),
            fifo.decr(len),
        ],
    )

    # The user called push.
    # If the queue is full, we should raise an error.
    # Otherwise, we should proceed with the core logic.
    push_logic = cb.if_with(
        fifo.eq_use(len.out, max_queue_len),
        raise_err,
        [  # `push` has been called and the queue is not full.
            # Write `value` to the queue, and increment `write` and `len`.
            fifo.mem_store_d1(mem, write.out, value, "write_payload_to_mem"),
            fifo.incr(write),
            fifo.incr(len),
        ],
    )

    # Was it a pop or push?
    # We can do those two cases in parallel.
    fifo.control += fifo.case(
        cmd,
        {
            0: pop_logic,
            1: push_logic
        },
    )

    return fifo


def build():
    """Top-level function to build the program."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv
    prog = cb.Builder()
    fifo = insert_fifo(prog, "fifo")
    qc.insert_main(prog, fifo, num_cmds, keepgoing=keepgoing)
    return prog.program


if __name__ == "__main__":
    build().emit()
