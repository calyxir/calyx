# pylint: disable=import-error
import fifo
import pifo
import calyx.builder as cb
from calyx import queue_call
from calyx import queue_util


def insert_stats(prog, name, static=False):
    """Inserts a stats component called `name` into the program `prog`.

    It maintains:
    - One input port, the index of a flow (0 or 1).
    - Two output ports, `count_0` and `count_1`.

    It also maintains two internal registers, `count_0_sto` and `count_1_sto`.

    The component continously outputs the values of the two registers into the
    two output ports.

    When invoked, the component reads the flow index and increments
    `count_0_sto` or `count_1_sto` as appropriate.

    If `static` is False, this is a dynamic component.
    Otherwise, it is a static component with delay 1.
    """

    stats: cb.ComponentBuilder = prog.component(name, latency=1 if static else None)

    flow = stats.input("flow", 1)
    stats.output("count_0", 32)
    stats.output("count_1", 32)

    # Two registers to count the number of times we've been invoked with each flow.
    count_0_sto = stats.reg("count_0_sto", 32)
    count_1_sto = stats.reg("count_1_sto", 32)

    # Wiring to increment the appropriate register.
    count_0_incr = stats.incr(count_0_sto, static=static)
    count_1_incr = stats.incr(count_1_sto, static=static)

    # The rest of the logic varies depending on whether the component is static.

    # If not static, we can use comb groups.
    if not static:
        flow_eq_0 = stats.eq_use(flow, 0)

        with stats.continuous:
            stats.this().count_0 = count_0_sto.out
            stats.this().count_1 = count_1_sto.out

        stats.control += cb.par(
            cb.if_with(flow_eq_0, count_0_incr, count_1_incr),
        )

    # If static, we need to use continuous assignments and not comb groups.
    else:
        eq_cell = stats.eq(1, "eq_cell")

        with stats.continuous:
            stats.this().count_0 = count_0_sto.out
            stats.this().count_1 = count_1_sto.out
            eq_cell.left = flow
            eq_cell.right = 0

        stats.control += cb.static_par(
            cb.static_if(eq_cell.out, count_0_incr, count_1_incr),
        )

    return stats


def insert_controller(prog, name, stats_component):
    """Inserts a controller component called `name` into the program `prog`.

    This component receives, by reference, a `stats` component.
    It invokes the `stats` component to retrieve its latest stats.
    """

    controller = prog.component(name)
    stats = controller.cell("stats_controller", stats_component, is_ref=True)

    count_0 = controller.reg("count_0", 32)
    count_1 = controller.reg("count_1", 32)

    with controller.group("get_data_locally") as get_data_locally:
        count_0.in_ = stats.count_0
        count_0.write_en = 1
        count_1.in_ = stats.count_1
        count_1.write_en = 1
        get_data_locally.done = (count_0.done & count_1.done) @ 1

    # The main logic.
    controller.control += [
        get_data_locally,
        # Great, now I have the data around locally.
    ]

    return controller


def insert_main(prog, dataplane, controller, stats_component):
    """Inserts the component `main` into the program.
    It triggers the dataplane and controller components.
    """

    main: cb.ComponentBuilder = prog.component("main")

    stats = main.cell("stats_main", stats_component)
    dataplane = main.cell("dataplane", dataplane)
    controller = main.cell("controller", controller)

    has_ans = main.reg("has_ans", 1)
    dataplane_ans = main.reg("dataplane_ans", 32)
    dataplane_err = main.reg("dataplane_err", 1)

    commands = main.seq_mem_d1("commands", 2, queue_util.MAX_CMDS, 32, is_external=True)
    values = main.seq_mem_d1("values", 32, queue_util.MAX_CMDS, 32, is_external=True)
    ans_mem = main.seq_mem_d1("ans_mem", 32, queue_util.MAX_CMDS, 32, is_external=True)

    j = main.reg("j", 32)  # The index on the answer-list we'll write to
    incr_j = main.incr(j)  # j++
    write_ans = main.mem_store_seq_d1(ans_mem, j.out, dataplane_ans.out, "write_ans")
    # ans_mem[j] = dataplane_ans
    lower_has_ans = main.reg_store(has_ans, 0, "lower_has_ans")  # has_ans := 0

    not_err = main.not_use(dataplane_err.out)

    main.control += [
        # We will run the dataplane and controller components in parallel,
        # in a while loop. The loop will terminate when the dataplane component
        # raises `dataplane_err`.
        cb.while_with(
            not_err,  # While the dataplane component has not errored out.
            [
                lower_has_ans,  # Lower the has-ans flag.
                cb.invoke(  # Invoke the dataplane component.
                    dataplane,
                    ref_commands=commands,
                    ref_values=values,
                    ref_has_ans=has_ans,
                    ref_component_ans=dataplane_ans,
                    ref_component_err=dataplane_err,
                    ref_stats_runner=stats,
                ),
                # If the dataplane component has an answer,
                # write it to the answer-list and increment the index `j`.
                cb.if_(has_ans.out, [write_ans, incr_j]),
                cb.invoke(  # Invoke the controller component.
                    controller,
                    ref_stats_controller=stats,
                ),
            ],
        )
    ]


def build(static=False):
    """Top-level function to build the program.
    The `static` flag determines whether the program is static or dynamic.
    """
    prog = cb.Builder()
    stats_component = insert_stats(prog, "stats", static)
    fifo_purple = fifo.insert_fifo(prog, "fifo_purple")
    fifo_tangerine = fifo.insert_fifo(prog, "fifo_tangerine")
    pifo_red = pifo.insert_pifo(prog, "pifo_red", fifo_purple, fifo_tangerine, 100)
    fifo_blue = fifo.insert_fifo(prog, "fifo_blue")
    pifo_root = pifo.insert_pifo(
        prog, "pifo_root", pifo_red, fifo_blue, 200, stats_component, static
    )
    # The root PIFO will take a stats component by reference.
    dataplane = queue_call.insert_runner(prog, pifo_root, "dataplane", stats_component)
    controller = insert_controller(prog, "controller", stats_component)
    insert_main(prog, dataplane, controller, stats_component)
    return prog.program


if __name__ == "__main__":
    build().emit()
