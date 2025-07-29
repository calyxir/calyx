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
    comp.reg(32, f"reg_2i__{idx}")
    comp.reg(32, f"reg_2ip1__{idx}")
    comp.add(32, f"adder__{idx}")
    comp.add(32, f"second_adder__{idx}")
    comp.seq_mem_d1(f"lane_mem__{idx}", 32, array_size, 32)
    comp.mult_pipe(32, f"mul__{idx}")

for idx in range(unroll_amount):
    adder = comp.get_cell(f"adder__{idx}")
    mul = comp.get_cell(f"mul__{idx}")
    with comp.group(f"do_mul__{idx}") as grp:
        adder.left = i.out
        adder.right = idx
        mul.left = adder.out
        mul.right = 2
        mul.go = HI
        grp.done = mul.done

for idx in range(unroll_amount):
    lane_mem = comp.get_cell(f"lane_mem__{idx}")
    reg_2i = comp.get_cell(f"reg_2i__{idx}")
    mul = comp.get_cell(f"mul__{idx}")
    with comp.group(f"first_read__{idx}") as grp:
        lane_mem.addr0 = mul.out
        lane_mem.content_en = HI
        reg_2i.write_en = lane_mem.done
        reg_2i.in_ = lane_mem.read_data
        grp.done = reg_2i.done

for idx in range(unroll_amount):
    lane_mem = comp.get_cell(f"lane_mem__{idx}")
    reg_2ip1 = comp.get_cell(f"reg_2ip1__{idx}")
    mul = comp.get_cell(f"mul__{idx}")
    adder = comp.get_cell(f"adder__{idx}")
    with comp.group(f"second_read__{idx}") as grp:
        adder.left = mul.out
        adder.right = 1
        lane_mem.addr0 = adder.out
        lane_mem.content_en = HI
        reg_2ip1.write_en = lane_mem.done
        reg_2ip1.in_ = lane_mem.read_data
        grp.done = reg_2ip1.done

for idx in range(unroll_amount):
    lane_mem = comp.get_cell(f"lane_mem__{idx}")
    reg_2ip1 = comp.get_cell(f"reg_2ip1__{idx}")
    reg_2i = comp.get_cell(f"reg_2i__{idx}")
    adder = comp.get_cell(f"adder__{idx}")
    second_adder = comp.get_cell(f"second_adder__{idx}")

    with comp.group(f"write__{idx}") as grp:
        second_adder.left = i.out
        second_adder.right = idx
        adder.left = reg_2i.out
        adder.right = reg_2ip1.out
        lane_mem.addr0 = second_adder.out
        lane_mem.write_en = HI
        lane_mem.content_en = HI
        lane_mem.write_data = adder.out
        grp.done = lane_mem.done

control_list = []

current_unroll = unroll_amount
while current_unroll > 0:
    control_list.append(
        par(
            *(
                [
                    comp.get_group(f"do_mul__{idx}"),
                    comp.get_group(f"first_read__{idx}"),
                    comp.get_group(f"second_read__{idx}"),
                    comp.get_group(f"write__{idx}"),
                ]
                for idx in range(current_unroll)
            )
        )
    )
    # Uncomment the below and remove the write__{idx} line from the above par
    # control_list.append(
    #     par(*(comp.get_group(f"write__{idx}") for idx in range(current_unroll)))
    # )
    current_unroll = current_unroll >> 1


comp.control += control_list


b.program.emit()
