# pylint: disable=import-error
import sys
import binheap
import calyx.builder as cb
import calyx.queue_call as qc

def insert_pieo(prog, name, val_queue, time_queue, rank_queue, queue_len, stats=None, static=False):
    pieo = prog.component(name)
    num_commands = 20000

    #External memory cells
    commands = pieo.seq_mem_d1("commands", 32, num_commands, 32, is_external=True)
    values = pieo.seq_mem_d1("values", 32, num_commands, 32, is_external=True)
    times = pieo.seq_mem_d1("times", 32, num_commands, 32, is_external=True)
    ranks = pieo.seq_mem_d1("ranks", 32, num_commands, 32, is_external=True)
    ans_mem = pieo.seq_mem_d1("ans_mem", 32, num_commands, 32, is_external=True)

    cmd_idx = pieo.reg(32)
    incr_cmd_idx = pieo.incr(cmd_idx)
    cmd_in_range = pieo.lt_use(cmd_idx.out, num_commands)

    cmd, value, time, rank = pieo.reg(32), pieo.reg(32), pieo.reg(32), pieo.reg(32)

    loads = [
        pieo.mem_load_d1(commands, cmd_idx.out, cmd, "load_cmd"),
        pieo.mem_load_d1(values, cmd_idx.out, value, "load_val"),
        pieo.mem_load_d1(times, cmd_idx.out, time, "load_time"),
        pieo.mem_load_d1(ranks, cmd_idx.out, rank, "load_rank")
    ]

    cmd_eqs = [
        pieo.eq_use(cmd.out, 0),
        pieo.eq_use(cmd.out, 1),
        pieo.eq_use(cmd.out, 2),
        pieo.eq_use(cmd.out, 3),
        pieo.eq_use(cmd.out, 4),
    ]

    # Declare the two sub-queues as cells of this component.
    val_queue = pieo.cell("val_queue", val_queue)
    time_queue = pieo.cell("time_queue", time_queue)
    rank_queue = pieo.cell("rank_queue", rank_queue)

    #Used to break ties between ranks and preserve FIFO order
    shift_rank = pieo.lsh_use(rank.out, rank, 32)
    add_rank = pieo.add_store_in_reg(rank.out, cmd_idx.out, rank)

    ans = pieo.reg(32)
    err = pieo.reg(1)

    num_elements = pieo.reg(32)
    ans_index = pieo.reg(32)

    not_maxed = pieo.le_use(num_elements.out, queue_len)
    not_minned = pieo.ge_use(num_elements.out, 0)

    def push():
        """Pushes an element into the PIEO.
            Push into the queue based on rank.
        """

        #In parallel, push value, time and rank into their respective heaps
        return cb.if_with(not_maxed,
            cb.seq(
                shift_rank,
                add_rank,
                cb.par(
                    cb.invoke(
                        val_queue,
                        in_value=value.out,
                        in_rank=rank.out,
                        in_cmd=cb.const(2, 2),
                        ref_ans=ans,
                        ref_err=err,
                    ),
                    cb.invoke(
                        time_queue,
                        in_value=time.out,
                        in_rank=rank.out,
                        in_cmd=cb.const(2, 2),
                        ref_ans=ans,
                        ref_err=err,
                    ),
                    cb.invoke(
                        rank_queue,
                        in_value=time.out,
                        in_rank=rank.out,
                        in_cmd=cb.const(2, 2),
                        ref_ans=ans,
                        ref_err=err,
                    )
                ), pieo.incr(num_elements)
            )
        )

    def query(pop, include_value=False):
        """
            Query a PIEO by either popping or peeking, with either just a time predicate or both time predicate and value parameter.
            Parameter `pop` determines whether we pop or peek the PIEO. Parameter `include_value` determines whether we factor in 
            the value register as a parameter or not.

            Returns the first element in the PIEO who is 'ripe' as per the specified time (its readiness time is earlier than the passed in time),
            and whose value matches the current value (if that is to factored in.)
        """
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

        cached_vals = pieo.seq_mem_d1(f"cached_vals_{cmd_idx.out}", 32, queue_len, 32)
        cached_times = pieo.seq_mem_d1(f"cached_times_{cmd_idx.out}", 32, queue_len, 32)
        cached_ranks = pieo.seq_mem_d1(f"cached_ranks_{cmd_idx.out}", 32, queue_len, 32)

        #Equality checkers
        val_eq = pieo.eq(32)
        time_le = pieo.le(32)
        pop_lt = pieo.lt(32)

        while_and = pieo.and_(1)
        while_and_val = pieo.and_(1)
        val_time_and = pieo.and_(1)

        #Design all necessary loop guards
        with pieo.comb_group(f"time_pop_guard_{cmd_idx.out}") as time_pop_guard:
            
            val_eq.left = value.out
            val_eq.right = val_ans.out

            time_le.left = ready_time.out
            time_le.right = time.out

            pop_lt.left = queue_index.out
            pop_lt.right = num_elements.out

            while_and.left = time_le.out
            while_and.right = pop_lt.out

            while_and_val.left = while_and.out
            while_and_val.right = val_eq.out

            val_time_and.left = val_eq.out
            val_time_and.right = time_le.out


        replace_count_peek = pieo.lt_use(replace_tracker.out, queue_index.out) #Push back all popped elements
        replace_count_pop = pieo.lt_use(replace_tracker.out, queue_index.out) #Don't push back the latest popped element

        return cb.seq(
            #Scan through heaps and pop value, time and rank until we either run out or find an accurate one
            cb.while_with (
                cb.CellAndGroup(while_and_val, time_pop_guard) if include_value else cb.CellAndGroup(while_and, time_pop_guard),
                cb.seq([
                    cb.par([
                        cb.invoke(
                            q,
                            in_value=cb.const(32, 0),
                            in_rank=cb.const(32, 0),
                            in_cmd=cb.const(32, 0), #Pop from queue
                            ref_ans=ans,
                            ref_err=err)
                        for (q, ans) in ((val_queue, val_ans), (time_queue, ready_time), (rank_queue, rank_ans))
                    ]),
                    pieo.mem_store_d1(cached_vals, queue_index.out, val_ans.out, f"cache_vals_{cmd_idx.out}"),
                    pieo.mem_store_d1(cached_times, queue_index.out, ready_time.out, f"cache_times_{cmd_idx.out}"),
                    pieo.mem_store_d1(cached_ranks, queue_index.out, rank_ans.out, f"cache_ranks_{cmd_idx.out}"),
                    pieo.incr(queue_index)
                ])
            ),

            #If the correct element was found, write it to ans_mem
            cb.if_with(cb.CellAndGroup(val_time_and, time_pop_guard) if include_value else cb.CellAndGroup(time_le, time_pop_guard),
                cb.seq(
                    pieo.mem_store_d1(ans_mem, ans_index.out, val_ans.out, f"store_mem_{cmd_idx.out}"),
                    pieo.incr(ans_index)
                )
            ),

            #Write elements back – don't write back the last popped element if popping.
            cb.while_with(replace_count_pop if pop else replace_count_peek,
                cb.seq([
                    #At each iteration, load the cached elements into registers
                    cb.par([
                        pieo.mem_load_d1(cached_vals, replace_tracker.out, cached_val, f"load_cached_val_{cmd_idx.out}"),
                        pieo.mem_load_d1(cached_times, replace_tracker.out, cached_time, f"load_cached_time_{cmd_idx.out}"),
                        pieo.mem_load_d1(cached_ranks, replace_tracker.out, cached_rank, f"load_cached_rank_{cmd_idx.out}"),
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
            )
        )
    
    pieo.control += cb.while_with(
        cmd_in_range, cb.seq (
            cb.par(loads),
            cb.par (
                cb.if_with(cmd_eqs[0],
                    cb.if_with(not_minned,
                        query(pop=False)
                    )
                ),

                cb.if_with(cmd_eqs[1],
                    cb.if_with(not_minned,
                        query(pop=True)
                    )
                ),

                cb.if_with(cmd_eqs[2],
                    cb.if_with(not_maxed,
                        push()
                    )
                ),

                cb.if_with(cmd_eqs[3],
                    cb.if_with(not_maxed,
                        query(pop=False, include_value=True)
                    )
                ),

                cb.if_with(cmd_eqs[4],
                    cb.if_with(not_maxed,
                        query(pop=True, include_value=True)
                    )
                )
            ),
            incr_cmd_idx
        )
    )

def build():
    """Top-level function to build the program."""
    num_cmds = 20000
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