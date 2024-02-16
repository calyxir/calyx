module.exports = grammar({
  name: 'calyx',

  extras: $ => [
    /\s|\\\r?\n/,
    $.comment
  ],

  rules: {
    source_file: $ => seq(
      repeat($.import),
      repeat(choice($.component, $.extern, $.primitive)),
      optional($.metadata)
    ),
    comment: $ => choice(
      seq('//', /.*/, '\n'),
      seq(/\/\*(.|\n)*\*\//)
    ),

    import: $ => seq('import', $.string, ';'),

    metadata_ci: $ => seq(/[mM][eE][tT][aA][dD][aA][tT][aA]/),
    metadata: $ => seq($.metadata_ci, /#\{(.|\n)*\}#/),

    // components
    signature: $ => seq($.io_port_list, '->', $.io_port_list),
    component: $ => seq(
      optional($.comb_or_static),
      'component',
      $.ident, optional($.attributes),
      $.signature, '{',
      optional($.cells),
      optional($.wires),
      optional($.control),
      '}'
    ),
    comb_or_static: $ => choice(
      'comb',
      $.static_annotation,
      seq('comb', $.static_annotation),
      seq($.static_annotation, 'comb')
    ),
    io_port: $ => seq(repeat($.at_attribute), $.ident, ':', choice($.number, $.ident)),
    io_port_list: $ => seq('(', repeat(seq($.io_port, ',')), optional($.io_port), ')'),

    // primitives
    params: $ => seq('[', seq(repeat(seq($.ident, ',')), $.ident), ']'),
    any_line: $ => /([^\}\n])*\n/,
    primitive_blob: $ => seq(
      '{',
      repeat($.any_line),
      '}'
    ),
    primitive: $ => seq(
      optional($.comb_or_static), 'primitive', $.ident, optional($.attributes),
      optional($.params), $.signature,
      optional($.primitive_blob),
      ';'
    ),
    extern: $ => seq('extern', $.string, '{', repeat($.primitive), '}'),

    // cells
    cells: $ => seq('cells', '{', optional($.cells_inner), '}'),
    cells_inner: $ => repeat1($.cell_assignment),
    cell_assignment: $ => seq(
      optional($.at_attribute), optional('ref'),
      $.ident, '=', $.instantiation, optional(';')
    ),
    instantiation: $ => seq($.ident, $.arg_list),
    arg_list: $ => seq('(', repeat(seq($.number, optional(','))), ')'),

    // wires
    wires: $ => seq('wires', '{', optional($.wires_inner), '}'),
    wires_inner: $ => repeat1(choice($.group, $.wire_assignment)),
    group: $ => seq(
      optional('comb'), 'group', $.ident, optional($.attributes),
      '{',
      repeat($.wire_assignment),
      '}'
    ),
    hole: $ => seq($.ident, '[', $.ident, ']'),
    port: $ => choice(seq($.ident, '.', $.ident), $.ident),
    lhs: $ => choice($.hole, $.port),
    base_expr: $ => choice($.hole, $.port, $.literal),
    expr: $ => choice(
      $.base_expr,
      $.cmp_expr,
      seq('(', $.expr, ')'),
    ),
    cmp_expr: $ => choice(
      prec.left(6, seq($.expr, "==", $.expr)),
      prec.left(5, seq($.expr, "!=", $.expr)),
      prec.left(4, seq($.expr, "<=", $.expr)),
      prec.left(3, seq($.expr, ">=", $.expr)),
      prec.left(2, seq($.expr, ">", $.expr)),
      prec.left(1, seq($.expr, "<", $.expr)),
    ),
    term: $ => choice(
      $.expr,
      seq('!', $.expr),
      prec.left(2, seq($.expr, '|', $.expr)),
      prec.left(1, seq($.expr, '&', $.expr))
    ),
    switch: $ => seq($.term, '?', $.base_expr),
    wire_assignment: $ => seq(optional($.at_attribute), $.lhs, '=', choice($.switch, $.expr), ';'),

    // control
    control: $ => seq('control', '{', $.control_inner, '}'),
    control_inner: $ => $.stmt,
    enable: $ => seq(repeat($.at_attribute), optional($.ident), ';'),
    invoke_ref_arg: $ => seq($.ident, '=', $.ident),
    invoke_ref_args: $ => seq(
      '[',
      optional(seq(repeat(seq($.invoke_ref_arg, ',')), $.invoke_ref_arg)),
      ']'
    ),
    invoke_arg: $ => seq($.ident, '=', choice($.port, $.literal)),
    invoke_args: $ => seq(
      '(',
      optional(seq(repeat(seq($.invoke_arg, ',')), $.invoke_arg)),
      ')'
    ),
    invoke: $ => seq(
      repeat($.at_attribute), optional($.static_annotation), 'invoke',
      $.ident,
      optional($.invoke_ref_args),
      $.invoke_args,
      $.invoke_args,
      optional(seq('with', $.ident)),
      ';'
    ),
    seq: $ => seq(
      repeat($.at_attribute), optional($.static_annotation),
      'seq', '{',
      repeat($.stmt),
      '}'
    ),
    par: $ => seq(
      repeat($.at_attribute), optional($.static_annotation),
      'par', '{',
      repeat($.stmt),
      '}'
    ),
    port_with: $ => seq($.port, optional(seq('with', $.ident))),
    block: $ => seq('{', $.stmt, '}'),
    if_stmt: $ => seq(
      repeat($.at_attribute), optional($.static_annotation),
      'if', $.port_with, $.block,
      optional(seq('else', choice($.block, $.if_stmt)))
    ),
    while_stmt: $ => seq(repeat($.at_attribute), 'while', $.port_with, $.block),
    repeat_stmt: $ => seq(repeat($.at_attribute), optional('static'), 'repeat', $.number, $.block),
    stmt: $ => choice(
      $.enable,
      $.invoke,
      $.seq,
      $.par,
      $.if_stmt,
      $.while_stmt,
      $.repeat_stmt
    ),

    attribute: $ => seq($.string, "=", $.number),
    attributes: $ => seq('<', seq(repeat(seq($.attribute, ',')), optional($.attribute)), '>'),
    latency_annotation: $ => seq('<', $.number, '>'),
    at_attribute: $ => seq('@', $.ident, optional(seq('(', $.number, ')'))),
    static_annotation: $ => seq('static', optional($.latency_annotation)),

    string: $ => /".*"/,
    ident: $ => /[a-zA-Z_]+[a-zA-Z0-9_\-]*/,
    number: $ => /[0-9]+/,
    literal: $ => seq($.number, "'", choice('d', 'b', 'x', 'o'), $.number)
  }
});
