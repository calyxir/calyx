from typing import List
from calyx.py_ast import (
    Connect, CompVar, Cell, Group, ConstantPort, CompPort, Stdlib,
    Component, ThisPort, And, HolePort, Atom, Not, PortDef, SeqComp,
    Enable, While, ParComp, Structure, CompInst, Invoke, Program, Control, CombGroup
)


def generate_cells(width: int) -> List[Component]:
    '''
    Generates cells for the msb component.
    '''
    stdlib = Stdlib()
    return [Cell(CompVar("rsh"), stdlib.op("rsh", width, signed=False)),
            Cell(CompVar("counter"), Stdlib().register(width)),
            Cell(CompVar("cur_val"), Stdlib().register(width)),
            Cell(CompVar("add"), stdlib.op("add", width, signed=False)),
            Cell(CompVar("sub"), stdlib.op("sub", width, signed=False)),
            Cell(CompVar("neq"), stdlib.op("neq", width, signed=False)),
            Cell(CompVar("lsh"), stdlib.op("lsh", width, signed=False)),
            Cell(CompVar("count_ans"), Stdlib().register(width)),
            Cell(CompVar("val_ans"), Stdlib().register(width)),
            Cell(CompVar("val_build"), Stdlib().register(width)),
            ]


def generate_groups(width, int_width) -> List[Group]:
    '''
    Generates groups for the msb component
    '''
    wr_cur_val = Group(
        id=CompVar("wr_cur_val"),
        connections=[
            Connect(
                CompPort(CompVar("rsh"), "left"),
                ThisPort(CompVar("in")),
            ),
            Connect(
                CompPort(CompVar("rsh"), "right"),
                ConstantPort(width, int_width),
            ),
            Connect(
                CompPort(CompVar("cur_val"), "in"),
                CompPort(CompVar("rsh"), "out"),
            ),
            Connect(
                CompPort(CompVar("cur_val"), "write_en"),
                ConstantPort(1, 1),
            ),
            Connect(
                HolePort(CompVar("wr_cur_val"), "done"),
                CompPort(CompVar("cur_val"), "done"),
            )
        ])

    wr_val_build = Group(
        id=CompVar("wr_val_build"),
        connections=[
            Connect(
                CompPort(CompVar("val_build"), "in"),
                ConstantPort(32, 1),
            ),
            Connect(
                CompPort(CompVar("val_build"), "write_en"),
                ConstantPort(1, 1),
            ),
            Connect(
                HolePort(CompVar("wr_val_build"), "done"),
                CompPort(CompVar("val_build"), "done"),
            )
        ])

    cur_val_cond = CombGroup(
        id=CompVar("cur_val_cond"),
        connections=[
            Connect(CompPort(CompVar("neq"), "left"),
                    ConstantPort(width, 0)),
            Connect(CompPort(CompVar("neq"), "right"),
                    CompPort(CompVar("cur_val"), "out")),
        ],
    )

    count_cond = CombGroup(
        id=CompVar("count_cond"),
        connections=[
            Connect(CompPort(CompVar("neq"), "left"),
                    ConstantPort(width, 0)),
            Connect(CompPort(CompVar("neq"), "right"),
                    CompPort(CompVar("counter"), "out")),
        ],
    )

    incr_count = Group(
        id=CompVar("incr_count"),
        connections=[
            Connect(
                CompPort(CompVar("add"), "left"),
                CompPort(CompVar("counter"), "out"),
            ),
            Connect(
                CompPort(CompVar("add"), "right"),
                ConstantPort(width, 1),
            ),
            Connect(
                CompPort(CompVar("counter"), "in"),
                CompPort(CompVar("add"), "out"),
            ),
            Connect(
                CompPort(CompVar("counter"), "write_en"),
                ConstantPort(1, 1),
            ),
            Connect(
                HolePort(CompVar("incr_count"), "done"),
                CompPort(CompVar("counter"), "done"),
            )
        ]
    )

    shift_cur_val = Group(
        id=CompVar("shift_cur_val"),
        connections=[
            Connect(
                CompPort(CompVar("rsh"), "left"),
                CompPort(CompVar("cur_val"), "out"),
            ),
            Connect(
                CompPort(CompVar("rsh"), "right"),
                ConstantPort(width, 1),
            ),
            Connect(
                CompPort(CompVar("cur_val"), "in"),
                CompPort(CompVar("rsh"), "out"),
            ),
            Connect(
                CompPort(CompVar("cur_val"), "write_en"),
                ConstantPort(1, 1),
            ),
            Connect(
                HolePort(CompVar("shift_cur_val"), "done"),
                CompPort(CompVar("cur_val"), "done"),
            )
        ]
    )

    shift_val_build = Group(
        id=CompVar("shift_val_build"),
        connections=[
            Connect(
                CompPort(CompVar("lsh"), "left"),
                CompPort(CompVar("val_build"), "out"),
            ),
            Connect(
                CompPort(CompVar("lsh"), "right"),
                ConstantPort(width, 1),
            ),
            Connect(
                CompPort(CompVar("val_build"), "in"),
                CompPort(CompVar("lsh"), "out"),
            ),
            Connect(
                CompPort(CompVar("val_build"), "write_en"),
                ConstantPort(1, 1),
            ),
            Connect(
                HolePort(CompVar("shift_val_build"), "done"),
                CompPort(CompVar("val_build"), "done"),
            )
        ]
    )

    decr_count = Group(
        id=CompVar("decr_count"),
        connections=[
            Connect(
                CompPort(CompVar("sub"), "left"),
                CompPort(CompVar("counter"), "out"),
            ),
            Connect(
                CompPort(CompVar("sub"), "right"),
                ConstantPort(width, 1),
            ),
            Connect(
                CompPort(CompVar("counter"), "in"),
                CompPort(CompVar("sub"), "out"),
            ),
            Connect(
                CompPort(CompVar("counter"), "write_en"),
                ConstantPort(1, 1),
            ),
            Connect(
                HolePort(CompVar("decr_count"), "done"),
                CompPort(CompVar("counter"), "done"),
            )
        ]
    )

    wr_count = Group(
        id=CompVar("wr_count"),
        connections=[
            Connect(
                CompPort(CompVar("lsh"), "left"),
                CompPort(CompVar("counter"), "out"),
            ),
            Connect(
                CompPort(CompVar("lsh"), "right"),
                ConstantPort(width, width-int_width)
            ),
            Connect(
                CompPort(CompVar("count_ans"), "in"),
                CompPort(CompVar("lsh"), "out"),
            ),
            Connect(
                CompPort(CompVar("count_ans"), "write_en"),
                ConstantPort(1, 1),
            ),
            Connect(
                HolePort(CompVar("wr_count"), "done"),
                CompPort(CompVar("count_ans"), "done"),
            )
        ]
    )

    wr_val = Group(
        id=CompVar("wr_val"),
        connections=[
            Connect(
                CompPort(CompVar("lsh"), "left"),
                CompPort(CompVar("val_build"), "out"),
            ),
            Connect(
                CompPort(CompVar("lsh"), "right"),
                ConstantPort(width, width-int_width)
            ),
            Connect(
                CompPort(CompVar("val_ans"), "in"),
                CompPort(CompVar("lsh"), "out"),
            ),
            Connect(
                CompPort(CompVar("val_ans"), "write_en"),
                ConstantPort(1, 1),
            ),
            Connect(
                HolePort(CompVar("wr_val"), "done"),
                CompPort(CompVar("val_ans"), "done"),
            )
        ]
    )

    return [wr_cur_val, wr_val_build, cur_val_cond, count_cond, incr_count,
            shift_cur_val, shift_val_build, decr_count, wr_count, wr_val]


