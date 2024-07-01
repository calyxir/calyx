import calyx.builder as cb

def pieo_component(prog, max_cmds, queue_size):
    comp = prog.component("main")

    #Memory dimensions
    queue_dim = (32, queue_size, 32, 32)
    input_dims = (32, max_cmds, 32, 32)

    #Memory read components
    commands = comp.seq_mem_d1("commands", *input_dims, is_external=True)
    values = comp.seq_mem_d1("values", *input_dims, is_external=True)
    times = comp.seq_mem_d1("times", *input_dims, is_external=True)
    ranks = comp.seq_mem_d1("ranks", *input_dims, is_external=True)
    ans_mem = comp.seq_mem_d1("ans_mem", *input_dims, is_external=True)

    #Queue memory component – stores values, times, and ranks
    queue = comp.seq_mem_d2("queue", *queue_dim, 3, is_external=False)

    #Location trackers
    command_idx = comp.reg(32, "command_idx")
    push_loc = comp.reg(32, "push_loc")
    query_loc = comp.reg(32, "query_loc")

    #Data tracekrs
    cmd = comp.reg(32, "cmd")
    value = comp.reg(32, "value")
    time = comp.reg(32, "time")
    rank = comp.reg(32, "rank")

    command_tracker = comp.lt_use(command_idx.out, max_cmds)

    #CELLS FOR PUSHING
    load_time = comp.mem_load_d1(times, command_idx.out, time, "load_time")
    load_value = comp.mem_load_d1(values, command_idx.out, value, "load_value")
    load_rank = comp.mem_load_d1(rank, command_idx.out, rank, "load_rank")

    push_val = comp.mem_store_d2(queue, push_loc, 0, value.out, "push_val")
    push_time = comp.mem_store_d2(queue, push_loc, 1, time.out, "push_time")
    push_rank = comp.mem_store_d2(queue, push_loc, 2, rank.out, "push_rank")

