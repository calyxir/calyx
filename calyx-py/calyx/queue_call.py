# pylint: disable=import-error
from .py_ast import Empty
import calyx.builder as cb


def insert_runner(prog, queue, name, num_cmds, use_ranks, stats_component=None):
    """Inserts the component `name` into the program.
    This will be used to `invoke` the component `queue` and feed it _one command_.
    This component is designed to be invoked by some other component, and does not
    directly interface with external memories.

    The user-facing interface of this component is captured by a number
    of items that are passed to this component by reference.
    #
    - 1: `commands`, a list of commands.
       Where each command is a 2-bit unsigned integer with the following format:
       `0`: pop
       `1`: peek
       `2`: push
    - 2: `values`, a list of values.
       Where each value is a 32-bit unsigned integer.
       The value at `i` is pushed if the command at `i` is `2`.
    - 3: `has_ans`, a 1-bit unsigned integer.
       We raise/lower this to indicate whether the queue had a reply to the command.
    - 4: `component_ans`, a 32-bit unsigned integer.
       We put in this register the answer, if any.
    - 5: `component_err`, a 1-bit unsigned integer.
       We raise/lower it to indicate whether an error occurred.
    """
    assert (
        name != "main"
    ), "This method is not designed for the creation of `main`-style components."

    runner: cb.ComponentBuilder = prog.component(name)

    # We take a stats component by reference,
    # but all we'll really do with it is pass it to the queue component.
    stats_cell = (
        runner.cell("stats_runner", stats_component, is_ref=True)
        if stats_component
        else None
    )

    # We'll invoke the queue component.
    queue = runner.cell("myqueue", queue)
    # The user-facing interface of the `queue` component is assumed to be:
    # - input `cmd`
    #    where each command is a 2-bit unsigned integer, with the following format:
    #    `0`: pop
    #    `1`: peek
    #    `2`: push
    # - input `value`
    #   which is a 32-bit unsigned integer. If `cmd` is `2`, push this value.
    # - ref register `ans`, into which the result of a pop or peek is written.
    # - ref register `err`, which is raised if an error occurs.

    # Our memories and registers, all of which are passed to us by reference.
    commands = runner.seq_mem_d1("commands", 2, num_cmds, 32, is_ref=True)
    values = runner.seq_mem_d1("values", 32, num_cmds, 32, is_ref=True)
    ranks = (
        runner.seq_mem_d1("ranks", 32, num_cmds, 32, is_ref=True) if use_ranks else None
    )
    has_ans = runner.reg(1, "has_ans", is_ref=True)
    ans = runner.reg(32, "component_ans", is_ref=True)
    err = runner.reg(1, "component_err", is_ref=True)

    i = runner.reg(32)  # The index of the command we're currently processing
    cmd = runner.reg(2)  # The command we're currently processing
    value = runner.reg(32)  # The value we're currently processing
    rank = runner.reg(32)  # The rank we're currently processing

    load_data = (
        [  # `cmd := commands[i]`, `value := values[i]`, `rank := ranks[i]`
            runner.mem_load_d1(commands, i.out, cmd, "write_cmd"),
            runner.mem_load_d1(values, i.out, value, "write_value"),
            runner.mem_load_d1(ranks, i.out, rank, "write_rank"),
        ]
        if use_ranks
        else [  # `cmd := commands[i]`, `value := values[i]`
            runner.mem_load_d1(commands, i.out, cmd, "write_cmd"),
            runner.mem_load_d1(values, i.out, value, "write_value"),
        ]
    )

    runner.control += [
        load_data,
        (
            cb.invoke(  # Invoke the queue with a stats component.
                queue,
                in_cmd=cmd.out,
                in_value=value.out,
                ref_ans=ans,
                ref_err=err,
                ref_stats=stats_cell,
            )
            if stats_component
            else (
                cb.invoke(  # Invoke the queue with ranks.
                    queue,
                    in_cmd=cmd.out,
                    in_value=value.out,
                    in_rank=rank.out,
                    ref_ans=ans,
                    ref_err=err,
                )
                if use_ranks
                else cb.invoke(  # Invoke the queue without stats or ranks.
                    queue,
                    in_cmd=cmd.out,
                    in_value=value.out,
                    ref_ans=ans,
                    ref_err=err,
                )
            )
        ),
        # We're back from the invoke, and it's time for some post-mortem analysis.
        cb.if_with(
            runner.not_use(err.out),  # If there was no error
            [
                # If cmd <= 1, meaning cmd is pop or peek, raise the `has_ans` flag.
                # Otherwise, lower the `has_ans` flag.
                runner.le_store_in_reg(cmd.out, 1, has_ans)[0]
            ],
        ),
        runner.incr(i),  # i++
        runner.reg_store(err, 0, "lower_err"),  # Lower the error flag.
    ]

    return runner


