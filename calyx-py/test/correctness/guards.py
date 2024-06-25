# pylint: disable=import-error
import calyx.builder as cb


def insert_main_component(prog):
    """Insert the main component into the program.
    This component will invoke the `muler`, `abs_diff`, `mux`, and `map` components.
    """

    comp = prog.component("main")

    mem = comp.seq_mem_d1("mem", 32, 1, 32, is_external=True)

    mul = comp.mult_pipe(32)
    zero = cb.const(32, 0)
    one = cb.const(32, 1)

    with comp.group("well-guarded_group") as wgg:
        mul.left = zero @ 4  # This will never be executed
        mul.left = (one <= zero) @ 5  # This will never be executed
        mul.left = ~zero @ 6  # This will work
        mul.right = (zero | one) @ 7  # This will work
        mul.right = (zero & one) @ 8  # This will never be executed
        mul.go = cb.HI
        wgg.done = mul.done

    put_ans_in_mem = comp.mem_store_d1(mem, 0, mul.out, "store")

    comp.control += [wgg, put_ans_in_mem]


def build():
    prog = cb.Builder()
    insert_main_component(prog)
    return prog.program


if __name__ == "__main__":
    build().emit()
