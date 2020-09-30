from . import ast

COUNTER = 0


def emit_mem_decl(name, size):
    return "{} = prim std_mem_d1(32, {}, {});".format(
        name,
        str(size),
        str(32)
    )


def emit_reg_decl(name, size):
    return "{} = prim std_reg({});".format(name, 32)


def emit_cond_group(suffix):
    return '''
group cond{0} {{
  le{0}.left = idx{0}.out;
  le{0}.right = 32'd10;

  cond{0}.done = 1'b1;
}}
    '''.format(suffix)


def emit_idx_group(suffix):
    return '''
group incr_idx{0} {{
  adder_idx{0}.left = idx{0}.out;
  adder_idx{0}.right = 32'b1;

  idx{0}.write_en = 1'b1;
  idx{0}.in = adder_idx{0}.out;

  incr_idx{0}[done] = idx{0}.done;
}}
    '''.format(COUNTER)


def emit_mem_offset(suffix, stmt):
    if isinstance(stmt.op, ast.Map):
        return "{}.addr0 = idx{}.out;".format(stmt.op.bind[0].src, suffix)
    else:
        return "{}.addr0 = idx{}.out;".format(stmt.op.bind[0].dest[1], suffix)


def emit_eval_body_group(suffix, stmt):
    mem_offset = emit_mem_offset(suffix, stmt)

    lhs = stmt.op.body.lhs
    rhs = stmt.op.body.rhs
    compute_left_op = (
        "{}.read_data".format(stmt.op.bind[0].src) if isinstance(lhs, ast.VarExpr)
        else "{}'d{}".format(32, lhs.value)
    )

    compute_right_op = (
        "{}.read_data".format(stmt.op.bind[0].src) if isinstance(rhs, ast.VarExpr)
        else "{}'d{}".format(32, rhs.value)
    )

    if isinstance(stmt.op, ast.Reduce):
        write = "{}.in = adder_op{}.out;".format(stmt.dest, suffix)
    else:
        write = "{}.write_data = adder_op{}.out;".format(stmt.dest, suffix)

    return '''
group eval_body{0} {{
  {1}.write_en = 1'b1;
  {1}.addr0 = idx{0}.out;

  {4}

  adder_op{0}.left = {2};
  adder_op{0}.right = {3};

  {5}

  eval_body{0}[done] = {1}.done;
}}
    '''.format(
            suffix,
            stmt.dest,
            compute_left_op,
            compute_right_op,
            mem_offset,
            write
        )


def gen_stmt_impl(stmt):
    result = dict()

    cells = []
    cells.append("le{} = prim std_lt(32);".format(COUNTER))
    cells.append("idx{} = prim std_reg(32);".format(COUNTER))
    cells.append("adder_idx{} = prim std_add(32);".format(COUNTER))
    cells.append("adder_op{} = prim std_add(32);".format(COUNTER))

    wires = []
    wires.append(emit_cond_group(COUNTER))
    wires.append(emit_idx_group(COUNTER))
    wires.append(emit_eval_body_group(COUNTER, stmt))

    control = []
    control.append('''
    while le{0}.out with cond{0} {{
      seq {{ eval_body{0}; incr_idx{0}; }}
    }}
    '''.format(COUNTER))
    return { "cells": cells, "wires": wires, "control": control }


def emit(prog):
    global COUNTER
    mems = []
    regs = []

    cells = []
    wires = []
    control = []

    for decl in prog.decls:
        if decl.type.size:  # A memory
            cells.append(emit_mem_decl(decl.name, decl.type.size))
        else:  # A register
            cells.append(emit_reg_decl(decl.name, 32))

    for stmt in prog.stmts:
        cells += gen_stmt_impl(stmt)["cells"]
        wires += gen_stmt_impl(stmt)["wires"]
        control += gen_stmt_impl(stmt)["control"]
        COUNTER += 1

    emitted = '''
import "primitives/std.lib";
component main() -> () {{
  cells {{
    {}
  }}

  wires {{
    {}
  }}

  control {{
    seq {{
      {}
    }}
  }}
}}
'''.format(
        "\n ".join(cells),
        "\n ".join(wires),
        "\n ".join(control)
    )
    print(emitted)