def insert_main(
    prog,
    queue,
    num_cmds,
    keepgoing=False,
    use_ranks=False,
    controller=None,
    stats_component=None,
):
    """Inserts the component `main` into the program.
    It triggers the dataplane and controller components.
    """

    main: cb.ComponentBuilder = prog.component("main")

    stats = main.cell("stats_main", stats_component) if stats_component else None
    controller = main.cell("controller", controller) if controller else None
    dataplane = insert_runner(
        prog, queue, "dataplane", num_cmds, use_ranks, stats_component
    )
    dataplane = main.cell("dataplane", dataplane)

    has_ans = main.reg(1)
    dataplane_ans = main.reg(32)
    dataplane_err = main.reg(1)

    commands = main.seq_mem_d1("commands", 2, num_cmds, 32, is_external=True)
    values = main.seq_mem_d1("values", 32, num_cmds, 32, is_external=True)
    ans_mem = main.seq_mem_d1("ans_mem", 32, num_cmds, 32, is_external=True)
    ranks = (
        main.seq_mem_d1("ranks", 32, num_cmds, 32, is_external=True)
        if use_ranks
        else None
    )
    i = main.reg(32)  # A counter for how many times we have invoked the dataplane.
    j = main.reg(32)  # The index on the answer-list we'll write to
    keep_looping = main.and_(1)  # If this is high, we keep going. Otherwise, we stop.
    lt = main.lt(32)
    not_err = main.not_(1)

    with main.comb_group("Compute_keep_looping") as compute_keep_looping:
        # The condition to keep looping is:
        # The index `i` is less than the number of commands `num_cmds`
        # AND
        # If the `keepgoing` flag is _not_ set, the dataplane error is not high.
        lt.left = i.out
        lt.right = num_cmds
        not_err.in_ = dataplane_err.out
        keep_looping.left = lt.out
        keep_looping.right = cb.HI if keepgoing else not_err.out

    main.control += cb.while_with(
        # We will run the dataplane and controller components in sequence,
        # in a while loop. The loop will terminate when `break_` has a value of `1`.
        cb.CellAndGroup(keep_looping, compute_keep_looping),
        [
            main.reg_store(has_ans, 0, "lower_has_ans"),  # Lower the has-ans flag.
            (
                cb.invoke(
                    # Invoke the dataplane component with a stats component.
                    dataplane,
                    ref_commands=commands,
                    ref_values=values,
                    ref_has_ans=has_ans,
                    ref_component_ans=dataplane_ans,
                    ref_component_err=dataplane_err,
                    ref_stats_runner=stats,
                )
                if stats_component
                else (
                    cb.invoke(
                        # Invoke the dataplane component with ranks.
                        dataplane,
                        ref_commands=commands,
                        ref_values=values,
                        ref_ranks=ranks,
                        ref_has_ans=has_ans,
                        ref_component_ans=dataplane_ans,
                        ref_component_err=dataplane_err,
                    )
                    if use_ranks
                    else cb.invoke(
                        # Invoke the dataplane component without stats or ranks.
                        dataplane,
                        ref_commands=commands,
                        ref_values=values,
                        ref_has_ans=has_ans,
                        ref_component_ans=dataplane_ans,
                        ref_component_err=dataplane_err,
                    )
                )
            ),
            # If the dataplane component has an answer,
            # write it to the answer-list and increment the index `j`.
            cb.if_(
                has_ans.out,
                [
                    main.mem_store_d1(ans_mem, j.out, dataplane_ans.out, "write_ans"),
                    main.incr(j),
                ],
            ),
            (
                cb.invoke(  # Invoke the controller component.
                    controller,
                    ref_stats_controller=stats,
                )
                if controller
                else Empty
            ),
            main.incr(i),  # i++
        ],
    )
