# pylint: disable=import-error
import calyx.builder as cb


def insert_tuplify(prog, name, w1, w2):
    """Inserts the combinational component `tuplify` into the program.

    It takes two inputs, `fst` (width `w1`) and `snd` (width `w2`),
    and outputs a tuple (width `w1 + w2`) that contains `fst` and `snd`.
    """

    width = w1 + w2

    comp = prog.comb_component(name)
    fst = comp.input("fst", w1)
    snd = comp.input("snd", w2)
    comp.output("tup", width)

    or_ = comp.or_(width)
    lsh = comp.lsh(width)
    pad1 = comp.pad(w1, width)  # Pads `w1`-widthed items to width `width`
    pad2 = comp.pad(w2, width)  # Pads `w2`-widthed items to width `width`

    with comp.continuous:
        # Directly writing to the wires section.
        pad1.in_ = fst  # Pad `a` to the width of the tuple
        pad2.in_ = snd  # Pad `b` to the width of the tuple
        lsh.left = pad1.out
        lsh.right = cb.const(width, w2)  # Shift `a` to the left by `w2` bits
        or_.left = lsh.out
        or_.right = pad2.out  # Combine `a` and `b` into a single tuple
        comp.this().tup = or_.out  # Output the tuple

    return comp


def insert_untuplify(prog, name, w1, w2):
    """Inserts the component `untuplify` into the program.

    It takes a single input, `tup` (width w1+w2),
    and outputs two items, `fst` (width w1) and `snd` (width w2),
    that are extracted from the tuple.
    `fst` is the first `w1` bits of `tup`, and `snd` is the last `w2` bits.
    """

    width = w1 + w2

    comp = prog.comb_component(name)
    tup = comp.input("tup", width)
    comp.output("fst", w1)
    comp.output("snd", w2)

    slice1 = comp.bit_slice("slice_out_fst", width, w2, width - 1, w1)
    slice2 = comp.slice("slice_out_snd", width, w2)

    with comp.continuous:
        # Directly writing to the wires section.
        slice1.in_ = tup
        comp.this().fst = slice1.out
        slice2.in_ = tup
        comp.this().snd = slice2.out

    return comp


def insert_main(prog):
    """Inserts the main component into the program.
    Calls the `tuplify` component with 32-bit values 4 and 2.
    Writes the output to `mem1[0]`.
    Calls the `untuplify` component with the 64-bit value 17179869186
    to extract 32-bit values. Writes these to `mem2[0]` and `mem3[0]`.
    """
    comp = prog.component("main")
    tuplify = comp.cell("tuplify", insert_tuplify(prog, "tuplify", 32, 32))
    untuplify = comp.cell("untuplify", insert_untuplify(prog, "untuplify", 32, 32))

    mem1 = comp.seq_mem_d1("mem1", 64, 1, 1, is_external=True)
    mem2 = comp.seq_mem_d1("mem2", 32, 1, 1, is_external=True)
    mem3 = comp.seq_mem_d1("mem3", 32, 1, 1, is_external=True)

    with comp.group("run_tuplify") as run_tuplify:
        tuplify.fst = cb.const(32, 4)
        tuplify.snd = cb.const(32, 2)
        mem1.addr0 = cb.const(1, 0)
        mem1.write_en = cb.HI
        mem1.write_data = tuplify.tup
        mem1.content_en = cb.HI
        run_tuplify.done = mem1.done

    with comp.group("run_untuplify") as run_untuplify:
        untuplify.tup = cb.const(64, 17179869186)
        mem2.addr0 = cb.const(1, 0)
        mem2.write_en = cb.HI
        mem2.write_data = untuplify.fst
        mem2.content_en = cb.HI
        mem3.addr0 = cb.const(1, 0)
        mem3.write_en = cb.HI
        mem3.write_data = untuplify.snd
        mem3.content_en = cb.HI
        run_untuplify.done = (mem2.done & mem3.done) @ cb.HI

    comp.control += cb.par(run_tuplify, run_untuplify)

    return comp


def build():
    """Top-level function to build the program."""
    prog = cb.Builder()
    insert_main(prog)
    return prog.program


if __name__ == "__main__":
    build().emit()
