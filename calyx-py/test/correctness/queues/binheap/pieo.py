# pylint: disable=import-error
import sys
import binheap
import calyx.builder as cb
import calyx.queue_call as qc

def insert_pieo(prog, name, val_queue, time_queue, rank_queue, queue_len, stats=None, static=False):
    pieo = prog.component(name)

    cmd_idx = pieo.reg(32)

    cmd = pieo.input("cmd", 3)
    value = pieo.input("value", 32)
    time = pieo.input("time", 32)
    rank = pieo.input("rank", 32)

    cmd_eqs = [pieo.eq_use(cmd, i) for i in range(5)]

    rank_reg = pieo.reg(32)

    store_rank = pieo.reg_store(rank_reg, rank)

    # Declare the sub-queues as cells of this component.
    val_queue = pieo.cell("val_queue", val_queue)
    time_queue = pieo.cell("time_queue", time_queue)
    rank_queue = pieo.cell("rank_queue", rank_queue)

    #Used to break ties between ranks and preserve FIFO order
    shift_rank = pieo.lsh_use(rank_reg.out, rank_reg, 32)
    add_rank, rank_reg = pieo.add_store_in_reg(rank_reg.out, cmd_idx.out, rank_reg)

    ans = pieo.reg(32)
    err = pieo.reg(1)

    num_elements = pieo.reg(32)

    not_maxed = pieo.le_use(num_elements.out, queue_len)
    not_minned = pieo.ge_use(num_elements.out, 0)

    #Querying register/memory components

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

    #Equality checkers
    val_eq = pieo.eq(32)
    time_le = pieo.le(32)
    pop_lt = pieo.lt(32)

    while_and = pieo.and_(1)
    while_and_val = pieo.and_(1)
    val_time_and = pieo.and_(1)

    replace_count_peek = pieo.lt_use(replace_tracker.out, queue_index.out) #Push back all popped elements
    replace_count_pop = pieo.lt_use(replace_tracker.out, queue_index.out) #Don't push back the latest popped element
    
    #Memory cells for cached values, times and ranks
    cached_data = [
        pieo.seq_mem_d1(f"cached_{i}", 32, queue_len, 32)
        for i in range(3)
    ]

    #Operations to cache values, times and ranks
    cache_data = [
        pieo.mem_store_d1(cached_data[i], queue_index.out, val_ans.out, f"cache_{i}")
        for i in range(3)
    ]

    #Load cached values, times and ranks
    load_cached_data = [
        pieo.mem_load_d1(cached_data[i], replace_tracker.out, cached_val, f"load_cached{i}")
        for i in range(3)
    ]

    #Increment trackers
    incr_num_elements = pieo.incr(num_elements)
    incr_queue_idx = pieo.incr(queue_index)
    decr_num_elements = pieo.decr(num_elements)
    incr_replace_tracker = pieo.incr(replace_tracker)

    #Functions for then necessary PIEO functionality
    
    def push():
        """
            Pushes an element into the PIEO.
            Push into the queue based on rank.
        """

        #In parallel, push value, time and rank into their respective heaps
        return cb.if_with(not_maxed,
            cb.seq(
                store_rank, shift_rank, add_rank,
                cb.par(
                    cb.invoke(
                        val_queue,
                        in_value=value,
                        in_rank=rank_reg.out,
                        in_cmd=cb.const(2, 2),
                        ref_ans=ans,
                        ref_err=err
                    ),
                    cb.invoke(
                        time_queue,
                        in_value=time,
                        in_rank=rank_reg.out,
                        in_cmd=cb.const(2, 2),
                        ref_ans=ans,
                        ref_err=err
                    ),
                    cb.invoke(
                        rank_queue,
                        in_value=rank_reg.out,
                        in_rank=rank_reg.out,
                        in_cmd=cb.const(2, 2),
                        ref_ans=ans,
                        ref_err=err
                    )
                ), incr_num_elements
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

        #Design all necessary loop guards
        with pieo.comb_group(f"time_pop_guard") as time_pop_guard:
            
            val_eq.left = val_ans.out
            val_eq.right = value

            time_le.left = ready_time.out
            time_le.right = time

            pop_lt.left = queue_index.out
            pop_lt.right = num_elements.out

            while_and.left = time_le.out
            while_and.right = pop_lt.out

            while_and_val.left = while_and.out
            while_and_val.right = val_eq.out

            val_time_and.left = val_eq.out
            val_time_and.right = time_le.out

        return cb.seq(
            #Scan through heaps and pop value, time and rank until we either run out or find an accurate one
            cb.par([
                pieo.reg_store(queue_index, 0),
                pieo.reg_store(replace_tracker, 0)
            ]),

            cb.while_with (
                cb.CellAndGroup(while_and_val, time_pop_guard)
                if include_value
                else cb.CellAndGroup(while_and, time_pop_guard),
                cb.seq([
                    cb.par([
                        cb.invoke(
                            q,
                            in_value=cb.const(32, 0),
                            in_rank=cb.const(32, 0),
                            in_cmd=cb.const(32, 0), #Pop from queue
                            ref_ans=ans,
                            ref_err=err)
                        for (q, ans) in
                        ((val_queue, val_ans), (time_queue, ready_time), (rank_queue, rank_ans))
                    ])
                ] + cache_data + [incr_queue_idx])
            ),

            #If the correct element was found, write it to ans_mem
            cb.if_with((
                cb.CellAndGroup(val_time_and, time_pop_guard)
                if include_value
                else cb.CellAndGroup(time_le, time_pop_guard)),
                cb.seq([
                    pieo.reg_store(ans, val_ans.out),
                    pieo.reg_store(err, cb.const(1, 1))
                ] + [decr_num_elements] if pop else []) #Decrement number of elements if we are popping
            ),

            #Write elements back – don't write back the last popped element if popping.
            cb.while_with(replace_count_pop if pop else replace_count_peek,
                cb.seq([
                    #At each iteration, load the cached elements into registers
                    cb.par(load_cached_data),

                    #Concurrently write them back into memory, using the cached rank to determine location
                    cb.par([
                        cb.invoke(
                            val_queue,
                            in_value=cached_val.out,
                            in_rank=cached_rank.out,
                            in_cmd=cb.const(2, 2),
                            ref_ans=ans,
                            ref_err=err
                        ),

                        cb.invoke(
                            time_queue,
                            in_value=cached_time.out,
                            in_rank=cached_rank.out,
                            in_cmd=cb.const(2, 2),
                            ref_ans=ans,
                            ref_err=err
                        ),
                        cb.invoke(
                            rank_queue,
                            in_value=cached_rank.out,
                            in_rank=cached_rank.out,
                            in_cmd=cb.const(2, 2),
                            ref_ans=ans,
                            ref_err=err,
                        )
                    ]), incr_replace_tracker
                ])
            )
        )

    pieo.control += cb.seq(
        cb.par (
            #Peek with time predicate, if we have not minned out
            cb.if_with(cmd_eqs[0],
                cb.if_with(not_minned,
                    query(pop=False),
                    pieo.reg_store(err, cb.const(1, 1))
                )
            ),

            cb.if_with(cmd_eqs[1],
                cb.if_with(not_minned,
                    query(pop=True),
                    pieo.reg_store(err, cb.const(1, 1))
                )
            ),

            cb.if_with(cmd_eqs[2],
                cb.if_with(not_maxed,
                    push(),
                    pieo.reg_store(err, cb.const(1, 1))
                )
            ),

            cb.if_with(cmd_eqs[3],
                cb.if_with(not_maxed,
                    query(pop=False, include_value=True),
                    pieo.reg_store(err, cb.const(1, 1))
                )
            ),

            cb.if_with(cmd_eqs[4],
                cb.if_with(not_maxed,
                    query(pop=True, include_value=True),
                    pieo.reg_store(err, cb.const(1, 1))
                )
            )
        ),
        pieo.incr(cmd_idx)
    )

    return pieo

def build():
    """Top-level function to build the program."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv
    prog = cb.Builder()
    val_queue = binheap.insert_binheap(prog, "val_queue", 4, 32, 32)
    time_queue = binheap.insert_binheap(prog, "time_queue", 4, 32, 32)
    rank_queue = binheap.insert_binheap(prog, "rank_queue", 4, 32, 32)
    pieo = insert_pieo(prog, "pieo", val_queue, time_queue, rank_queue, 16)
    qc.insert_main(prog, pieo, num_cmds, keepgoing=keepgoing, use_ranks=True, use_times=True)
    return prog.program


if __name__ == "__main__":
    build().emit()