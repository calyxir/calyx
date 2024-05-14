# pylint: disable=import-error
import calyx.builder as cb
import calyx.queue_call as qc

# This determines the maximum possible length of the queue:
# The max length of the queue will be 2^QUEUE_LEN_FACTOR.
QUEUE_LEN_FACTOR = 4


def insert_fifo(prog, name, queue_len_factor=QUEUE_LEN_FACTOR):
    """Inserts the component `fifo` into the program.

    It has:
    - two inputs, `cmd` and `value`.
    - one memory, `mem`, of size 10.
    - two registers, `next_write` and `next_read`.
    - two ref registers, `ans` and `err`.
    """

    fifo: cb.ComponentBuilder = prog.component(name)
    cmd = fifo.input("cmd", 2)
    # If it is 0, we pop.
    # If it is 1, we peek.
    # If it is 2, we push `value` to the queue.
    value = fifo.input("value", 32)  # The value to push to the queue

    max_queue_len = 2**queue_len_factor
    mem = fifo.seq_mem_d1("mem", 32, max_queue_len, queue_len_factor)
    write = fifo.reg(queue_len_factor, "next_write")  # The next address to write to
    read = fifo.reg(queue_len_factor, "next_read")  # The next address to read from
    # We will orchestrate `mem`, along with the two pointers above, to
    # simulate a circular queue of size 2^queue_len_factor.

    ans = fifo.reg(32, "ans", is_ref=True)
    # If the user wants to pop or peek, we will write the value to `ans`.

    err = fifo.reg(1, "err", is_ref=True)
    # We'll raise this as a general error flag for overflow and underflow.

    len = fifo.reg(32)  # The active length of the FIFO.

    # Cells and groups to check which command we got.
    cmd_eq_0 = fifo.eq_use(cmd, 0)
    cmd_lt_2 = fifo.lt_use(cmd, 2)
    cmd_eq_2 = fifo.eq_use(cmd, 2)

    len_eq_0 = fifo.eq_use(len.out, 0)
    len_eq_max_queue_len = fifo.eq_use(len.out, max_queue_len)

    # Cells and groups to increment read and write registers
    write_incr = fifo.incr(write)  # write++
    read_incr = fifo.incr(read)  # read++
    len_incr = fifo.incr(len)  # len++
    len_decr = fifo.decr(len)  # len--

    raise_err = fifo.reg_store(err, 1, "raise_err")  # err := 1

    # Load and store into an arbitary slot in memory
    write_to_mem = fifo.mem_store_seq_d1(mem, write.out, value, "write_payload_to_mem")
    read_from_mem = fifo.mem_read_seq_d1(mem, read.out, "read_payload_from_mem_phase1")
    write_to_ans = fifo.mem_write_seq_d1_to_reg(
        mem, ans, "read_payload_from_mem_phase2"
    )

    fifo.control += cb.par(
        # Was it a (pop/peek), or a push? We can do those two cases in parallel.
        # The logic is shared for pops and peeks, with just a few differences.
        cb.if_with(
            # Did the user call pop/peek?
            cmd_lt_2,
            cb.if_with(
                # Yes, the user called pop/peek. But is the queue empty?
                len_eq_0,
                raise_err,  # The queue is empty: underflow.
                [  # The queue is not empty. Proceed.
                    read_from_mem,  # Read from the queue.
                    write_to_ans,  # Write the answer to the answer register.
                    cb.if_with(
                        cmd_eq_0,  # Did the user call pop?
                        [  # Yes, so we have some work to do besides peeking.
                            read_incr,  # Increment the read pointer.
                            len_decr,  # Decrement the active length.
                        ],
                    ),
                ],
            ),
        ),
        cb.if_with(
            # Did the user call push?
            cmd_eq_2,
            cb.if_with(
                # Yes, the user called push. But is the queue full?
                len_eq_max_queue_len,
                raise_err,  # The queue is empty: underflow.
                [  # The queue is not full. Proceed.
                    write_to_mem,  # Write `value` to the queue.
                    write_incr,  # Increment the write pointer.
                    len_incr,  # Increment the active length.
                ],
            ),
        ),
    )

    return fifo


def build():
    """Top-level function to build the program."""
    prog = cb.Builder()
    fifo = insert_fifo(prog, "fifo")
    qc.insert_main(prog, fifo)
    return prog.program


if __name__ == "__main__":
    build().emit()