def generate_control() -> Control:
    '''
    Generates control for the msb component. 
    '''
    return SeqComp([
        Enable("wr_cur_val"),
        While(CompPort(CompVar("neq"), "out"), CompVar("cur_val_cond"),
              SeqComp([Enable("incr_count"), Enable("shift_cur_val")])),
        Enable("decr_count"),
        Enable("wr_count"),
        Enable("wr_val_build"),
        While(CompPort(CompVar("neq"), "out"), CompVar("count_cond"),
              SeqComp([Enable("decr_count"), Enable("shift_val_build")])),
        Enable("wr_val"),
    ])


def gen_msb_calc(
    width: int, int_width: int
) -> List[Component]:
    """
    Generates Calyx component to produce the following:
    For a given fixed point number x>=1, we want to select the largest n such
    that 2^n <= x. Note that this is essentially finding the index of the most
    significant bit in x. count_ans is the value for `n`, and `value_ans` is the
    value for `2^n`.
    Essentially, the component uses a while loop, a counter register, and shifts the input 
    1 bit to the right at each iteration until it equals 0. 
    Important note: this component doesn't work when the input is 0.  
    """
    return [Component(
        "msb_calc",
        inputs=[PortDef(CompVar("in"), width)],
        outputs=[PortDef(CompVar("count"), width), PortDef(CompVar("value"), width)],
        structs=generate_cells(width)
        + generate_groups(width, int_width) + [Connect(ThisPort(CompVar("count")),
                                                       CompPort(CompVar("count_ans"), "out")),
                                               Connect(ThisPort(CompVar("value")),
                                                       CompPort(CompVar("val_ans"), "out"))
                                               ],
        controls=generate_control(),
    )]
