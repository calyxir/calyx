from calyx.builder import HI, Builder, par


b = Builder()
b.import_("primitives/memories/seq.futil")
b.import_("primitives/binary_operators.futil")

array_size = 64
unroll_amount = array_size >> 1

comp = b.component("main")
i = comp.reg(32, "i")
input_array = comp.seq_mem_d1("input_array", 32, array_size, 32, True)


for idx in range(unroll_amount):
    comp.reg(32, f"reg_first__{idx}")
    comp.reg(32, f"reg_second__{idx}")
    comp.add(32, f"adder__{idx}")
    comp.add(32, f"second_adder__{idx}")
    comp.seq_mem_d1(f"lane_mem__{idx}", 32, array_size, 32)


for idx in range(unroll_amount):
    lane_mem = comp.get_cell(f"lane_mem__{idx}")
    reg = comp.get_cell(f"reg_first__{idx}")
    with comp.group(f"first_read__{idx}") as grp:
        lane_mem.addr0 = idx
        lane_mem.content_en = HI
        reg.write_en = lane_mem.done
        reg.in_ = lane_mem.read_data
        grp.done = reg.done


for idx in range(unroll_amount):
    lane_mem = comp.get_cell(f"lane_mem__{idx}")
    reg_second = comp.get_cell(f"reg_second__{idx}")
    reg = comp.get_cell(f"reg_first__{idx}")
    adder = comp.get_cell(f"adder__{idx}")

    with comp.group(f"write__{idx}") as grp:
        adder.left = reg.out
        adder.right = reg_second.out
        lane_mem.addr0 = idx
        lane_mem.write_en = HI
        lane_mem.content_en = HI
        lane_mem.write_data = adder.out
        grp.done = lane_mem.done

control_list = []

current_unroll = unroll_amount
while current_unroll > 0:
    for idx in range(current_unroll):
        lane_mem = comp.get_cell(f"lane_mem__{idx}")
        reg_second = comp.get_cell(f"reg_second__{idx}")
        adder = comp.get_cell(f"adder__{idx}")
        with comp.group(f"second_read__{current_unroll}__{idx}") as grp:
            lane_mem.addr0 = idx + current_unroll
            lane_mem.content_en = HI
            reg_second.write_en = lane_mem.done
            reg_second.in_ = lane_mem.read_data
            grp.done = reg_second.done

    control_list.append(
        par(
            *(
                [
                    comp.get_group(f"first_read__{idx}"),
                    comp.get_group(f"second_read__{current_unroll}__{idx}"),
                    comp.get_group(f"write__{idx}"),
                ]
                for idx in range(current_unroll)
            )
        )
    )
    current_unroll = current_unroll >> 1


comp.control += control_list


b.program.emit()
