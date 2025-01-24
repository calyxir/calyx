# pylint: disable=import-error
import calyx.builder as cb
import queues.binheap.binheap as bh


def insert_main(prog):
    """Inserts the `main` component into the program.

    Invokes the `binheap` component with the following workload:
        push(9, 9),
        push(12, 12),
        push(6, 6),
        push(3, 3),
        pop(),
        push(8, 8),
        push(10, 10),
        pop(),
        pop(),
        pop(),
        pop(),
        pop(),
        push(3, 3),
        push(4, 4),
        push(5, 5),
        push(6, 6),
        push(10, 10),
        pop()
    """

    comp = prog.component("main")

    queue_size_factor = 4

    heap = bh.insert_binheap(prog, "heap", queue_size_factor, 64, 32)
    heap = comp.cell("heap", heap)

    out = comp.seq_mem_d1("out", 32, 15, queue_size_factor, is_external=True)

    ans = comp.reg(32)
    err = comp.reg(1)

    index = 0

    def push(value, rank):
        return cb.invoke(
            heap,
            in_value=cb.const(32, value),
            in_rank=cb.const(64, rank),
            in_cmd=cb.const(1, 1),
            ref_ans=ans,
            ref_err=err,
        )

    def pop_and_store():
        nonlocal index
        index += 1

        return [
            cb.invoke(
                heap,
                in_value=cb.const(32, 50),
                in_rank=cb.const(64, 50),
                in_cmd=cb.const(1, 0),
                ref_ans=ans,
                ref_err=err,
            ),
            comp.mem_store_d1(out, index - 1, ans.out, f"store_ans_{index}"),
        ]

    comp.control += [
        push(9, 9),
        push(12, 12),
        push(6, 6),
        push(3, 3),
        pop_and_store(),
        push(8, 8),
        push(10, 10),
        pop_and_store(),
        pop_and_store(),
        pop_and_store(),
        pop_and_store(),
        pop_and_store(),
        push(3, 3),
        push(4, 4),
        push(5, 5),
        push(6, 6),
        push(10, 10),
        pop_and_store(),
    ]


if __name__ == "__main__":
    """Invoke the top-level function to build the program."""
    prog = cb.Builder()
    insert_main(prog)
    prog.program.emit()
