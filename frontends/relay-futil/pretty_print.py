from futil_ast import *

def mk_block(decl, contents, indent=2):
    """Format a block like this:
        decl {
          contents
        }
    where `decl` is one line but contents can be multiple lines.
    """
    return decl + ' {\n' + textwrap.indent(contents, indent * ' ') + '\n}'


def pretty_print_component(component: FComponent):
    subcomponents = []
    for cell in component.cells:
        subcomponents.append(pretty_print_cell(cell))
    cells = mk_block("cells", '\n'.join(subcomponents))

    # TODO(cgyurgyik): Need to actually make wire connections.
    wires = mk_block("wires", "")
    control = mk_block("control", "")

    return mk_block('component ' + component.name + '() -> ()', '\n'.join([cells, wires, control]))


def pretty_print_cell(cell: FCell):
    data = cell.primitive.data
    if cell.primitive.type == PrimitiveType.Register:
        return cell.primitive.name + " = " + "prim std_reg(" + str(data[0]) + ");"
    elif cell.primitive.type == PrimitiveType.Constant:
        return cell.primitive.name + " = " + "prim std_const(" + str(data[0]) + ", " + str(data[1]) + ");"
    else:
        assert False, "Unimplemented"
