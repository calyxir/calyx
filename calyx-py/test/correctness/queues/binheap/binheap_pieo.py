# pylint: disable=import-error
import sys
import binheap
import calyx.builder as cb
import calyx.queue_call as qc

def insert_pieo(prog, name, val_queue, time_queue, rank_queue, queue_len, stats=None, static=False):
    pieo = prog.component(name)

    queue_size_factor = 4

    # Declare the two sub-queues as cells of this component.
    val_queue = pieo.cell("val_queue", val_queue)
    time_queue = pieo.cell("time_queue", time_queue)
    rank_queue = pieo.cell("rank_queue", rank_queue)

    ans_mem = pieo.seq_mem_d1("out", 32, queue_len, queue_size_factor, is_external=True)

    ans = pieo.reg(32)
    err = pieo.reg(1)

    ans_index = pieo.reg(32)

    def push(value, rank, time=0):
        """Push an element into the heap (timewise) """
        #Parallelly push value, time and rank into their respective heaps
        return cb.par(
            cb.invoke(
                val_queue,
                in_value=cb.const(32, value),
                in_rank=cb.const(64, rank),
                in_cmd=cb.const(2, 2),
                ref_ans=ans,
                ref_err=err,
            ),
            cb.invoke(
                time_queue,
                in_value=cb.const(32, time),
                in_rank=cb.const(64, rank),
                in_cmd=cb.const(2, 2),
                ref_ans=ans,
                ref_err=err,
            ),
            cb.invoke(
                rank_queue,
                in_value=cb.const(32, time),
                in_rank=cb.const(64, rank),
                in_cmd=cb.const(2, 2),
                ref_ans=ans,
                ref_err=err,
            )
        )

    def peek_by_value(value, time):
        #Scan every element of the heap until the correct one is found
        queue_index = pieo.reg(32)

        #Stores accessed times from popping queue
        time_ans = pieo.reg(32)
        val_ans = pieo.reg(32)
        rank_ans = pieo.reg(32)


        cached_vals = pieo.seq_mem_d1("cached_vals", 32, queue_len, 32)
        cached_times = pieo.seq_mem_d1("cached_times", 32, queue_len, 32)
        cached_ranks = pieo.seq_mem_d1("cached_ranks", 32, queue_len, 32)

        #Equality checkers
        value_eq = pieo.eq_use(value, time_ans)
        time_leq = pieo.le_use(val_ans, time)
        overflow_check = pieo.lt_use(queue_index, queue_len)

        return [
            cb.seq(
                cb.while_with ((value_eq & time_leq & overflow_check),
                    cb.seq([
                        cb.par(
                            [cb.invoke(
                                q,
                                in_value=0,
                                in_rank=0,
                                in_cmd=0, #Pop from queue
                                ref_ans=ans,
                                ref_err=err,
                            )]
                            for (q, ans) in ((val_queue, val_ans), (time_queue, time_ans), (rank_queue, rank_ans))
                        ),
                        pieo.mem_store_d1(cached_vals, queue_index, val_ans, "cache_vals"),
                        pieo.mem_store_d1(cached_times, queue_index, time_ans, "cache_times"),
                        pieo.mem_store_d1(cached_ranks, queue_index, rank_ans, "cache_ranks"),
                        pieo.incr(queue_index)
                    ])
                ),
                cb.if_with((value_eq & time_leq),
                    cb.seq([
                        pieo.mem_store_d1(ans_mem, ans_index, val_ans, ""),
                        cb.incr(ans_index),

                        cb.
                            cb.par(
                                cb.invoke(
                                    val_queue,
                                    in_value=cb.const(32, v),
                                    in_rank=cb.const(64, r),
                                    in_cmd=cb.const(2, 2),
                                    ref_ans=ans,
                                    ref_err=err,
                                ),
                                cb.invoke(
                                    time_queue,
                                    in_value=cb.const(32, v),
                                    in_rank=cb.const(64, r),
                                    in_cmd=cb.const(2, 2),
                                    ref_ans=ans,
                                    ref_err=err,
                                ),
                                cb.invoke(
                                    rank_queue,
                                    in_value=cb.const(32, r),
                                    in_rank=cb.const(64, r),
                                    in_cmd=cb.const(2, 2),
                                    ref_ans=ans,
                                    ref_err=err,
                                )
                            )
                            for (v, t, r) in 
                    )
                )
            )
            # comp.mem_store_d1(out, index - 1, ans.out, f"store_ans_{index}"),
        ]

    def pop_and_store():
        queue_index = pieo.reg(32)

        return [
            cb.invoke(
                val_queue,
                in_value=cb.const(32, 50),
                in_rank=cb.const(64, 50),
                in_cmd=cb.const(2, 0),
                ref_ans=ans,
                ref_err=err,
            ),
            pieo.mem_store_d1(ans_mem, index - 1, ans.out, f"store_ans_{index}"),
        ]

def build():
    """Top-level function to build the program."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv
    prog = cb.Builder()
    val_queue = binheap.insert_binheap(prog, "val_queue", 4, 32, 32)
    time_queue = binheap.insert_binheap(prog, "time_queue", 4, 32, 32)
    rank_queue = binheap.insert_binheap(prog, "rank_queue", 4, 32, 32)
    pieo = insert_pieo(prog, "pieo", val_queue, time_queue, rank_queue, 16)
    qc.insert_main(prog, pieo, num_cmds, keepgoing=keepgoing)
    return prog.program


if __name__ == "__main__":
    build().emit()