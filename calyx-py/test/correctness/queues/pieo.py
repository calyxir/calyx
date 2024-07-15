import sys
import calyx.builder as cb
import calyx.queue_call as qc
from calyx.utils import bits_needed

def insert_find_push_loc(prog, name, length, rank):
    """Inserts component to find push location by rank"""

    comp = prog.component(name)

    mem = comp.seq_mem_d2("queue", *length, 3, is_ref=True)
    mid = comp.reg(32, "mid", is_ref=True)

    low, high = (
        comp.reg(32, "low"),
        comp.reg(32, "high"),
    )

    high_init = comp.reg_store(high, length)
    mid_element = comp.reg(32, "mid_elem")
    load_mem_mid = comp.mem_load_d2(mem, mid, 2, mid_element)
    
    mid_calc = [
        comp.sub_store_in_reg(high.out, low.out, mid),
        comp.div_store_in_reg(mid.out, 2, mid),
        comp.div_store_in_reg(low.out, mid.out, mid)
    ]

    #Guards
    length_lt, rank_neq, rank_gt, rank_lt = (
        comp.lt_use(low.out, high.out),
        comp.neq_use(rank.out, mid_element.out),
        comp.gt_use(rank.out, mid_element.out),
        comp.lt_use(rank.out, mid_element.out)
    )

    #Update indices after each iteration
    update_low = comp.add_store_in_reg(mid.out, 1, low)
    update_high = comp.reg_store(high, mid.out)

    comp.control += cb.seq (
        high_init,
        mid_calc + [
            load_mem_mid,
            cb.while_with (
                length_lt & rank_neq, cb.seq (
                    mid_calc + [
                        load_mem_mid,
                        cb.if_with(rank_gt, update_low),
                        cb.if_with(rank_lt, update_high)
                    ]
                )
            )
        ] + mid_calc
    )

    return comp
    
    
def insert_push_loc(prog, name, length, data, idx):
    """Component for pushing an element at specified location and shifting the remaining elements forward"""

    comp = prog.component(name)
    loc = comp.reg(32, "loc")
    prev_loc = comp.sub_use(loc.out, 1)
    dec = comp.decr(loc)

    mem = comp.seq_mem_d2("queue", *length, 3, is_ref=True)

    initialize_loc = comp.reg_store(loc, length-1)

    #Registers to hold values in previous index (to shift forward)
    prevs = [
        comp.reg(32, f"prev_{i}")
        for i in range(3)
    ]

    #Load values from previous index
    load_prevs = [
        comp.mem_load_d2(mem, prev_loc.out, i, prevs[i], f"load_prevs_{i}")
        for i in range(3)
    ]

    #Store previous value into current loc
    overwrite_currents = [
        comp.mem_store_d2(mem, loc, i, prevs[i], f"store_current_{i}")
        for i in range(3)
    ]

    #Zero out previous index
    zero_prevs = [
        comp.mem_store_d2(mem, prev_loc.out, i, 0, f"zero_prev_{i}")
        for i in range(3)
    ]

    #Insert data at final location
    insert_data = [
        comp.mem_store_d2(mem, loc, i, data[i].out, f"store_data_{i}")
        for i in range(3)
    ]

    tracker = comp.gt_use(loc.out, idx.out)

    comp.control += cb.seq (
        initialize_loc,
        cb.while_with (
            tracker, cb.seq(
                cb.par(load_prevs),
                cb.par(overwrite_currents),
                cb.par(zero_prevs),
                dec
            )
        ),
        insert_data
    )

    return comp


def insert_shift_backward(prog, name, length, idx):
    """Component for deleting an element by shifting all remaining elements backwards"""

    comp = prog.component(name)
    loc = comp.reg(32, "loc")
    next_loc = comp.add_use(loc.out, 1)
    inc = comp.incr(loc)

    mem = comp.seq_mem_d2("queue", *length, 3, is_ref=True)

    initialize_loc = comp.reg_store(loc, idx)

    #Registers to hold values in next index (to shift backward)
    next = [
        comp.reg(32, f"next_{i}")
        for i in range(3)
    ]

    #Load values from next index
    load_next = [
        comp.mem_load_d2(mem, next_loc.out, i, next[i], f"load_next_{i}")
        for i in range(3)
    ]

    #Store next value into current loc
    overwrite_currents = [
        comp.mem_store_d2(mem, loc, i, next[i], f"overwrite_currents_{i}")
        for i in range(3)
    ]

    #Zero out previous index
    zero_next = [
        comp.mem_store_d2(mem, next_loc.out, i, 0, f"zero_next_{i}")
        for i in range(3)
    ]

    tracker = comp.gt_use(loc.out, idx.out)

    comp.control += cb.seq (
        initialize_loc,
        cb.while_with (
            tracker, cb.seq(
                cb.par(load_next),
                cb.par(overwrite_currents),
                cb.par(zero_next),
                inc
            )
        )
    )

