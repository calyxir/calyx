%{
  open Core
  open Extr
%}

%token <int> INT 
%token <string> ID
%token <string> STRING
%token DOT
(* numerical attributes *)
%token NUM GO DONE STATIC WRITE_TOGETHER
(* boolean attributes *)
%token BOOL TOP_LEVEL EXTERNAL NO_INTERFACE RESET CLK STABLE DATA
(* more boolean attributes *)
%token CAPS_CONTROL SHARE STATE_SHARE GENERATED NEW_FSM INLINE
%token COMPONENTS ENTRYPOINT
%token NAME SIGNATURE CELLS GROUPS STATIC_GROUPS COMB_GROUPS CONT_ASSNS CONTROL IS_COMB ATTRIBUTES
%token TRUE FALSE DST SRC GUARD PORTS PROTOTYPE THIS_COMPONENT REFERENCE
%token LPAREN RPAREN EOF
%token INPUT OUTPUT INOUT
%token WIDTH
%token HOLES
%token PARENT
%token DIRECTION
%token ASSIGNMENTS
%token LATENCY
%token PRIMITIVE
%token VAL
%token PARAM_BINDING
%token CONSTANT
(* Guard expressions. *)
%token PORT AND
(* Control statements. *)
%token SEQ ENABLE EMPTY STMTS GROUP

%start <Extr.context> main
%%

main: 
  | LPAREN;
      LPAREN; COMPONENTS; comps = list(component); RPAREN; 
      LPAREN; ENTRYPOINT; DOT; entry = STRING; RPAREN;
    RPAREN; EOF
  { {ctx_comps = comps; ctx_entrypoint = entry} }

attrs_clause:
  | LPAREN; ATTRIBUTES; attrs = list(attribute); RPAREN
   { attrs }

component: 
  | LPAREN;
      LPAREN; NAME; DOT; name = STRING; RPAREN; 
      LPAREN; SIGNATURE; signature = cell; RPAREN; 
      LPAREN; CELLS; cells = list(paren_cell); RPAREN;
      LPAREN; GROUPS; groups = list(paren_group); RPAREN; 
      LPAREN; STATIC_GROUPS; sgroups = list(sgroup); RPAREN; 
      LPAREN; COMB_GROUPS; cgroups = list(cgroup); RPAREN; 
      LPAREN; CONT_ASSNS; assns = list(assignment); RPAREN; 
      LPAREN; CONTROL; ctl = control; RPAREN; 
      attributes = attrs_clause;
      LPAREN; IS_COMB; DOT; is_comb = bool; RPAREN;
      LPAREN; LATENCY; RPAREN;
    RPAREN
{ {comp_attrs = attributes; comp_name = name; comp_sig = signature;
comp_cells = cells; comp_groups = groups; comp_comb_groups = cgroups;
comp_static_groups = sgroups; comp_cont_assns = assns; comp_control = ctl;
comp_is_comb = is_comb} }

cgroup:
  | LPAREN;
      LPAREN; NAME; DOT; comb_group_name = STRING; RPAREN;
      LPAREN; ASSIGNMENTS; comb_group_assns = list(assignment); RPAREN;
      comb_group_attrs = attrs_clause;
    RPAREN
    { { comb_group_name;
        comb_group_attrs;
        comb_group_assns } }

paren_cell:
  | LPAREN; cell = cell; RPAREN { cell }

cell:
  | LPAREN; NAME; DOT; name = STRING; RPAREN; 
    LPAREN; PORTS; ports = list(port); RPAREN;
    LPAREN; PROTOTYPE; proto = prototype; RPAREN; 
    attributes = attrs_clause;
    LPAREN; REFERENCE; DOT; reference = bool; RPAREN;
{ let ins = List.filter ports ~f:(fun p -> is_in p.port_dir) in
  let outs = List.filter ports ~f:(fun p -> is_out p.port_dir) in
  {cell_name = name;
   cell_attrs = attributes;
   cell_in_ports = ins;
   cell_out_ports = outs;
   cell_proto = proto;
   cell_ref = reference} }

port: 
| LPAREN; LPAREN; NAME; DOT; name = STRING; RPAREN; 
    LPAREN; WIDTH; DOT; width = INT; RPAREN;
    LPAREN; DIRECTION; DOT; dir = direction; RPAREN;
    LPAREN; PARENT; DOT; par = STRING; RPAREN; 
    attributes = attrs_clause;
  RPAREN
  { {port_name = name; port_width = width; port_dir = dir; parent = par; 
     port_attribute = attributes} }

