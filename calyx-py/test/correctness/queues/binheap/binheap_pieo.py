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

    num_elements = pieo.reg(32)
    ans_index = pieo.reg(32)

    overflow_check = pieo.le_use(num_elements.out, queue_len.out)
    underflow_check = pieo.ge_use(num_elements.out, 0)

    def push(value, rank, time):
        """Push an element into the heap (timewise) """
        #In parallel, push value, time and rank into their respective heaps
        return cb.if_with(overflow_check,
            cb.seq(
                cb.par(
                    cb.invoke(
                        val_queue,
                        in_value=cb.const(32, value.out),
                        in_rank=cb.const(64, rank.out),
                        in_cmd=cb.const(2, 2),
                        ref_ans=ans,
                        ref_err=err,
                    ),
                    cb.invoke(
                        time_queue,
                        in_value=cb.const(32, time.out),
                        in_rank=cb.const(64, rank.out),
                        in_cmd=cb.const(2, 2),
                        ref_ans=ans,
                        ref_err=err,
                    ),
                    cb.invoke(
                        rank_queue,
                        in_value=cb.const(32, time.out),
                        in_rank=cb.const(64, rank.out),
                        in_cmd=cb.const(2, 2),
                        ref_ans=ans,
                        ref_err=err,
                    )
                ),
                pieo.incr(num_elements)
            )
        )

    def peek_by_time(time):
        #Scan every element of the heap until the correct one is found
        
        queue_index = pieo.reg(32) #Tracker while scanning through heap
        replace_tracker = pieo.reg(32) #Loop counter while writing elements back into heap

        #Stores accessed times from popping queue
        ready_time = pieo.reg(32)
        val_ans = pieo.reg(32)
        rank_ans = pieo.reg(32)

        #Load when writing back to queue
        cached_time = pieo.reg(32)
        cached_val = pieo.reg(32)
        cached_rank = pieo.reg(32)

        cached_vals = pieo.seq_mem_d1("cached_vals", 32, queue_len.out, 32)
        cached_times = pieo.seq_mem_d1("cached_times", 32, queue_len.out, 32)
        cached_ranks = pieo.seq_mem_d1("cached_ranks", 32, queue_len.out, 32)

        #Equality checkers
        time_leq = pieo.le_use(ready_time.out, time.out)
        pop_count = pieo.lt_use(queue_index.out, num_elements.out)
        replace_count = pieo.le_use(replace_tracker.out, queue_index.out)

        return cb.seq (
            #Scan through heaps and pop value, time and rank until we either run out or find an accurate one
            cb.while_with ((time_leq & pop_count),
                cb.seq([
                    cb.par([
                        cb.invoke(
                            q,
                            in_value=0,
                            in_rank=0,
                            in_cmd=0, #Pop from queue
                            ref_ans=ans,
                            ref_err=err)
                        for (q, ans) in ((val_queue, val_ans), (time_queue, ready_time), (rank_queue, rank_ans))
                    ]),
                    pieo.mem_store_d1(cached_vals, queue_index.out, val_ans.out, "cache_vals"),
                    pieo.mem_store_d1(cached_times, queue_index.out, ready_time.out, "cache_times"),
                    pieo.mem_store_d1(cached_ranks, queue_index.out, rank_ans.out, "cache_ranks"),
                    pieo.incr(queue_index)
                ])
            ),

            #Write elements back
            cb.while_with(replace_count,
                cb.seq([
                    #At each iteration, load the cached elements into registers
                    cb.par([
                        pieo.mem_load_d1(cached_vals, replace_tracker.out, cached_val),
                        pieo.mem_load_d1(cached_times, replace_tracker.out, cached_time),
                        pieo.mem_load_d1(cached_ranks, replace_tracker.out, cached_rank),
                    ]),

                    #Concurrently write them back into memory, using the cached rank to determine location
                    cb.par([
                        cb.invoke(
                            val_queue,
                            in_value=cb.const(32, cached_val.out),
                            in_rank=cb.const(64, cached_rank.out),
                            in_cmd=cb.const(2, 2),
                            ref_ans=ans,
                            ref_err=err
                        ),

                        cb.invoke(
                            time_queue,
                            in_value=cb.const(32, cached_time.out),
                            in_rank=cb.const(64, cached_rank.out),
                            in_cmd=cb.const(2, 2),
                            ref_ans=ans,
                            ref_err=err
                        ),
                        cb.invoke(
                            rank_queue,
                            in_value=cb.const(32, cached_rank.out),
                            in_rank=cb.const(64, cached_rank.out),
                            in_cmd=cb.const(2, 2),
                            ref_ans=ans,
                            ref_err=err,
                        )
                    ]), pieo.incr(replace_tracker)
                ])
            ),

            #If the correct element was found, write it to ans_mem
            cb.if_with(time_leq,
                cb.seq(
                    pieo.mem_store_d1(ans_mem, ans_index, val_ans, "store_mem"),
                    pieo.incr(ans_index)
                )
            )
        )

    def peek_by_value(value, time):
        #Scan every element of the heap until the correct one is found
        
        queue_index = pieo.reg(32) #Tracker while scanning through heap
        replace_tracker = pieo.reg(32) #Loop counter while writing elements back into heap

        #Stores accessed times from popping queue
        ready_time = pieo.reg(32)
        val_ans = pieo.reg(32)
        rank_ans = pieo.reg(32)

        #Load when writing back to queue
        cached_time = pieo.reg(32)
        cached_val = pieo.reg(32)
        cached_rank = pieo.reg(32)

        cached_vals = pieo.seq_mem_d1("cached_vals", 32, queue_len.out, 32)
        cached_times = pieo.seq_mem_d1("cached_times", 32, queue_len.out, 32)
        cached_ranks = pieo.seq_mem_d1("cached_ranks", 32, queue_len.out, 32)

        #Equality checkers
        value_eq = pieo.eq_use(value.out, val_ans.out)
        time_leq = pieo.le_use(ready_time.out, time.out)
        pop_count = pieo.lt_use(queue_index.out, num_elements.out)
        replace_count = pieo.le_use(replace_tracker.out, queue_index.out)

        return cb.seq(
            #Scan through heaps and pop value, time and rank until we either run out or find an accurate one
            cb.while_with ((value_eq & time_leq & pop_count),
                cb.seq([
                    cb.par([
                        cb.invoke(
                            q,
                            in_value=0,
                            in_rank=0,
                            in_cmd=0, #Pop from queue
                            ref_ans=ans,
                            ref_err=err)
                        for (q, ans) in ((val_queue, val_ans), (time_queue, ready_time), (rank_queue, rank_ans))
                    ]),
                    pieo.mem_store_d1(cached_vals, queue_index.out, val_ans.out, "cache_vals"),
                    pieo.mem_store_d1(cached_times, queue_index.out, ready_time.out, "cache_times"),
                    pieo.mem_store_d1(cached_ranks, queue_index.out, rank_ans.out, "cache_ranks"),
                    pieo.incr(queue_index)
                ])
            ),

            #Write elements back
            cb.while_with(replace_count,
                cb.seq([
                    #At each iteration, load the cached elements into registers
                    cb.par([
                        pieo.mem_load_d1(cached_vals, replace_tracker.out, cached_val),
                        pieo.mem_load_d1(cached_times, replace_tracker.out, cached_time),
                        pieo.mem_load_d1(cached_ranks, replace_tracker.out, cached_rank),
                    ]),

                    #Concurrently write them back into memory, using the cached rank to determine location
                    cb.par([
                        cb.invoke(
                            val_queue,
                            in_value=cb.const(32, cached_val.out),
                            in_rank=cb.const(64, cached_rank.out),
                            in_cmd=cb.const(2, 2),
                            ref_ans=ans,
                            ref_err=err
                        ),

                        cb.invoke(
                            time_queue,
                            in_value=cb.const(32, cached_time.out),
                            in_rank=cb.const(64, cached_rank.out),
                            in_cmd=cb.const(2, 2),
                            ref_ans=ans,
                            ref_err=err
                        ),
                        cb.invoke(
                            rank_queue,
                            in_value=cb.const(32, cached_rank.out),
                            in_rank=cb.const(64, cached_rank.out),
                            in_cmd=cb.const(2, 2),
                            ref_ans=ans,
                            ref_err=err,
                        )
                    ]), pieo.incr(replace_tracker)
                ])
            ),

            #If the correct element was found, write it to ans_mem
            cb.if_with((value_eq & time_leq),
                cb.seq(
                    pieo.mem_store_d1(ans_mem, ans_index, val_ans, "store_mem"),
                    cb.incr(ans_index)
                )
            )
        )
    
    def pop_by_time(time):
        #Scan every element of the heap until the correct one is found
        
        queue_index = pieo.reg(32) #Tracker while scanning through heap
        replace_tracker = pieo.reg(32) #Loop counter while writing elements back into heap

        #Stores accessed times from popping queue
        ready_time = pieo.reg(32)
        val_ans = pieo.reg(32)
        rank_ans = pieo.reg(32)

        #Load when writing back to queue
        cached_time = pieo.reg(32)
        cached_val = pieo.reg(32)
        cached_rank = pieo.reg(32)

        cached_vals = pieo.seq_mem_d1("cached_vals", 32, queue_len.out, 32)
        cached_times = pieo.seq_mem_d1("cached_times", 32, queue_len.out, 32)
        cached_ranks = pieo.seq_mem_d1("cached_ranks", 32, queue_len.out, 32)

        #Equality checkers
        time_leq = pieo.le_use(ready_time.out, time.out)
        pop_count = pieo.lt_use(queue_index.out, num_elements.out)
        replace_count = pieo.le_use(replace_tracker.out, queue_index.out)

        return cb.seq (
            #Scan through heaps and pop value, time and rank until we either run out or find an accurate one
            cb.while_with ((time_leq & pop_count),
                cb.seq([
                    cb.par([
                        cb.invoke(
                            q,
                            in_value=0,
                            in_rank=0,
                            in_cmd=0, #Pop from queue
                            ref_ans=ans,
                            ref_err=err)
                        for (q, ans) in ((val_queue, val_ans), (time_queue, ready_time), (rank_queue, rank_ans))
                    ]),
                    pieo.mem_store_d1(cached_vals, queue_index.out, val_ans.out, "cache_vals"),
                    pieo.mem_store_d1(cached_times, queue_index.out, ready_time.out, "cache_times"),
                    pieo.mem_store_d1(cached_ranks, queue_index.out, rank_ans.out, "cache_ranks"),
                    pieo.incr(queue_index)
                ])
            ),

            #Write elements back
            cb.while_with(replace_count,
                cb.seq([
                    #At each iteration, load the cached elements into registers
                    cb.par([
                        pieo.mem_load_d1(cached_vals, replace_tracker.out, cached_val),
                        pieo.mem_load_d1(cached_times, replace_tracker.out, cached_time),
                        pieo.mem_load_d1(cached_ranks, replace_tracker.out, cached_rank),
                    ]),

                    #Concurrently write them back into memory, using the cached rank to determine location
                    cb.par([
                        cb.invoke(
                            val_queue,
                            in_value=cb.const(32, cached_val.out),
                            in_rank=cb.const(64, cached_rank.out),
                            in_cmd=cb.const(2, 2),
                            ref_ans=ans,
                            ref_err=err
                        ),

                        cb.invoke(
                            time_queue,
                            in_value=cb.const(32, cached_time.out),
                            in_rank=cb.const(64, cached_rank.out),
                            in_cmd=cb.const(2, 2),
                            ref_ans=ans,
                            ref_err=err
                        ),
                        cb.invoke(
                            rank_queue,
                            in_value=cb.const(32, cached_rank.out),
                            in_rank=cb.const(64, cached_rank.out),
                            in_cmd=cb.const(2, 2),
                            ref_ans=ans,
                            ref_err=err,
                        )
                    ]), pieo.incr(replace_tracker)
                ])
            ),

            #If the correct element was found, write it to ans_mem
            cb.if_with(time_leq,
                cb.seq(
                    pieo.mem_store_d1(ans_mem, ans_index, val_ans, "store_mem"),
                    pieo.incr(ans_index)
                )
            )
        )

    def pop_by_value(value, time):
        #Scan every element of the heap until the correct one is found
        
        queue_index = pieo.reg(32) #Tracker while scanning through heap
        replace_tracker = pieo.reg(32) #Loop counter while writing elements back into heap

        #Stores accessed times from popping queue
        ready_time = pieo.reg(32)
        val_ans = pieo.reg(32)
        rank_ans = pieo.reg(32)

        #Load when writing back to queue
        cached_time = pieo.reg(32)
        cached_val = pieo.reg(32)
        cached_rank = pieo.reg(32)

        cached_vals = pieo.seq_mem_d1("cached_vals", 32, queue_len.out, 32)
        cached_times = pieo.seq_mem_d1("cached_times", 32, queue_len.out, 32)
        cached_ranks = pieo.seq_mem_d1("cached_ranks", 32, queue_len.out, 32)

        #Equality checkers
        value_eq = pieo.eq_use(value.out, val_ans.out)
        time_leq = pieo.le_use(ready_time.out, time.out)
        pop_count = pieo.lt_use(queue_index.out, num_elements.out)
        replace_count = pieo.le_use(replace_tracker.out, queue_index.out)

        return cb.seq(
            #Scan through heaps and pop value, time and rank until we either run out or find an accurate one
            cb.while_with ((value_eq & time_leq & pop_count),
                cb.seq([
                    cb.par([
                        cb.invoke(
                            q,
                            in_value=0,
                            in_rank=0,
                            in_cmd=0, #Pop from queue
                            ref_ans=ans,
                            ref_err=err)
                        for (q, ans) in ((val_queue, val_ans), (time_queue, ready_time), (rank_queue, rank_ans))
                    ]),
                    pieo.mem_store_d1(cached_vals, queue_index.out, val_ans.out, "cache_vals"),
                    pieo.mem_store_d1(cached_times, queue_index.out, ready_time.out, "cache_times"),
                    pieo.mem_store_d1(cached_ranks, queue_index.out, rank_ans.out, "cache_ranks"),
                    pieo.incr(queue_index)
                ])
            ),

            #Write elements back
            cb.while_with(replace_count,
                cb.seq([
                    #At each iteration, load the cached elements into registers
                    cb.par([
                        pieo.mem_load_d1(cached_vals, replace_tracker.out, cached_val),
                        pieo.mem_load_d1(cached_times, replace_tracker.out, cached_time),
                        pieo.mem_load_d1(cached_ranks, replace_tracker.out, cached_rank),
                    ]),

                    #Concurrently write them back into memory, using the cached rank to determine location
                    cb.par([
                        cb.invoke(
                            val_queue,
                            in_value=cb.const(32, cached_val.out),
                            in_rank=cb.const(64, cached_rank.out),
                            in_cmd=cb.const(2, 2),
                            ref_ans=ans,
                            ref_err=err
                        ),

                        cb.invoke(
                            time_queue,
                            in_value=cb.const(32, cached_time.out),
                            in_rank=cb.const(64, cached_rank.out),
                            in_cmd=cb.const(2, 2),
                            ref_ans=ans,
                            ref_err=err
                        ),
                        cb.invoke(
                            rank_queue,
                            in_value=cb.const(32, cached_rank.out),
                            in_rank=cb.const(64, cached_rank.out),
                            in_cmd=cb.const(2, 2),
                            ref_ans=ans,
                            ref_err=err,
                        )
                    ]), pieo.incr(replace_tracker)
                ])
            ),

            #If the correct element was found, write it to ans_mem
            cb.if_with((value_eq & time_leq),
                cb.seq(
                    pieo.mem_store_d1(ans_mem, ans_index, val_ans, "store_mem"),
                    cb.incr(ans_index)
                )
            )
        )

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