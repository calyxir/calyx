# pylint: disable=import-error
import sys
import fifo
import strict_and_rr_queues.gen_strict_or_rr as strict_or_rr
import calyx.builder as cb
from calyx import queue_call


def insert_stats(prog, name, static=False):
    """Inserts a stats component called `name` into the program `prog`.

    It maintains:
    - One input port, the index of a flow (0 or 1).
    - Two output ports, `count_0` and `count_1`.

    It maintains two internal registers, `count_0_sto` and `count_1_sto`.

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
    count_0_sto = stats.reg(32)
    count_1_sto = stats.reg(32)

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

        stats.control += cb.if_with(flow_eq_0, count_0_incr, count_1_incr)

    # If static, we need to use continuous assignments and not comb groups.
    else:
        eq_cell = stats.eq(1, "eq_cell")

        with stats.continuous:
            stats.this().count_0 = count_0_sto.out
            stats.this().count_1 = count_1_sto.out
            eq_cell.left = flow
            eq_cell.right = 0

        stats.control += cb.static_if(eq_cell.out, count_0_incr, count_1_incr)

    return stats


def insert_controller(prog, name, stats_component):
    """Inserts a controller component called `name` into the program `prog`.

    This component receives, by reference, a `stats` component.
    It invokes the `stats` component to retrieve its latest stats.
    """

    controller = prog.component(name)
    stats = controller.cell("stats_controller", stats_component, is_ref=True)

    count_0 = controller.reg(32)
    count_1 = controller.reg(32)

    with controller.group("get_data_locally_count0") as get_data_locally_count0:
        count_0.in_ = stats.count_0
        count_0.write_en = 1
        get_data_locally_count0.done = count_0.done

    with controller.group("get_data_locally_count1") as get_data_locally_count1:
        count_1.in_ = stats.count_1
        count_1.write_en = 1
        get_data_locally_count1.done = count_1.done

    # The main logic.
    controller.control += cb.par(get_data_locally_count0, get_data_locally_count1)
    # Great, now I have the data around locally.

    return controller


def build(static=False):
    """Top-level function to build the program.
    The `static` flag determines whether the program is static or dynamic.
    """
    static = "--static" in sys.argv
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv

    prog = cb.Builder()
    stats_component = insert_stats(prog, "stats", static)
    controller = insert_controller(prog, "controller", stats_component)

    fifo_purple = fifo.insert_fifo(prog, "fifo_purple")
    fifo_tangerine = fifo.insert_fifo(prog, "fifo_tangerine")
    pifo_red = strict_or_rr.insert_queue(
        prog, "pifo_red", [fifo_purple, fifo_tangerine], [0, 100, 200], 2, [], True
    )
    fifo_blue = fifo.insert_fifo(prog, "fifo_blue")
    pifo_root = strict_or_rr.insert_queue(
        prog, "pifo_root", [pifo_red, fifo_blue], [0, 200, 400], 2, [], True
    )

    queue_call.insert_main(prog, pifo_root, num_cmds, keepgoing=keepgoing)
    return prog.program


# We will have a command line argument to determine whether the program is static.
if __name__ == "__main__":
    build().emit()