port_ref: 
| LPAREN; NAME; DOT; name = STRING; RPAREN; 
  LPAREN; WIDTH; DOT; width = INT; RPAREN;
  LPAREN; DIRECTION; DOT; dir = direction; RPAREN;
  LPAREN; PARENT; DOT; par = STRING; RPAREN; 
  attributes = attrs_clause;
    { let _ = attributes in
      let _ = width in
      let _ = dir in
      if String.equal par "_this"
      then PThis name
      else PRef (par, name) }

direction: 
| INPUT { Input }
| OUTPUT { Output }
| INOUT { InOut }

paren_group:
  | LPAREN; group = group; RPAREN { group }

group:
  | LPAREN; NAME; DOT; group_name = STRING; RPAREN;
    LPAREN; ASSIGNMENTS; group_assns = list(assignment); RPAREN;
    LPAREN; HOLES; group_holes = list(port); RPAREN;
    group_attrs = attrs_clause
    { { group_attrs;
        group_name;
        group_assns;
        group_holes; } }

guard:
| TRUE
  { GTrue }
| LPAREN; PORT; p = port; RPAREN
  { GPort p }
| LPAREN; AND; g1 = guard; g2 = guard; RPAREN
  { GAnd (g1, g2) }

assignment: 
  | LPAREN;
      LPAREN; DST; dst = port_ref; RPAREN;
      LPAREN; SRC; src = port_ref; RPAREN; 
      LPAREN; GUARD; DOT; assign_guard = guard; RPAREN; 
      attrs = attrs_clause;
    RPAREN
    { { dst; src; assign_guard; attrs } }

control: 
  | LPAREN; EMPTY; LPAREN; attrs = attrs_clause; RPAREN; RPAREN
    { CEmpty attrs }
  | LPAREN; SEQ; LPAREN;
      LPAREN; STMTS; stmts = list(control); RPAREN;
      attrs = attrs_clause;
    RPAREN; RPAREN
    { CSeq (stmts, attrs) }
  | ENABLE; LPAREN; GROUP; grp = group; RPAREN; attrs = attrs_clause
    { CEnable (grp.group_name, attrs) }

num_attr_name:
| GO { Go }
| DONE { Done }
| STATIC { Static }
| WRITE_TOGETHER { WriteTogether }

bool_attr_name:
| TOP_LEVEL { TopLevel }
| EXTERNAL { External }
| NO_INTERFACE { NoInterface }
| RESET { Reset }
| CLK { Clk }
| STABLE { Stable }
| DATA { Data }
| CAPS_CONTROL { Control }
| SHARE { Share }
| STATE_SHARE { StateShare }
| GENERATED { Generated }
| NEW_FSM { NewFSM }
| INLINE { Inline }

attribute:
| LPAREN; LPAREN; NUM; DOT; name = num_attr_name; RPAREN; DOT; value = INT; RPAREN
   { NumAttr (name, value) }
| LPAREN; LPAREN; BOOL; DOT; name = bool_attr_name; RPAREN; DOT; value = INT; RPAREN
   { BoolAttr (name, value <> 0) }

bool: 
| TRUE { true }
| FALSE { false }

param_binding:
| LPAREN; name = STRING; value = INT; RPAREN
  { (name, value) }

prototype:
  (* TODO other cases *)
  | DOT; THIS_COMPONENT
    { ProtoThis }
  | PRIMITIVE;
    LPAREN; NAME; DOT; name = STRING; RPAREN; 
    LPAREN; PARAM_BINDING; param_binding = list(param_binding); RPAREN; 
    LPAREN; IS_COMB; DOT; is_comb = bool; RPAREN;
    LPAREN; LATENCY; RPAREN;
    { ProtoPrim (name, param_binding, is_comb) }
  | CONSTANT;
      LPAREN; VAL; DOT; value = INT; RPAREN;
      LPAREN; WIDTH; DOT; width = INT; RPAREN;
    { ProtoConst (value, width) }

sgroup:
  | LPAREN;
      LPAREN; NAME; static_group_name = ID; RPAREN;
      LPAREN; ASSIGNMENTS; static_group_assns = list(assignment); RPAREN;
      LPAREN; HOLES; static_group_holes = list(port); RPAREN;
      static_group_attrs = attrs_clause;
    RPAREN
    { { static_group_attrs;
        static_group_name;
        static_group_assns;
        static_group_holes;
        static_latency = failwith "couldn't parse latency of a static group" } }
