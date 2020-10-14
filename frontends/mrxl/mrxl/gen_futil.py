from . import ast

COUNTER = 0


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


def emit_idx_group(suffix, b=None):
    bank_suffix = "_b" + str(b) + "_" if b is not None else ""
    return '''
group incr_idx{1}{0} {{
  adder_idx{1}{0}.left = idx{1}{0}.out;
  adder_idx{1}{0}.right = 32'b1;

  idx{1}{0}.write_en = 1'b1;
  idx{1}{0}.in = adder_idx{1}{0}.out;

  incr_idx{1}{0}[done] = idx{1}{0}.done;
}}
    '''.format(COUNTER, bank_suffix)


def emit_eval_body_group(suffix, stmt, b=None):
    bank_suffix = "_b" + str(b) if b is not None else ""

    mem_offsets = []
    name2arr = dict()
    for bi in stmt.op.bind:
        idx = 0 if isinstance(stmt.op, ast.Map) else 1
        name2arr[bi.dest[idx]] = bi.src
        mem_offsets.append(
            "{0}{1}.addr0 = idx{1}_{2}.out;".format(bi.src, bank_suffix, suffix)
        )

    if isinstance(stmt.op, ast.Map):
        mem_offsets.append(
            "{0}{1}.addr0 = idx{1}_{2}.out;".format(stmt.dest, bank_suffix, suffix)
        )

    lhs = stmt.op.body.lhs
    rhs = stmt.op.body.rhs

    if isinstance(lhs, ast.VarExpr):
        if isinstance(stmt.op, ast.Map):
            compute_left_op = "{}{}.read_data".format(name2arr[lhs.name], bank_suffix)
        else:
            compute_left_op = "{}.out".format(stmt.dest, suffix)
    else:
        compute_left_op = "{}'d{}".format(32, lhs.value)


    if isinstance(rhs, ast.VarExpr):
        compute_right_op = "{}{}.read_data".format(name2arr[rhs.name], bank_suffix)
    else:
        compute_right_op = "{}'d{}".format(32, rhs.value)

    if isinstance(stmt.op, ast.Map):
        write = "{0}{1}.write_data = adder_op{1}_{2}.out;".format(stmt.dest, bank_suffix, suffix)
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


def gen_stmt_impl(stmt, arr_size, name2par):
    result = dict()

    cells = []
    if isinstance(stmt.op, ast.Map):
        for b in range(name2par[stmt.dest]):
            cells.append("le_b{}_{} = prim std_lt(32);".format(b, COUNTER))
            cells.append("idx_b{}_{} = prim std_reg(32);".format(b, COUNTER))
            cells.append("adder_idx_b{}_{} = prim std_add(32);".format(b, COUNTER))
    else:
        cells.append("le{} = prim std_lt(32);".format(COUNTER))
        cells.append("idx{} = prim std_reg(32);".format(COUNTER))
        cells.append("adder_idx{} = prim std_add(32);".format(COUNTER))

    op_name = "mult" if stmt.op.body.op == "mul" else "add"
    if isinstance(stmt.op, ast.Map):
        for b in range(name2par[stmt.dest]):
            cells.append("adder_op_b{}_{} = prim std_{}(32);".format(b, COUNTER, op_name))
    else:
        cells.append("adder_op{} = prim std_{}(32);".format(COUNTER, op_name))

    wires = []
    if isinstance(stmt.op, ast.Map):
        for b in range(name2par[stmt.dest]):
            wires.append(emit_cond_group(COUNTER, arr_size // name2par[stmt.dest], b))
    else:
        wires.append(emit_cond_group(COUNTER, arr_size))
    if isinstance(stmt.op, ast.Map):
        for b in range(name2par[stmt.dest]):
            wires.append(emit_idx_group(COUNTER, b))
    else:
        wires.append(emit_idx_group(COUNTER))

    if isinstance(stmt.op, ast.Map):
        for b in range(name2par[stmt.dest]):
            wires.append(emit_eval_body_group(COUNTER, stmt, b))
    else:
            wires.append(emit_eval_body_group(COUNTER, stmt))

    control = []
    map_loops = []
    for b in range(name2par[stmt.dest]):
        map_loops.append('''
        while le{0}{1}.out with cond{0}{1} {{
          seq {{ eval_body{0}{1}; incr_idx{0}{1}; }}
        }}
        '''.format("_b" + str(b) + "_", COUNTER
        ))

    control.append('''
    par {{
      {} 
    }}
    '''.format("\n".join(map_loops)))

    return {"cells": cells, "wires": wires, "control": control}


def emit(prog):
    global COUNTER
    mems = []
    regs = []

    cells = []
    wires = []
    control = []

    arr_size = None
    used_names = []
    
    name2par = dict()
    for stmt in prog.stmts:
        if isinstance(stmt.op, ast.Map):
            name2par[stmt.dest] = stmt.op.par
            for b in stmt.op.bind:
                name2par[b.src] = stmt.op.par

    for decl in prog.decls:
        used_names.append(decl.name)
        if decl.type.size:  # A memory
            arr_size = decl.type.size
            cells.append(emit_mem_decl(decl.name, decl.type.size, name2par[decl.name]))
        else:  # A register
            cells.append(emit_reg_decl(decl.name, 32))

    for stmt in prog.stmts:
        if stmt.dest not in used_names:
            if isinstance(stmt.op, ast.Map):
                cells.append(emit_mem_decl(stmt.dest, arr_size, name2par[stmt.dest]))
            else:
                cells.append(emit_reg_decl(stmt.dest, 32))
            used_names.append(stmt.dest)

    assert arr_size is not None

    for stmt in prog.stmts:
        stmt_impl = gen_stmt_impl(stmt, arr_size, name2par)
        cells += stmt_impl["cells"]
        wires += stmt_impl["wires"]
        control += stmt_impl["control"]
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

