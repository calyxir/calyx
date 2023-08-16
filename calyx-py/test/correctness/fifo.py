# pylint: disable=import-error
import calyx.builder as cb
import calyx.builder_util as util
import calyx.queue_call as qc

MAX_QUEUE_LEN = 10


def insert_fifo(prog, name):
    """Inserts the component `fifo` into the program.

    It has:
    - one input, `cmd`.
    - one memory, `mem`, of size MAX_QUEUE_LEN.
    - two registers, `next_write` and `next_read`.
    - two ref registers, `ans` and `err`.
    """

    fifo: cb.ComponentBuilder = prog.component(name)
    cmd = fifo.input("cmd", 32)
    # If this is 0, we pop. If it is 1, we peek. Otherwise, we push the value.

    mem = fifo.seq_mem_d1("mem", 32, MAX_QUEUE_LEN, 32)
    write = fifo.reg("next_write", 32)  # The next address to write to
    read = fifo.reg("next_read", 32)  # The next address to read from
    # We will orchestrate `mem`, along with the two pointers above, to
    # simulate a circular queue of size MAX_QUEUE_LEN.

    ans = fifo.reg("ans", 32, is_ref=True)
    # If the user wants to pop, we will write the popped value to `ans`

    err = fifo.reg("err", 1, is_ref=True)
    # We'll raise this as a general error flag for overflow and underflow

    len = fifo.reg("len", 32)  # The length of the FIFO

    # Cells and groups to compute equality
    cmd_eq_0 = util.insert_eq(fifo, cmd, 0, "cmd_eq_0", 32)  # `cmd` == 0
    cmd_eq_1 = util.insert_eq(fifo, cmd, 1, "cmd_eq_1", 32)  # `cmd` == 1
    cmd_gt_1 = util.insert_gt(fifo, cmd, 1, "cmd_gt_1", 32)  # `cmd` > 1

    write_eq_max_queue_len = util.insert_eq(
        fifo, write.out, MAX_QUEUE_LEN, "write_eq_MAX_QUEUE_LEN", 32
    )  # `write` == MAX_QUEUE_LEN
    read_eq_max_queue_len = util.insert_eq(
        fifo, read.out, MAX_QUEUE_LEN, "read_eq_MAX_QUEUE_LEN", 32
    )  # `read` == MAX_QUEUE_LEN
    len_eq_0 = util.insert_eq(fifo, len.out, 0, "len_eq_0", 32)  # `len` == 0
    len_eq_max_queue_len = util.insert_eq(
        fifo, len.out, MAX_QUEUE_LEN, "len_eq_MAX_QUEUE_LEN", 32
    )  # `len` == MAX_QUEUE_LEN

    # Cells and groups to increment read and write registers
    write_incr = util.insert_incr(fifo, write, "write_incr")  # write++
    read_incr = util.insert_incr(fifo, read, "read_incr")  # read++
    len_incr = util.insert_incr(fifo, len, "len_incr")  # len++
    len_decr = util.insert_decr(fifo, len, "len_decr")  # len--

    # Cells and groups to modify flags, which are registers
    write_wrap = util.insert_reg_store(
        fifo, write, 0, "write_wraparound"
    )  # zero out `write`
    read_wrap = util.insert_reg_store(
        fifo, read, 0, "read_wraparound"
    )  # zero out `read`
    raise_err = util.insert_reg_store(fifo, err, 1, "raise_err")  # set `err` to 1
    zero_out_ans = util.insert_reg_store(fifo, ans, 0, "zero_out_ans")  # zero out `ans`

    # Load and store into an arbitary slot in memory
    write_to_mem = util.mem_store_seq_d1(
        fifo, mem, write.out, cmd, "write_payload_to_mem"
    )
    read_from_mem = util.mem_read_seq_d1(
        fifo, mem, read.out, "read_payload_from_mem_phase1"
    )
    write_to_ans = util.mem_write_seq_d1_to_reg(
        fifo, mem, ans, "read_payload_from_mem_phase2"
    )

    fifo.control += [
        cb.par(
            # Was it a pop or a push? We can do both cases in parallel.
            cb.if_(
                # Did the user call pop?
                cmd_eq_0[0].out,
                cmd_eq_0[1],
                cb.if_(
                    # Yes, the user called pop. But is the queue empty?
                    len_eq_0[0].out,
                    len_eq_0[1],
                    [raise_err, zero_out_ans],  # The queue is empty: underflow.
                    [  # The queue is not empty. Proceed.
                        read_from_mem,  # Read from the queue.
                        write_to_ans,  # Write the answer to the answer register.
                        read_incr,  # Increment the read pointer.
                        cb.if_(
                            # Wrap around if necessary.
                            read_eq_max_queue_len[0].out,
                            read_eq_max_queue_len[1],
                            read_wrap,
                        ),
                        len_decr,  # Decrement the length.
                    ],
                ),
            ),
            cb.if_(
                # Did the user call peek?
                cmd_eq_1[0].out,
                cmd_eq_1[1],
                cb.if_(  # Yes, the user called peek. But is the queue empty?
                    len_eq_0[0].out,
                    len_eq_0[1],
                    [raise_err, zero_out_ans],  # The queue is empty: underflow.
                    [  # The queue is not empty. Proceed.
                        read_from_mem,  # Read from the queue.
                        write_to_ans,  # Write the answer to the answer register.
                        # But don't increment the read pointer or change the length.
                    ],
                ),
            ),
            cb.if_(
                # Did the user call push?
                cmd_gt_1[0].out,
                cmd_gt_1[1],
                cb.if_(
                    # Yes, the user called push. But is the queue full?
                    len_eq_max_queue_len[0].out,
                    len_eq_max_queue_len[1],
                    [raise_err, zero_out_ans],  # The queue is full: overflow.
                    [  # The queue is not full. Proceed.
                        write_to_mem,  # Write to the queue.
                        write_incr,  # Increment the write pointer.
                        cb.if_(
                            # Wrap around if necessary.
                            write_eq_max_queue_len[0].out,
                            write_eq_max_queue_len[1],
                            write_wrap,
                        ),
                        len_incr,  # Increment the length.
                    ],
                ),
            ),
        ),
    ]

    return fifo


def build():
    """Top-level function to build the program."""
    prog = cb.Builder()
    fifo = insert_fifo(prog, "fifo")
    qc.insert_main(prog, fifo)
    return prog.program


if __name__ == "__main__":
    build().emit()
