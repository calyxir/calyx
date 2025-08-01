from calyx.builder import HI, Builder, par, while_with

THREAD_COUNT = 4
MAIN_MEMORY_SIZE = 50

INSTRUCTION_COUNT = 16

INSTRUCTION_WIDTH = (3 * 16) + 8  # three 16 bit addresses and one 8 bit opcode
MEMORY_WIDTH = 32

NO_RW_DATA_RACE = False
# no WW race will also eliminate the RW race
MAKE_SEQ_WRITES = False

assert INSTRUCTION_COUNT % THREAD_COUNT == 0, (
    "Instruction count should be a multiple of the thread count"
)

b = Builder()
b.import_("primitives/memories/seq.futil")
b.import_("primitives/binary_operators.futil")

main = b.component("main")
main.seq_mem_d1("main_memory", MEMORY_WIDTH, MAIN_MEMORY_SIZE, 16, True)

main.seq_mem_d1(
    "instruction_memory", INSTRUCTION_WIDTH, INSTRUCTION_COUNT, MEMORY_WIDTH, True
)

instruction_pointer = main.reg(MEMORY_WIDTH, "instruction_pointer")


par_blocks = []
if NO_RW_DATA_RACE:
    decode_pars = []
if MAKE_SEQ_WRITES:
    write_blocks = []

for thread in range(THREAD_COUNT):
    lane_mem = main.seq_mem_d1(
        f"lane_{thread}_memory", MEMORY_WIDTH, MAIN_MEMORY_SIZE, 16
    )
    instr_mem = main.seq_mem_d1(
        f"lane_{thread}_instruction", INSTRUCTION_WIDTH, INSTRUCTION_COUNT, MEMORY_WIDTH
    )
    addr0 = main.reg(16, f"addr0__{thread}")
    addr1 = main.reg(16, f"addr1__{thread}")
    addr2 = main.reg(16, f"addr2__{thread}")
    op = main.reg(8, f"op__{thread}")

    addr0_slicer = main.bit_slice(
        f"addr0_slicer__{thread}", INSTRUCTION_WIDTH, 0, 15, 16
    )
    addr1_slicer = main.bit_slice(
        f"addr1_slicer__{thread}", INSTRUCTION_WIDTH, 16, 31, 16
    )
    addr2_slicer = main.bit_slice(
        f"addr2_slicer__{thread}", INSTRUCTION_WIDTH, MEMORY_WIDTH, 47, 16
    )
    op_slicer = main.bit_slice(f"op_slicer__{thread}", INSTRUCTION_WIDTH, 48, 55, 8)
    add = main.add(MEMORY_WIDTH)

    v1 = main.reg(MEMORY_WIDTH, f"v1__{thread}")
    v2 = main.reg(MEMORY_WIDTH, f"v2__{thread}")
    res = main.reg(MEMORY_WIDTH, f"result_{thread}")

    with main.group(f"decode__{thread}") as group:
        addr0_slicer.in_ = instr_mem.read_data
        addr1_slicer.in_ = instr_mem.read_data
        addr2_slicer.in_ = instr_mem.read_data
        op_slicer.in_ = instr_mem.read_data

        add.left = instruction_pointer.out
        add.right = thread

        instr_mem.addr0 = add.out
        instr_mem.content_en = HI

        addr0.in_ = addr0_slicer.out
        addr1.in_ = addr1_slicer.out
        addr2.in_ = addr2_slicer.out
        addr0.write_en = instr_mem.done
        addr1.write_en = instr_mem.done
        addr2.write_en = instr_mem.done

        op.in_ = op_slicer.out
        op.write_en = instr_mem.done
        group.done = op.done

    read_0 = main.mem_load_d1(lane_mem, addr0.out, v1, f"read_addr0__{thread}")
    read_1 = main.mem_load_d1(lane_mem, addr1.out, v2, f"read_addr1__{thread}")
    do_add = main.add_store_in_reg(v1.out, v2.out, res)[0]
    do_sub = main.sub_store_in_reg(v1.out, v2.out, res)[0]
    do_mul = main.mult_store_in_reg(v1.out, v2.out, res)[0]
    write_res = main.mem_store_d1(lane_mem, addr2.out, res.out, f"write_res_{thread}")

    prelude = [
        main.get_group(f"decode__{thread}"),
        read_0,
        read_1,
    ]

    compute_block = [
        main.case(
            op.out,
            {0: do_mul.as_enable(), 1: do_add.as_enable(), 2: do_sub.as_enable()},
        ),
    ]

    if NO_RW_DATA_RACE and MAKE_SEQ_WRITES:
        decode_pars.append(prelude)
        par_blocks.append(compute_block)
        write_blocks.append(write_res)
    elif NO_RW_DATA_RACE:
        decode_pars.append(prelude)
        par_blocks.append(compute_block + [write_res])
    elif MAKE_SEQ_WRITES:
        par_blocks.append(prelude + compute_block)
        write_blocks.append(write_res)
    else:
        par_blocks.append(prelude + compute_block + [write_res])


incr_instruction_pointer = main.incr(instruction_pointer, THREAD_COUNT)

while_body = [par(*par_blocks), incr_instruction_pointer]
if NO_RW_DATA_RACE:
    while_body.insert(0, par(*decode_pars))
if MAKE_SEQ_WRITES:
    while_body.append(write_blocks)

control = while_with(
    main.lt_use(instruction_pointer.out, INSTRUCTION_COUNT),
    while_body,
)

main.control += control


b.program.emit()
