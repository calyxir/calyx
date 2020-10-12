import math
from dataclasses import dataclass

IdDictionary = {'cond': 0, 'std_const': 0, 'control': 0, 'group': 0, 'incr': 0, 'index': 0, 'incr': 0, 'let': 0,
                'seq': 0, 'std_add': 0, 'std_le': 0, 'std_mem_d1': 0, 'std_mem_d2': 0, 'std_mem_d3': 0, 'std_reg': 0}


def id(element):
    """
    Returns the next available id for an element type.
    This provides an identification system to produce unique variable names. While some of these are members of the
    standard library, others are commonly found names such as `cond` for branches, and `let` for loops.
    """
    assert (element in IdDictionary), 'Add this element to the id_dictionary.'
    id_number = IdDictionary[element]
    IdDictionary[element] += 1
    return id_number


@dataclass
class Register:
    bitwidth: int
    name: str
    primitive_type: str = 'std_reg'

    def __init__(self, bitwidth: int, name: str = 'reg'):
        if name != "reg":
            assert (name in IdDictionary), f'Named value `{name}` must be in the IdDictionary.'
            self.name = name + str(id(name))
        else:
            self.name = name + str(id('std_reg'))
        self.bitwidth = bitwidth

    def construct(self):
        return f'{self.name} = prim {self.primitive_type}({self.bitwidth});'

@dataclass
class Const:
    bitwidth: int
    value: int
    name: str
    primitive_type: str = 'std_const'

    def __init__(self, bitwidth: int, value: int, name: str = 'const'):
        if name != "const":
            assert (name in IdDictionary), f'Named value `{name}` must be in the IdDictionary.'
            self.name = name + str(id(name))
        else:
            self.name = name + str(id('std_const'))
        self.bitwidth = bitwidth
        self.value = value

    def construct(self):
        return f'{self.name} = prim {self.primitive_type}({self.bitwidth}, {self.value});'

@dataclass
class BinaryOp:
    bitwidth: int
    op: str
    primitive_type: str

    def __init__(self, bitwidth: int, op: str):
        op_id = "std_" + op
        assert (op_id in IdDictionary), f'Op value `{op_id}` must be in the IdDictionary.'
        self.name = op + str(id(op_id))
        self.bitwidth = bitwidth
        self.primitive_type = op_id
        self.op = op

    def construct(self):
        return f'{self.name} = prim {self.primitive_type}({self.bitwidth});'

@dataclass
class Tensor1D:
    bitwidth: int
    memory_size: int
    index_size: int
    name: str
    primitive_type: str = 'std_mem_d1'

    def __init__(self, bitwidth: int, memory_size: int, index_size: int, name: str = '1D_tensor'):
        if name != "1D_tensor":
            assert (name in IdDictionary), f'Named value `{name}` must be in the IdDictionary.'
            self.name = name + str(id(name))
        else:
            self.name = name + str(id('std_mem_d1'))
            self.bitwidth = bitwidth
            self.memory_size = memory_size
            self.index_size = index_size

    def construct(self):
        return f'{self.name} = prim {self.primitive_type}({self.bitwidth}, {self.memory_size}, {self.index_size});'


def ExtractTensorTypes(tensor_type):
    '''
    Extracts information from the tensor type.
    '''
    dimension = tensor_type.shape
    type = tensor_type.dtype
    bitwidth = int(''.join(filter(str.isdigit, type)))

    number_of_dimensions = len(dimension)
    assert(number_of_dimensions >= 0 and number_of_dimensions <= 3), "Dimensional count N must be 0 <= N <= 3"

    if number_of_dimensions == 0:
        # Scalar
        return 0, "", "", bitwidth

    elif number_of_dimensions == 2 and dimension[0] == 1:
        # 1-dimensional tensor
        dimensions = dimension[0]
        mem_size = dimension[1].__int__()
        mem_index_size = str(int(math.log2(mem_size)))

    elif number_of_dimensions == 2:
        # 2-dimensional tensor
        assert(False), "Unimplemented."
    else:
        assert(False), "Unimplemented."
        # 3-dimensional tensor

    return dimension[0], mem_size, mem_index_size, bitwidth


def ExtractBinaryArgumentTypes(a1, a2):
    """
    Extracts necessary information for binary arguments.
    """
    arg1_type = a1.checked_type
    arg2_type = a2.checked_type
    dimension_arg1, mem_size_arg1, mem_index_arg1, bitwidth1 = ExtractTensorTypes(arg1_type)
    dimension_arg2, mem_size_arg2, mem_index_arg2, bitwidth2 = ExtractTensorTypes(arg2_type)

    assert bitwidth1 == bitwidth2, f'The arguments for {call.op.name} have different bitwidths.'
    assert dimension_arg1 == dimension_arg2, f'The arguments for {call.op.name} have different dimensions.'
    assert mem_size_arg1 == mem_size_arg2, f'The arguments for {call.op.name} have different memory sizes.'
    assert mem_index_arg1 == mem_index_arg2, f'The arguments for {call.op.name} have different index sizes.'

    return dimension_arg1, mem_size_arg1, mem_index_arg1, bitwidth1
