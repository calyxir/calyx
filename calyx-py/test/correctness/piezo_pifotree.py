# pylint: disable=import-error
import fifo
import pifo
import calyx.builder as cb
import calyx.queue_call as qc

MAX_CMDS = 15
ANS_MEM_LEN = 10


def insert_stats(prog, name):
    """Inserts a stats component called `name` into the program `prog`.

    It accepts, as input ports, two things:
    - a flag that indicates whether a _report_ is sought.
    - the index of a flow (0 or 1).

    It maintains two output ports, `count_0` and `count_1`.

    It also maintains two internal registers, `count_0_sto` and `count_1_sto`.

    If the `report` flag is set:
    The component doesn't change the stored counts, it just copies them over
    to the output ports.

    If the `report` flag is not set:
    The component reads the flow index and increments `count_0_sto` or `count_1_sto`.
    """

    stats: cb.ComponentBuilder = prog.component(name)
    report = stats.input(
        "report", 1
    )  # If 0, increment a counter. If 1, report the counts so far.
    flow = stats.input(
        "flow", 1
    )  # If 0, increment `count_0_sto`. If 1, increment `count_1_sto`.
    stats.output("count_0", 32)
    stats.output("count_1", 32)

    # Two registers to count the number of times we've been invoked with each flow.
    count_0_sto = stats.reg("count_0_sto", 32)
    count_1_sto = stats.reg("count_1_sto", 32)

    # Wiring to increment the appropriate register.
    count_0_incr = stats.incr(count_0_sto)
    count_1_incr = stats.incr(count_1_sto)

    # Equality checks.
    flow_eq_0 = stats.eq_use(flow, 0)
    flow_eq_1 = stats.eq_use(flow, 1)
    report_eq_0 = stats.eq_use(report, 0)

    with stats.continuous:
        stats.this().count_0 = count_0_sto.out
        stats.this().count_1 = count_1_sto.out

    # The main logic.
    stats.control += [
        cb.if_with(
            report_eq_0,  # Report is low, so the user wants to update the counts.
            cb.par(
                cb.if_with(flow_eq_0, [count_0_incr]),
                cb.if_with(flow_eq_1, [count_1_incr]),
            ),
        ),
    ]

    return stats


def insert_controller(prog, name, stats):
    """Inserts a controller component called `name` into the program `prog`.

    This component invokes the `stats` component, to which it has a handle,
    to retrieve its latest stats.

    The eventual goal is to have this happen _periodically_.
    For now, we just do it once.
    """

    controller = prog.component(name)
    stats = controller.cell("stats", stats)

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
        cb.invoke(
            stats,
            in_flow=cb.LO,  # Bogus.
            in_report=cb.HI,  # Yes, please give me a report.
        ),  # Invoke the stats component.
        get_data_locally,
        # Great, now I have the data around locally.
        # TODO: loop, with delay.
    ]

    return controller


def insert_main(prog, dataplane, controller):
    """Inserts the component `main` into the program.
    It triggers the dataplane and controller components.
    """

    main: cb.ComponentBuilder = prog.component("main")

    dataplane = main.cell("dataplane", dataplane)
    controller = main.cell("controller", controller)

    commands = main.seq_mem_d1("commands", 2, MAX_CMDS, 32, is_external=True)
    values = main.seq_mem_d1("values", 32, MAX_CMDS, 32, is_external=True)
    ans_mem = main.seq_mem_d1("ans_mem", 32, 10, 32, is_external=True)

    main.control += [
        cb.par(
            cb.invoke(  # Invoke the dataplane component.
                dataplane,
                ref_commands=commands,
                ref_values=values,
                ref_ans_mem=ans_mem,
            ),
            cb.invoke(controller),  # Invoke the controller component.
        )
    ]
    # In reality we need to write this in near-RTL:
    # group fake_par
    # {
    #     dataplane.my_fake_mem.
    #     dataplane.go = cb.HI
    #     controller.go = cb.HI
    #     fake_par.done = dataplane.done
    #     # NOTE: Conditioned on the dataplane being done, and not the controller.
    # }
    # BUT working in near-RTL means that need to do more:
    # We need to pass the memories "by reference" but cannot use the nice
    # abstraction of `invoke`.


def build():
    """Top-level function to build the program."""
    prog = cb.Builder()
    stats = insert_stats(prog, "stats")
    fifo_purple = fifo.insert_fifo(prog, "fifo_purple")
    fifo_tangerine = fifo.insert_fifo(prog, "fifo_tangerine")
    pifo_red = pifo.insert_pifo(prog, "pifo_red", fifo_purple, fifo_tangerine, 100)
    fifo_blue = fifo.insert_fifo(prog, "fifo_blue")
    pifo_root = pifo.insert_pifo(prog, "pifo_root", pifo_red, fifo_blue, 200, stats)
    # The root PIFO has a stats component.
    dataplane = qc.insert_main(prog, pifo_root, "dataplane")
    controller = insert_controller(prog, "controller", stats)
    insert_main(prog, dataplane, controller)
    return prog.program


if __name__ == "__main__":
    build().emit()
