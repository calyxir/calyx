from calyx.builder import HI, Builder, ComponentBuilder, par
from math import log2


def gen_iteration(
    comp: ComponentBuilder, idx: int, outer_iteration: int | None, shared_items: dict
):
    if outer_iteration is None:
        outer_string = ""
    else:
        outer_string = f"__{outer_iteration}"

    reg_2i = comp.reg(32, f"reg_2i{outer_string}__{idx}")
    reg_2ip1 = comp.reg(32, f"reg_2ip1{outer_string}__{idx}")
    adder = comp.add(32, f"adder{outer_string}__{idx}")
    second_adder = comp.add(32, f"second_adder{outer_string}__{idx}")
    lane_mem = comp.seq_mem_d1(
        f"lane_mem{outer_string}__{idx}", 32, shared_items["array_size"], 32
    )
    shared_items["mems"].append(lane_mem.name)
    mul = comp.mult_pipe(32, f"mul{outer_string}__{idx}")
    i = shared_items["i"]

    with comp.group(f"do_mul{outer_string}__{idx}") as do_mul:
        adder.left = i.out
        adder.right = idx
        mul.left = adder.out
        mul.right = 2
        mul.go = HI
        do_mul.done = mul.done

    with comp.group(f"first_read{outer_string}__{idx}") as first_read:
        lane_mem.addr0 = mul.out
        lane_mem.content_en = HI
        reg_2i.write_en = lane_mem.done
        reg_2i.in_ = lane_mem.read_data
        first_read.done = reg_2i.done

    with comp.group(f"second_read{outer_string}__{idx}") as second_read:
        adder.left = mul.out
        adder.right = 1
        lane_mem.addr0 = adder.out
        lane_mem.content_en = HI
        reg_2ip1.write_en = lane_mem.done
        reg_2ip1.in_ = lane_mem.done @ lane_mem.read_data
        second_read.done = reg_2ip1.done

    with comp.group(f"write{outer_string}__{idx}") as write:
        second_adder.left = i.out
        second_adder.right = idx
        adder.left = reg_2i.out
        adder.right = reg_2ip1.out
        lane_mem.addr0 = second_adder.out
        lane_mem.write_en = HI
        lane_mem.content_en = HI
        lane_mem.write_data = adder.out
        write.done = lane_mem.done

    return [do_mul, first_read, second_read, write]


def gen_unroll_only_inner(
    comp: ComponentBuilder, array_size: int, no_data_race: bool, shared_items: dict
):
    unroll_amount = array_size >> 1
    iterations = [
        gen_iteration(comp, x, None, shared_items) for x in range(unroll_amount)
    ]

    if no_data_race:
        write_groups = [x.pop() for x in iterations]

    control_list = []

    current_unroll = unroll_amount
    while current_unroll > 0:
        control_list.append(par(*(iterations[i] for i in range(current_unroll))))
        if no_data_race:
            control_list.append(par(*(write_groups[i] for i in range(current_unroll))))
        current_unroll = current_unroll >> 1

    return control_list


def gen_unroll_fully(
    comp: ComponentBuilder, array_size: int, outer_count: int, shared_items
):
    items = []
    for outer in range(outer_count):
        unroll_amount = array_size >> (outer + 1)
        for iteration in range(unroll_amount):
            items.append(gen_iteration(comp, iteration, outer, shared_items))

    return [par(*items)]


def make_tree_adder(array_size: int, full_unroll: bool):
    b = Builder()
    b.import_("primitives/memories/seq.futil")
    b.import_("primitives/binary_operators.futil")

    outer_loop_count = log2(array_size)
    assert outer_loop_count.is_integer(), "array size must be a power of two"
    outer_loop_count = int(outer_loop_count)

    comp = b.component("main")
    i = comp.reg(32, "i")
    comp.seq_mem_d1("input_array", 32, array_size, 32, True)
    shared_items = {"array_size": array_size, "i": i, "mems": ["input_array"]}

    if full_unroll:
        control_list = gen_unroll_fully(
            comp, array_size, outer_loop_count, shared_items
        )
    else:
        control_list = gen_unroll_only_inner(comp, array_size, True, shared_items)

    comp.control += control_list
    print(f"// --entangle '{','.join(shared_items['mems'])}'")
    b.program.emit()


def main():
    make_tree_adder(64, False)


if __name__ == "__main__":
    main()
