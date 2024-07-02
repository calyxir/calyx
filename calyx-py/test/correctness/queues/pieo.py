import sys
import calyx.builder as cb
import calyx.queue_call as qc
from calyx.utils import bits_needed

def insert_pieo(prog, max_cmds, queue_size):
    pieo = prog.component("main")

    #Memory dimensions
    queue_dim = (32, queue_size, 32, 32)
    input_dims = (32, max_cmds, 32, 32)

    #Memory read components
    commands = pieo.seq_mem_d1("commands", *input_dims, is_external=True)
    values = pieo.seq_mem_d1("values", *input_dims, is_external=True)
    times = pieo.seq_mem_d1("times", *input_dims, is_external=True)
    ranks = pieo.seq_mem_d1("ranks", *input_dims, is_external=True)
    ans_mem = pieo.seq_mem_d1("ans_mem", *input_dims, is_external=True)

    #Queue memory component – stores values, times, and ranks
    queue = pieo.seq_mem_d2("queue", *queue_dim, 3, is_external=False)

    #Queue size trackers
    current_size = pieo.reg(32, "queue_size")
    incr_queue_size = pieo.add_store_in_reg(current_size.out, 1, current_size, "incr_size")
    decr_queue_size = pieo.sub_store_in_reg(current_size.out, 1, current_size, "decr_size")
    not_maxed = pieo.neq_use(current_size, queue_size)
    not_minned = pieo.neq_use(current_size, 0)


    #Location trackers
    cmd_idx = pieo.reg(32, "command_idx")
    push_loc = pieo.reg(32, "push_loc")
    query_loc = pieo.reg(32, "query_loc")

    #Data trackers (reading from external memory)
    cmd, value, time, rank = pieo.reg(3, "cmd"), pieo.reg(32, "value"), pieo.reg(32, "time"), pieo.reg(32, "rank")

    #Data trackers (reading from queue)
    q_val, q_time, q_rank = pieo.reg(32, "q_val"), pieo.reg(32, "q_time"), pieo.reg(32, "q_rank")

    #Ensure that commands don't fall out of range
    command_tracker = pieo.lt_use(cmd_idx.out, max_cmds)

    #Guard to check that current queue element rank <= specified rank
    rank_checker = pieo.lt_use(q_rank.out, rank.out)

    #Guard to check that current queue element time <= specified time
    time_checker = pieo.lt_use(q_rank.out, rank.out)

    #Load command
    load_cmd = pieo.mem_load_d1(commands, cmd_idx.out, cmd, "load_cmd")

    #Increment command index
    incr_cmd_idx = pieo.add_store_in_reg(cmd_idx.out, 1, cmd_idx, "incr_cmd_idx")

    #CELLS FOR PUSHING

    pos_memories = [values, times, ranks]
    pos_registers = [value, time, rank]

    #Load values, times, rank from memory
    loads = [pieo.mem_store_d1(pos_memories[i], cmd_idx.out, pos_registers[i], f"load_{pos_registers[i]}") for i in range(3)]

    #Load values, times, rank from queue
    load_queue_data = [pieo.mem_load_d2(queue, push_loc, i, f"load_{pos_registers[i]}") for i in range(3)]

    #Store values, times, rank in memory
    stores = [pieo.mem_store_d2(queue, push_loc, i, pos_registers[i].out, f"push_{pos_registers[i]}") for i in range(3)]

    #Increment and reset location for pushing
    incr_push_loc = pieo.add_store_in_reg(push_loc.out, 1, push_loc, "incr_push_loc")
    reset_push_loc = pieo.reg_store(push_loc, 0)

    #Check the type of command
    cmd_eqs = [pieo.eq_use(cmd.out, i) for i in range(5)]

    pieo.control += cb.while_with(
        command_tracker, cb.seq(
            load_cmd,
            cb.par(
                cb.if_with(cmd_eqs[0], print("Peek by time")),
                cb.if_with(cmd_eqs[1], print("Pop by time")),

                cb.if_with(cmd_eqs[2],
                    cb.if_with(
                        not_maxed,
                        cb.seq(
                            cb.while_with(
                                rank_checker,

                            )
                        )
                    )
                ),

                cb.if_with(cmd_eqs[3], print("Peek by value")),
                cb.if_with(cmd_eqs[4], print("Pop by value"))
            ),
            incr_cmd_idx
        )
    )


def build():
    """Top-level function to build the program."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv
    prog = cb.Builder()
    fifo = insert_pieo(prog, "pieo")
    qc.insert_main(prog, fifo, num_cmds, keepgoing=keepgoing)
    return prog.program

if __name__ == "__main__":
    build().emit()