def query_time(prog, name, length, current_time):
    """Component for finding the lowest-rank element matching a time predicate"""
    comp = prog.component(name)

    mem = comp.seq_mem_d2("queue", *length, 3, is_ref=True)
    idx = comp.reg(32, is_ref=True)
    
    #Register loads in the readiness time at each level
    ready_time = comp.reg(32)
    inc = comp.incr(idx)

    found = comp.reg(1, is_ref=True)
    
    load_time = comp.mem_load_d2(mem, idx, 1, ready_time, "load_time")
    
    time_leq = comp.le_use(ready_time.out, current_time.out)
    idx_leq = comp.lt_use(idx.out, length)

    comp.control += cb.seq (
        load_time,
        cb.while_with(time_leq & idx_leq,
            cb.seq(
                inc,
                load_time
            )
        ), cb.if_with(time_leq, comp.bitwise_flip_reg(found))
    )


def query_value(prog, name, length, value):
    comp = prog.component(name)
    idx = comp.reg(32)
    found = comp.reg(32)
    inc = comp.incr(idx)
    elem_value = comp.reg(32)

    mem = comp.seq_mem_d2("queue", *length, 3, is_ref=True)
    idx = comp.reg(32, is_ref=True)
    found = comp.reg(1, is_ref=True)
    
    load_val = comp.mem_load_d2(mem, idx, 1, elem_value, "load_val")
    val_eq = comp.eq_use(elem_value.out, value.out)
    idx_leq = comp.lt_use(idx.out, length)

    comp.control += cb.seq (
        load_val,
        cb.while_with(val_eq & idx_leq,
            cb.seq(
                inc,
                load_val
            )
        ), cb.if_with(val_eq, comp.bitwise_flip_reg(found))
    )

    
def insert_pieo(prog, max_cmds, queue_size):
    pieo = prog.component("main")

    #Memory dimensions
    queue_dim = (32, queue_size, 32, 32)
    input_dims = (32, max_cmds, 32, 32)

    #Memory read components
    commands = pieo.seq_mem_d1("commands", *input_dims, True)
    values = pieo.seq_mem_d1("values", *input_dims, True)
    times = pieo.seq_mem_d1("times", *input_dims, True)
    ranks = pieo.seq_mem_d1("ranks", *input_dims, True)
    ans_mem = pieo.seq_mem_d1("ans_mem", *input_dims, True)

    #Queue memory component – stores values, times, and ranks
    queue = pieo.seq_mem_d2("queue", *queue_dim, 3, is_external=False)

    #Queue size trackers
    current_size = pieo.reg(32, "queue_size")
    not_maxed = pieo.neq_use(current_size.out, queue_size)
    not_minned = pieo.neq_use(current_size.out, 0)

    #Location trackers
    cmd_idx = pieo.reg(32, "command_idx")
    incr_cmd_idx = pieo.add_store_in_reg(cmd_idx.out, 1, cmd_idx, "incr_cmd_idx")
    cmd_in_range = pieo.lt_use(cmd_idx.out, max_cmds)

    #Data trackers (reading from external memory)
    cmd, value, time, rank = (
        pieo.reg(3, "cmd"),
        pieo.reg(32, "value"),
        pieo.reg(32, "time"),
        pieo.reg(32, "rank")
    )

    #Load Data
    load_cmd = pieo.mem_load_d1(commands, cmd_idx.out, cmd, "load_cmd")
    pos_memories = [values, times, ranks]
    pos_registers = [value, time, rank]

    loads = [
        pieo.mem_load_d1(
            pos_memories[i], 
            cmd_idx.out,
            pos_registers[i],
            f"load_{pos_registers[i]}")
        for i in range(3)
    ]

    #Reference registers
    ans_index = pieo.reg(32, "ans_index")
    insert_pos = pieo.reg(32)
    remove_pos = pieo.reg(32)
    result = pieo.reg(32)

    #Invoke cells
    find_push_loc = insert_find_push_loc(prog, "find_push_loc", queue_dim, rank)
    push_loc = insert_push_loc(prog, "push_loc", queue_dim, (value, time, rank), insert_pos)
    shift_backward = insert_shift_backward(prog, "shift_backward", )

    write = pieo.mem_store_d1(ans_mem, ans_index.out, result, "store_result")

    #Check the type of command
    cmd_eqs = [pieo.eq_use(cmd.out, i) for i in range(5)]

    pieo.control += cb.while_with(
        cmd_in_range, cb.seq (
            load_cmd,
            cb.par (
                cb.if_with(cmd_eqs[0] & not_minned,
                    cb.seq(

                    )
                ),

                cb.if_with(cmd_eqs[1] & not_minned,
                    print("Pop by time")
                ),

                cb.if_with(cmd_eqs[2] & not_maxed,
                    cb.seq(
                        cb.invoke(find_push_loc, ref_mem=queue, ref_mid=insert_pos),
                        cb.invoke(push_loc, ref_mem=queue),
                        pieo.incr(current_size)
                    )
                ),

                cb.if_with(cmd_eqs[3] & not_maxed,
                    print("Peek by value")
                ),

                cb.if_with(cmd_eqs[4] & not_maxed,
                    print("Pop by value")
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
    pieo = insert_pieo(prog, 20000, 16)
    qc.insert_main(prog, pieo, num_cmds, keepgoing=keepgoing)
    return prog.program

if __name__ == "__main__":
    build().emit()
