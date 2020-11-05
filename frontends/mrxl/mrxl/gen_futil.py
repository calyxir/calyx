from . import ast


def emit_mem_decl(name, size, par):
    banked_mems = []
    for i in range(par):
        banked_mems.append("{} = prim std_mem_d1(32, {}, {});".format(
            name + "_b" + str(i),
            str(size // par),
            str(32)
        ))
    return "\n".join(banked_mems)


def emit_reg_decl(name, size):
    return "{} = prim std_reg({});".format(name, 32)


def emit_cond_group(suffix, arr_size, b=None):
    bank_suffix = "_b" + str(b) + "_" if b is not None else ""
    return '''
group cond{2}{0} {{
  le{2}{0}.left = idx{2}{0}.out;
  le{2}{0}.right = 32'd{1};

  cond{2}{0}.done = 1'b1;
}}
    '''.format(suffix, arr_size, bank_suffix)


def emit_idx_group(s_idx, b=None):
    bank_suffix = "_b" + str(b) + "_" if b is not None else ""
    return '''
group incr_idx{1}{0} {{
  adder_idx{1}{0}.left = idx{1}{0}.out;
  adder_idx{1}{0}.right = 32'b1;

  idx{1}{0}.write_en = 1'b1;
  idx{1}{0}.in = adder_idx{1}{0}.out;

  incr_idx{1}{0}[done] = idx{1}{0}.done;
}}
    '''.format(s_idx, bank_suffix)


def emit_compute_op(exp, op, dest, name2arr, suffix, bank_suffix):
    if isinstance(exp, ast.VarExpr):
        if isinstance(op, ast.Map):
            return "{}{}.read_data".format(name2arr[exp.name], bank_suffix)
        else:
            return "{}.out".format(dest, suffix)
    else:
        return "{}'d{}".format(32, exp.value)


def emit_eval_body_group(suffix, stmt, b=None):
    bank_suffix = "_b" + str(b) if b is not None else ""

    mem_offsets = []
    name2arr = dict()
    for bi in stmt.op.bind:
        idx = 0 if isinstance(stmt.op, ast.Map) else 1
        name2arr[bi.dest[idx]] = bi.src
        mem_offsets.append(
            "{0}{1}.addr0 = idx{1}_{2}.out;".format(
                bi.src, bank_suffix, suffix
            )
        )

    if isinstance(stmt.op, ast.Map):
        mem_offsets.append(
            "{0}{1}.addr0 = idx{1}_{2}.out;".format(
                stmt.dest, bank_suffix, suffix
            )
        )

    compute_left_op = emit_compute_op(
        stmt.op.body.lhs, stmt.op, stmt.dest, name2arr, suffix, bank_suffix
    )

    compute_right_op = emit_compute_op(
        stmt.op.body.rhs, stmt.op, stmt.dest, name2arr, suffix, bank_suffix
    )

    if isinstance(stmt.op, ast.Map):
        write = "{0}{1}.write_data = adder_op{1}_{2}.out;".format(
                stmt.dest, bank_suffix, suffix
        )
    else:
        write = "{}.in = adder_op{}.out;".format(stmt.dest, suffix)

    return '''
group eval_body{6}_{0} {{
  {1}{6}.write_en = 1'b1;

  {4}

  adder_op{6}_{0}.left = {2};
  adder_op{6}_{0}.right = {3};

  {5}

  eval_body{6}_{0}[done] = {1}{6}.done;
}}
    '''.format(
            suffix,
            stmt.dest,
            compute_left_op,
            compute_right_op,
            "\n".join(mem_offsets),
            write,
            bank_suffix
        )


def gen_reduce_impl(stmt, arr_size, s_idx):
    result = dict()

    cells = []
    op_name = "mult" if stmt.op.body.op == "mul" else "add"
    cells.append("le{} = prim std_lt(32);".format(s_idx))
    cells.append("idx{} = prim std_reg(32);".format(s_idx))
    cells.append("adder_idx{} = prim std_add(32);".format(s_idx))
    cells.append("adder_op{} = prim std_{}(32);".format(s_idx, op_name))

    wires = []
    wires.append(emit_cond_group(s_idx, arr_size))
    wires.append(emit_idx_group(s_idx))
    wires.append(emit_eval_body_group(s_idx, stmt, 0))

    control = []
    control.append('''
while le{0}.out with cond{0} {{
  seq {{ eval_body{0}; incr_idx{0}; }}
}}
    '''.format(s_idx))

    return {"cells": cells, "wires": wires, "control": control}


def gen_map_impl(stmt, arr_size, bank_factor, s_idx):
    result = dict()

    cells = []
    for b in range(bank_factor):
        cells.append("le_b{}_{} = prim std_lt(32);".format(b, s_idx))
        cells.append("idx_b{}_{} = prim std_reg(32);".format(b, s_idx))
        cells.append("adder_idx_b{}_{} = prim std_add(32);".format(b, s_idx))

    op_name = "mult" if stmt.op.body.op == "mul" else "add"
    for b in range(bank_factor):
        cells.append("adder_op_b{}_{} = prim std_{}(32);".format(
            b, s_idx, op_name
        ))

    wires = []
    for b in range(bank_factor):
        wires.append(emit_cond_group(s_idx, arr_size // bank_factor, b))
        wires.append(emit_idx_group(s_idx, b))
        wires.append(emit_eval_body_group(s_idx, stmt, b))

    control = []
    map_loops = []
    for b in range(bank_factor):
        map_loops.append('''
{2}while le{0}{1}.out with cond{0}{1} {{
{2}  seq {{ eval_body{0}{1}; incr_idx{0}{1}; }}
{2}}}
        '''.format("_b" + str(b) + "_", s_idx, 8 * " "))

    control.append('''
{1}par {{
{1}  {0}
{1}}}
    '''.format("".join(map_loops), 6 * " "))

    return {"cells": cells, "wires": wires, "control": control}


def gen_stmt_impl(stmt, arr_size, name2par, s_idx):
    if isinstance(stmt.op, ast.Map):
        return gen_map_impl(stmt, arr_size, name2par[stmt.dest], s_idx)
    else:
        return gen_reduce_impl(stmt, arr_size, s_idx)


def emit(prog):
    cells = []
    wires = []
    control = []

    print(prog)
    # All arrays must be the same size. The first array we see determines the
    # size that we'll assume for the rest of the program's arrays.
    arr_size = None

    # Collect banking factors.
    name2par = dict()
    for stmt in prog.stmts:
        if isinstance(stmt.op, ast.Map):
            name2par[stmt.dest] = stmt.op.par
            for b in stmt.op.bind:
                name2par[b.src] = stmt.op.par

    # Collect memory and register declarations.
    used_names = []
    for decl in prog.decls:
        used_names.append(decl.name)
        if decl.type.size:  # A memory
            arr_size = decl.type.size
            cells.append(emit_mem_decl(decl.name, decl.type.size, name2par[decl.name]))
        else:  # A register
            cells.append(emit_reg_decl(decl.name, 32))

    # Collect implicit memory and register declarations.
    for stmt in prog.stmts:
        if stmt.dest not in used_names:
            if isinstance(stmt.op, ast.Map):
                cells.append(emit_mem_decl(stmt.dest, arr_size, name2par[stmt.dest]))
            else:
                cells.append(emit_reg_decl(stmt.dest, 32))
            used_names.append(stmt.dest)

    # Generate FuTIL.
    for i, stmt in enumerate(prog.stmts):
        stmt_impl = gen_stmt_impl(stmt, arr_size, name2par, i)
        cells += stmt_impl["cells"]
        wires += stmt_impl["wires"]
        control += stmt_impl["control"]

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
        "\n".join(cells),
        "\n".join(wires),
        "".join(control)
    )
    print(emitted)
