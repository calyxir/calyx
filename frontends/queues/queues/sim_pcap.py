# pylint: disable=import-error
import calyx.builder as cb
from calyx.utils import bits_needed
from calyx.tuple import insert_tuplify

ERR_CODE = 2**32 - 1
PUSH_CODE = 2**32 - 2


def insert_runner(prog, queue, name, num_cmds, num_flows):
    """Inserts the component `name` into the program.
    This will be used to `invoke` the component `queue` and feed it _one command_.
    """
    flow_bits = bits_needed(num_flows - 1)

    runner = prog.component(name)

    tuplify = insert_tuplify(prog, f"{name}_tuplify", flow_bits, 32 - flow_bits)
    tuplify = runner.cell("tuplify", tuplify)

    queue = runner.cell("myqueue", queue)
    # The user-facing interface of the `queue` component is assumed to be:
    # - input `cmd`
    #    where each command is a 2-bit unsigned integer, with the following format:
    #    `0`: pop
    #    `1`: push
    # - input `value`
    #   which is a 32-bit unsigned integer. If `cmd` is `1`, push this value.
    # - ref register `ans`, into which the result of a pop is written.
    # - ref register `err`, which is raised if an error occurs.

    commands = runner.seq_mem_d1("commands", 1, num_cmds, 32, is_ref=True)
    values = runner.seq_mem_d1("values", 32, num_cmds, 32, is_ref=True)
    arrival_cycles = runner.seq_mem_d1("arrival_cycles", 32, num_cmds, 32, is_ref=True)
    flows = runner.seq_mem_d1("flows", flow_bits, num_cmds, 32, is_ref=True)

    has_ans = runner.reg(1, "has_ans", is_ref=True)
    ans = runner.reg(32, "ans", is_ref=True)
    err = runner.reg(1, "err", is_ref=True)

    cycle_counter = runner.reg(32, "cycle_counter", is_ref=True)
    i = runner.reg(
        32, "i", is_ref=True
    )  # Index of the command we're currently processing
    try_again = runner.reg(
        1, "try_again", is_ref=True
    )  # Flag indicating if the `i`th packet has arrived

    cmd = runner.reg(1)
    value = runner.reg(32)
    arrival_cycle = runner.reg(32)
    flow = runner.reg(flow_bits)

    load_data = [
        runner.mem_load_d1(commands, i.out, cmd, "write_cmd"),
        runner.mem_load_d1(values, i.out, value, "write_value"),
        runner.mem_load_d1(arrival_cycles, i.out, arrival_cycle, "write_arrival_cycle"),
        runner.mem_load_d1(flows, i.out, flow, "write_flow"),
    ]

    slice = runner.slice("slice", 32, 32 - flow_bits)
    with runner.continuous:
        slice.in_ = value.out
        tuplify.fst = flow.out
        tuplify.snd = slice.out

    and_ = runner.and_(32)
    with runner.group("zero_out_top") as zero_out_top:
        and_.left = ans.out
        and_.right = cb.const(32, 2 ** (32 - flow_bits) - 1)
        ans.in_ = and_.out
        ans.write_en = cb.HI
        zero_out_top.done = ans.done

    runner.control += [
        load_data,
        cb.if_with(
            runner.ge_use(cycle_counter.out, arrival_cycle.out),
            [
                cb.invoke(
                    queue,
                    in_cmd=cmd.out,
                    in_value=tuplify.tup,
                    ref_ans=ans,
                    ref_err=err,
                ),
                cb.if_with(
                    runner.not_use(err.out),
                    [runner.eq_store_in_reg(cmd.out, 0, has_ans)[0], zero_out_top],
                ),
            ],
            runner.reg_store(try_again, 1),
        ),
    ]

    return runner


def insert_main(prog, queue, num_cmds, num_flows):
    flow_bits = bits_needed(num_flows - 1)

    main = prog.component("main")

    cycle_counter = main.reg(32, "cycle_counter")
    cycle_adder = main.add(32)
    with main.continuous:
        cycle_adder.left = cycle_counter.out
        cycle_adder.right = 1
        cycle_counter.in_ = cycle_adder.out
        cycle_counter.write_en = cb.HI

    dataplane = insert_runner(prog, queue, "dataplane", num_cmds, num_flows)
    dataplane = main.cell("dataplane", dataplane)

    has_ans = main.reg(1)
    ans = main.reg(32)
    err = main.reg(1)

    commands = main.seq_mem_d1("commands", 1, num_cmds, 32, is_external=True)
    values = main.seq_mem_d1("values", 32, num_cmds, 32, is_external=True)
    arrival_cycles = main.seq_mem_d1(
        "arrival_cycles", 32, num_cmds, 32, is_external=True
    )
    flows = main.seq_mem_d1("flows", flow_bits, num_cmds, 32, is_external=True)
    ans_mem = main.seq_mem_d1("ans_mem", 32, num_cmds, 32, is_external=True)
    departure_cycles = main.seq_mem_d1(
        "departure_cycles", 32, num_cmds, 32, is_external=True
    )

    i = main.reg(32, "i")
    try_again = main.reg(1, "try_again")

    # Lower the has-ans, err, and try_again flags
    lower_flags = [
        main.reg_store(has_ans, 0, "lower_has_ans"),
        main.reg_store(err, 0, "lower_err"),
        main.reg_store(try_again, 0, "lower_try_again"),
    ]

    main.control += cb.while_with(
        main.lt_use(i.out, num_cmds),
        [
            lower_flags,
            cb.invoke(
                dataplane,
                ref_commands=commands,
                ref_values=values,
                ref_arrival_cycles=arrival_cycles,
                ref_flows=flows,
                ref_has_ans=has_ans,
                ref_ans=ans,
                ref_err=err,
                ref_cycle_counter=cycle_counter,
                ref_i=i,
                ref_try_again=try_again,
            ),
            cb.if_(
                has_ans.out,
                [
                    main.mem_store_d1(ans_mem, i.out, ans.out, "write_ans"),
                    main.mem_store_d1(
                        departure_cycles, i.out, cycle_counter.out, "write_cycle"
                    ),
                ],
                cb.if_(
                    err.out,
                    main.mem_store_d1(
                        ans_mem,
                        i.out,
                        cb.const(32, ERR_CODE),
                        "write_err",
                    ),
                    main.mem_store_d1(
                        ans_mem,
                        i.out,
                        cb.const(32, PUSH_CODE),
                        "write_push",
                    ),
                ),
            ),
            cb.if_with(
                main.not_use(try_again.out), main.incr(i)
            ),  # i++ if try_again == 0
        ],
    )
