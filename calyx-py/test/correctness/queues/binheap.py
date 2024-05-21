# pylint: disable=import-error
import calyx.builder as cb


def insert_tuplify(prog, name, w1, w2):
    """Inserts the component `tuplify` into the program.

    It takes two inputs, `a` (width w1) and `b` (width w2),
    and outputs a (w1+w2)-bit tuple that contains `a` and `b`.
    """

    width = w1 + w2

    comp = prog.component(name)
    a = comp.input("a", w1)
    b = comp.input("b", w2)
    comp.output("tup", width)

    or_ = comp.or_(width)
    lsh = comp.lsh(width)
    pad1 = comp.pad(w1, width)  # Pads `a`-widthed items to width `width`
    pad2 = comp.pad(w2, width)  # Pads `b`-widthed items to width `width`

    with comp.group("tup_group") as tup_group:
        pad1.in_ = a  # Pad `a` to the width of the tuple
        pad2.in_ = b  # Pad `b` to the width of the tuple
        lsh.left = pad1.out
        lsh.right = cb.const(width, w2)  # Shift `a` to the left by `w2` bits
        or_.left = lsh.out
        or_.right = pad2.out  # Combine `a` and `b` into a single tuple
        comp.this().tup = or_.out

    comp.control += tup_group

    return comp


def insert_untuplify(prog, name, w1, w2):
    """Inserts the component `untuplify` into the program.

    It takes a single input, `tup` (width w1+w2),
    and outputs two items, `a` (width w1) and `b` (width w2),
    that are extracted from the tuple.
    `a` is the first `w1` bits of `tup`, and `b` is the last `w2` bits.
    """

    width = w1 + w2

    comp = prog.component(name)
    tup = comp.input("tup", width)
    comp.output("a", w1)
    comp.output("b", w2)

    slice1 = comp.bit_slice(width, w2, width, w1)
    slice2 = comp.slice(width, w2)

    with comp.group("untup_group") as untup_group:
        slice1.in_ = tup
        comp.this().a = slice1.out
        slice2.in_ = tup
        comp.this().b = slice2.out

    comp.control += untup_group

    return comp


def insert_binheap(prog, name, tuplify, untuplify):
    """Inserts the component `binheap` into the program.

    It is a minimum binary heap, represented as an array.

    It follows the interface of the `pifo` component:
    It has three inputs:
    - `cmd`: tells us what operation to execute.
    The heap supports the operations `pop`, `peek`, and `push`.
    - `value`: the value to push to the queue.
    - `rank`: the rank with which to push the value.

    If an answer is expected, it is written to the `ans` register.
    If an error occurs, the `err` register is set to 1.
    """
    comp: cb.ComponentBuilder = prog.component(name)
    cmd = comp.input("cmd", 2)
    # If this is 0, we pop. If it is 1, we peek.
    # If it is 2, we push `value` to the queue.
    value = comp.input("value", 32)  # The value to push to the queue
    rank = comp.input("rank", 32)  # The rank with which to push the value

    ans = comp.reg(32, "ans", is_ref=True)
    # If the user wants to pop, we will write the popped value to `ans`.
    err = comp.reg(1, "err", is_ref=True)
    # We'll raise this as a general error flag for overflow and underflow.

    tuplify = comp.cell("tuplify", tuplify)
    untuplify = comp.cell("untuplify", untuplify)

    mem = comp.seq_mem_d1("mem", 64, 15, 4)
    # The memory to store the heap, represented as an array.
    # For now it has a hardcoded max length of 15, i.e., a binary heap of height 4.
    # Each cel of the memory is 64 bits wide:
    # the first 32 bits represent the value,
    # and the next 32 bits represent the rank.

    size = comp.reg(4)  # Active size, and also the next address to write to.

    add = comp.add(4)
    sub = comp.sub(64)
    mul = comp.mult_pipe(64)
    div = comp.div_pipe(64)

    parent = comp.reg(4)
    leftchild = comp.reg(4)

    with comp.group("find_parent") as find_parent:
        # Find the parent of the `leftchild`th element and store it in `parent`.
        # That is, parent := floor((leftchild − 1) / 2)
        sub.left = leftchild.out
        sub.right = 1
        sub.go = cb.HI
        div.left = sub.out
        div.right = 2
        div.go = sub.done
        parent.in_ = div.out
        parent.go = div.done
        find_parent.done = parent.done

    with comp.group("find_left_child") as find_leftchild:
        # Find the left child of the `parent`th element and store it in `leftchild`.
        # That is, leftchild := 2*`parent` + 1
        mul.left = parent.out
        mul.right = 2
        mul.go = cb.HI
        add.left = mul.out
        add.right = 1
        add.go = mul.done
        leftchild.in_ = add.out
        leftchild.go = add.done
        find_leftchild.done = leftchild.done

    return comp


def build():
    """Top-level function to build the program."""
    prog = cb.Builder()
    tuplify = insert_tuplify(prog, "tuplify", 32, 32)
    untuplify = insert_untuplify(prog, "untuplify", 32, 32)
    _ = insert_binheap(prog, "binheap", tuplify, untuplify)
    return prog.program


if __name__ == "__main__":
    build().emit()
