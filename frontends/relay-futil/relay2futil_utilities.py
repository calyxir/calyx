import math
from dataclasses import dataclass
from collections import defaultdict

IdDictionary = defaultdict(int)


def id(element):
    """
    Returns the next available id for an element type.
    This provides an identification system to produce unique variable names. While some of these are members of the
    standard library, others are commonly found names such as `cond` for branches, and `let` for loops.
    """
    id_number = IdDictionary[element]
    IdDictionary[element] += 1
    return id_number


@dataclass
class Register:
    bitwidth: int
    name: str
    primitive_type: str = 'std_reg'

    def __init__(self, bitwidth: int, name: str = 'reg', is_function_argument: bool = False):
        self.name = name if is_function_argument else name + str(id(name))
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
        self.name = name + str(id(name))
        self.bitwidth = bitwidth
        self.value = value

    def construct(self):
        return f'{self.name} = prim {self.primitive_type}({self.bitwidth}, {self.value});'

@dataclass
class Slice:
    bitwidth: int
    value: int
    name: str
    primitive_type: str = 'std_slice'

    def __init__(self, bitwidth: int, value: int, name: str = 'slice'):
        self.name = name + str(id(name))
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
    index_bitwidth: int
    name: str
    primitive_type: str = 'std_mem_d1'

    def __init__(self, bitwidth: int, memory_size: int, index_bitwidth: int, name: str = 'tensor1D_',
                 is_function_argument: bool = False):
        self.name = name if is_function_argument else name + str(id(name))
        self.bitwidth = bitwidth
        self.memory_size = memory_size
        self.index_bitwidth = index_bitwidth

    def construct(self):
        return f'{self.name} = prim {self.primitive_type}({self.bitwidth}, {self.memory_size}, {self.index_bitwidth});'


@dataclass
class Tensor2D:
    bitwidth: int
    memory_sizes: (int, int)
    index_bitwidths: (int, int)
    name: str
    primitive_type: str = 'std_mem_d2'

    def __init__(self, bitwidth: int, memory_sizes: (int, int), index_bitwidths: (int, int), name: str = 'tensor2D_',
                 is_function_argument: bool = False):
        self.name = name if is_function_argument else name + str(id(name))
        self.bitwidth = bitwidth
        self.memory_sizes = memory_sizes
        self.index_bitwidths = index_bitwidths

    def construct(self):
        return f'{self.name} = prim {self.primitive_type}({self.bitwidth}, {self.memory_sizes[0]},' \
               f' {self.memory_sizes[1]}, {self.index_bitwidths[0]}, {self.index_bitwidths[1]});'


def ExtractTensorTypes(tensor_type):
    '''
    Extracts information from the tensor type.
    dimensions: The number of dimensions in the tensor. This must be N where 0 <= N <= 3.
    bitwidth: The bitwidth of the values in the tensor.
    memory_size: The number of elements in the tensor.
    memory_index_bitwidth: The bitwidth of the index used to increment the size.
                           This will be equivalent to log2(memory_size).
    '''
    dimension = tensor_type.shape
    type = tensor_type.dtype
    bitwidth = int(''.join(filter(str.isdigit, type)))

    number_of_dimensions = len(dimension)
    assert (number_of_dimensions >= 0 and number_of_dimensions <= 3), "Dimensional count N must be 0 <= N <= 3"

    if number_of_dimensions == 0:
        # Scalar
        return 0, 0, 0, bitwidth

    elif number_of_dimensions == 2 and dimension[0] == 1:
        # 1-dimensional tensor
        dimensions = dimension[0]
        mem_size = dimension[1].__int__()
        mem_index_bitwidth = int(math.log2(mem_size))

    elif number_of_dimensions == 2:
        # 2-dimensional tensor
        dimensions = number_of_dimensions
        mem_size = (dimension[0].__int__(), dimension[1].__int__())
        mem_index_bitwidth = (int(math.log2(mem_size[0])), int(math.log2(mem_size[1])))
    else:
        assert (False), "Unimplemented."
        # 3-dimensional tensor

    return dimensions, mem_size, mem_index_bitwidth, bitwidth


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
