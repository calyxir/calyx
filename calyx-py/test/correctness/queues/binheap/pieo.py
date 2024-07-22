# pylint: disable=import-error
import sys
import stable_binheap
import calyx.builder as cb
import calyx.queue_call as qc

FACTOR = 4

def insert_pieo(prog, name, queue_len, queue_len_factor=FACTOR, stats=None, static=False):
    pieo = prog.component(name)

    #Ref cells
    ans = pieo.reg(32, "ans", is_ref=True)
    err = pieo.reg(1, "err", is_ref=True)

    #Input commands passed in
    cmd = pieo.input("cmd", 3)
    value = pieo.input("value", 32)
    time = pieo.input("time", 32)
    rank = pieo.input("rank", 32)

    cmd_eqs = [pieo.eq_use(cmd, i) for i in range(5)]

    # Declare the sub-queues as cells of this component.
    val_queue = pieo.cell("val_queue", stable_binheap.insert_stable_binheap(prog, "val_queue", queue_len_factor))
    time_queue = pieo.cell("time_queue", stable_binheap.insert_stable_binheap(prog, "time_queue", queue_len_factor))
    rank_queue = pieo.cell("rank_queue", stable_binheap.insert_stable_binheap(prog, "rank_queue", queue_len_factor))

    #Registers/cells for ensuring no overflow or underflow
    num_elements = pieo.reg(32, "num_elements")
    overflow_check = pieo.lt_use(num_elements.out, queue_len)
    underflow_check = pieo.gt_use(num_elements.out, 0)

    #Querying register/memory components
    queue_index = pieo.reg(32, "queue_idx") #Tracker while scanning through heap
    replace_tracker = pieo.reg(32, "replace_tracker") #Loop counter while writing elements back into heap

    #Stores accessed times from popping queue
    ready_time = pieo.reg(32, "ready_time")
    val_ans = pieo.reg(32, "val_ans")
    rank_ans = pieo.reg(32, "rank_ans")

    #Equality checkers
    val_eq = pieo.eq(32)
    time_le = pieo.le(32)
    pop_lt = pieo.lt(32)

    while_and = pieo.and_(1)
    while_and_val = pieo.and_(1)
    val_time_and = pieo.and_(1)

    replace_count_peek = pieo.lt_use(replace_tracker.out, queue_index.out) #Push back all popped elements
    replace_count_pop = pieo.lt_use(replace_tracker.out, queue_index.out) #Don't push back the latest popped element

    #Store answer
    store_ans = pieo.reg_store(ans, val_ans.out, "store_ans")
    
    #Registers for individual cached value, time and rank
    cached_data_registers = [pieo.reg(32)] * 3

    #Memory cells for cached values, times and ranks
    cached_data = [pieo.seq_mem_d1(f"cached_{i}", 32, queue_len, 32) for i in range(3)]

    #Operations to cache values, times and ranks
    cache_data = [
        pieo.mem_store_d1(cached_data[0], queue_index.out, val_ans.out, f"cache_{0}"),
        pieo.mem_store_d1(cached_data[1], queue_index.out, ready_time.out, f"cache_{1}"),
        pieo.mem_store_d1(cached_data[2], queue_index.out, rank_ans.out, f"cache_{2}")
    ]

    #Load cached values, times and ranks
    load_cached_data = [
        pieo.mem_load_d1(cached_data[i], replace_tracker.out, cached_data_registers[i], f"load_cached{i}")
        for i in range(3)
    ]

    #Increment trackers
    incr_num_elements = pieo.incr(num_elements)
    incr_queue_idx = pieo.incr(queue_index)
    decr_num_elements = pieo.decr(num_elements)
    incr_replace_tracker = pieo.incr(replace_tracker)

    #Error and tracker resets
    raise_err = pieo.reg_store(err, cb.const(1, 1), "raise_err")
    reset_err = pieo.reg_store(err, cb.const(1, 0), "reset_err")
    reset_queue_idx = pieo.reg_store(queue_index, cb.const(32, 0), "reset_queue_idx")
    reset_replace_tracker = pieo.reg_store(replace_tracker, cb.const(32, 0), "reset_replace_tracker")

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

    #Functions for the necessary PIEO functionality
    
    def push():
        """
            Pushes an element into the PIEO.
            Push into the queue based on rank.
        """

        #In parallel, push value, time and rank into their respective heaps
        return cb.seq(
            cb.seq(
                cb.invoke(
                    val_queue,
                    in_value=value,
                    in_rank=rank,
                    in_cmd=cb.const(2, 2),
                    ref_ans=ans,
                    ref_err=err
                ),
                cb.invoke(
                    time_queue,
                    in_value=time,
                    in_rank=rank,
                    in_cmd=cb.const(2, 2),
                    ref_ans=ans,
                    ref_err=err
                ),
                cb.invoke(
                    rank_queue,
                    in_value=rank,
                    in_rank=rank,
                    in_cmd=cb.const(2, 2),
                    ref_ans=ans,
                    ref_err=err
                )
            ), incr_num_elements
        )

    def query(pop, include_value=False):
        """
            Query a PIEO by either popping or peeking, with either just a time predicate or both time predicate and value parameter.
            Parameter `pop` determines whether we pop or peek the PIEO. Parameter `include_value` determines whether we factor in 
            the value register as a parameter or not.

            Returns the first element in the PIEO who is 'ripe' as per the specified time (its readiness time is earlier than the passed in time),
            and whose value matches the current value (if that is to factored in.)
        """

        #Scan through heaps and pop value, time and rank until we either run out or find an accurate one
        return cb.seq(
            #Reset all trackers
            cb.par([reset_err, reset_queue_idx, reset_replace_tracker]),

            cb.while_with ( #Iterate while a suitable value has not yet been found
                cb.CellAndGroup(while_and_val, time_pop_guard)
                if include_value
                else cb.CellAndGroup(while_and, time_pop_guard),
                
                #Query each heap and pop the first value
                cb.seq([
                    cb.invoke(
                        val_queue,
                        in_value=cb.const(32, 0),
                        in_rank=cb.const(32, 0),
                        in_cmd=cb.const(2, 0), #Pop from queue
                        ref_ans=ans,
                        ref_err=err
                    ), pieo.reg_store(val_ans, ans.out),

                    cb.invoke(
                        time_queue,
                        in_value=cb.const(32, 0),
                        in_rank=cb.const(32, 0),
                        in_cmd=cb.const(2, 0), #Pop from queue
                        ref_ans=ans,
                        ref_err=err
                    ), pieo.reg_store(ready_time, ans.out),

                    cb.invoke(
                        rank_queue,
                        in_value=cb.const(32, 0),
                        in_rank=cb.const(32, 0),
                        in_cmd=cb.const(2, 0), #Pop from queue
                        ref_ans=ans,
                        ref_err=err
                    ), pieo.reg_store(rank_ans, ans.out)
                ] + cache_data + [incr_queue_idx])
            ),

            #If the correct element was found, write it to ans_mem
            cb.if_with((
                cb.CellAndGroup(val_time_and, time_pop_guard)
                if include_value
                else cb.CellAndGroup(time_le, time_pop_guard)),
                cb.seq([store_ans, raise_err] + [decr_num_elements] if pop else [])
                #Decrement number of elements if we are popping
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
                            in_value=cached_data_registers[0].out,
                            in_rank=cached_data_registers[2].out,
                            in_cmd=cb.const(2, 2), #Push back to memory
                            ref_ans=ans,
                            ref_err=err
                        ),

                        cb.invoke(
                            time_queue,
                            in_value=cached_data_registers[1].out,
                            in_rank=cached_data_registers[2].out,
                            in_cmd=cb.const(2, 2), #Push back to memory
                            ref_ans=ans,
                            ref_err=err
                        ),
                        cb.invoke(
                            rank_queue,
                            in_value=cached_data_registers[2].out,
                            in_rank=cached_data_registers[2].out,
                            in_cmd=cb.const(2, 2), #Push back to memory
                            ref_ans=ans,
                            ref_err=err,
                        )
                    ]), incr_replace_tracker
                ])
            )
        )

    pieo.control += cb.seq([
        reset_err,
        cb.par (
            #Peek with time predicate, if we have not minned out
            cb.if_with(cmd_eqs[0],
                cb.if_with(underflow_check,
                    query(pop=False),
                    raise_err
                )
            ),

            cb.if_with(cmd_eqs[1],
                cb.if_with(underflow_check,
                    query(pop=True),
                    raise_err
                )
            ),

            cb.if_with(cmd_eqs[2],
                cb.if_with(overflow_check,
                    push(),
                    raise_err
                )
            ),

            cb.if_with(cmd_eqs[3],
                cb.if_with(underflow_check,
                    query(pop=False, include_value=True),
                    raise_err
                )
            ),

            cb.if_with(cmd_eqs[4],
                cb.if_with(underflow_check,
                    query(pop=True, include_value=True),
                    raise_err
                )
            )
        )
    ])

    return pieo

def build():
    """Top-level function to build the program."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv
    prog = cb.Builder()
    pieo = insert_pieo(prog, "pieo", 16)
    qc.insert_main(prog, pieo, num_cmds, keepgoing=keepgoing, use_ranks=True, use_times=True)
    return prog.program


if __name__ == "__main__":
    build().emit()