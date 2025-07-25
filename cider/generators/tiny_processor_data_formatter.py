from enum import Enum
from typing import List, Tuple


class Op(Enum):
    MUL = 0
    ADD = 1
    SUB = 2


def format_instruction(op: Op, left: int, right: int, dest: int) -> int:
    left = left & 0xFF
    right = right & 0xFF
    dest = dest & 0xFF

    out = left
    out = out | (right << 16)
    out = out | (dest << 32)
    out = out | (op.value << 48)

    (decoded_op, decoded_left, decoded_right, decoded_dest) = decode(out)
    assert decoded_op == op
    assert decoded_left == left
    assert decoded_right == right
    assert decoded_dest == dest

    return out


def decode(input: int) -> Tuple[Op, int, int, int]:
    left = input & 0xFF
    right = (input & (0xFF << 16)) >> 16
    dest = (input & (0xFF << 32)) >> 32
    op = input >> 48

    return (Op(op), left, right, dest)


def instruction_stream(input: List[Tuple[Op, int, int, int]]) -> List[int]:
    return [format_instruction(*args) for args in input]


instructions = [
    (Op.MUL, 0, 1, 30),
    (Op.ADD, 0, 1, 31),
    (Op.SUB, 2, 3, 32),
    (Op.MUL, 4, 5, 33),
    #
    (Op.MUL, 30, 5, 40),
    (Op.ADD, 15, 3, 41),
    (Op.SUB, 17, 2, 42),
    (Op.MUL, 20, 15, 43),
    #
    (Op.ADD, 30, 31, 30),
    (Op.ADD, 32, 33, 31),
    (Op.ADD, 15, 17, 37),
    (Op.ADD, 10, 10, 38),
    #
    (Op.MUL, 13, 15, 34),
    (Op.MUL, 13, 15, 34),
    (Op.SUB, 21, 22, 35),
    (Op.SUB, 23, 24, 36),
]


print(instruction_stream(instructions))
