%{
  open Core
  open Extr
%}

%token <int> INT 
%token <string> ID
(* numerical attributes *)
%token NUM GO DONE
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
%token EMPTY
%token LATENCY

%start <Extr.context> main
%%

main: 
  | LPAREN;
      LPAREN; COMPONENTS; LPAREN; comps = list(component); RPAREN; RPAREN; 
      LPAREN; ENTRYPOINT; entry = ID; RPAREN;
    RPAREN; EOF
  { {ctx_comps = comps; ctx_entrypoint = entry} }

attrs_clause:
  | LPAREN; ATTRIBUTES; LPAREN; attrs = list(attribute); RPAREN; RPAREN
   { attrs }

component: 
  | LPAREN;
      LPAREN; NAME; name = ID; RPAREN; 
      LPAREN; SIGNATURE; signature = cell; RPAREN; 
      LPAREN; CELLS; LPAREN; cells = list(cell); RPAREN; RPAREN;
      LPAREN; GROUPS; LPAREN; groups = list(group); RPAREN; RPAREN; 
      LPAREN; STATIC_GROUPS; LPAREN; sgroups = list(sgroup); RPAREN; RPAREN; 
      LPAREN; COMB_GROUPS; LPAREN; cgroups = list(cgroup); RPAREN; RPAREN; 
      LPAREN; CONT_ASSNS; LPAREN; assns = list(assignment); RPAREN; RPAREN; 
      LPAREN; CONTROL; LPAREN; ctl = control; RPAREN; RPAREN; 
      attributes = attrs_clause;
      LPAREN; IS_COMB; comb = bool; RPAREN;
      LPAREN; LATENCY; LPAREN; RPAREN; RPAREN;
    RPAREN
{ {comp_attrs = attributes; comp_name = name; comp_sig = signature;
comp_cells = cells; comp_groups = groups; comp_comb_groups = cgroups;
comp_static_groups = sgroups; comp_cont_assns = assns; comp_control = ctl;
comp_is_comb = comb} }

cgroup:
  | LPAREN;
      LPAREN; NAME; comb_group_name = ID; RPAREN;
      LPAREN; ASSIGNMENTS; LPAREN; comb_group_assns = list(assignment); RPAREN; RPAREN;
      comb_group_attrs = attrs_clause;
    RPAREN
    { { comb_group_name;
        comb_group_attrs;
        comb_group_assns } }
cell:
| LPAREN; LPAREN; NAME; name = ID; RPAREN; 
LPAREN; PORTS; LPAREN; ports = list(port); RPAREN; RPAREN;
LPAREN; PROTOTYPE; proto = prototype; RPAREN; 
attributes = attrs_clause;
LPAREN; REFERENCE; reference = bool; RPAREN; RPAREN
{ let ins = List.filter ports ~f:(fun p -> is_in p.port_dir) in
  let outs = List.filter ports ~f:(fun p -> is_out p.port_dir) in
  {cell_name = name;
   cell_attrs = attributes;
   cell_in_ports = ins;
   cell_out_ports = outs;
   cell_proto = proto;
   cell_ref = reference} }

port: 
| LPAREN; LPAREN; NAME; name = ID; RPAREN; 
    LPAREN; WIDTH; width = INT; RPAREN;
    LPAREN; DIRECTION; dir = direction; RPAREN;
    LPAREN; PARENT; par = ID; RPAREN; 
    attributes = attrs_clause;
  RPAREN
  { {port_name = name; port_width = width; port_dir = dir; parent = par; 
     port_attribute = attributes} }

direction: 
| INPUT { Input }
| OUTPUT { Output }
| INOUT { InOut }

group:
  | LPAREN;
      LPAREN; NAME; group_name = ID; RPAREN;
      LPAREN; ASSIGNMENTS; LPAREN; group_assns = list(assignment); RPAREN; RPAREN;
      LPAREN; HOLES; LPAREN; group_holes = list(port); RPAREN; RPAREN;
      group_attrs = attrs_clause;
    RPAREN
    { { group_attrs;
        group_name;
        group_assns;
        group_holes; } }

guard:
| TRUE
  { GTrue }

assignment: 
  | LPAREN;
      LPAREN; DST; dst = ID; RPAREN;
      LPAREN; SRC; src = ID; RPAREN; 
      LPAREN; GUARD; assign_guard = guard; RPAREN; 
      attrs = attrs_clause;
    RPAREN
    { { dst; src; assign_guard; attrs } }

control: 
| EMPTY; LPAREN; attrs = attrs_clause; RPAREN
  { CEmpty attrs }

num_attr_name:
| GO { Go }
| DONE { Done }

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
| LPAREN; LPAREN; NUM; name = num_attr_name; RPAREN; value = INT; RPAREN
   { NumAttr (name, value) }
| LPAREN; LPAREN; BOOL; name = bool_attr_name; RPAREN; value = INT; RPAREN
   { BoolAttr (name, value <> 0) }

bool: 
| TRUE { true }
| FALSE { false }

prototype:
(* TODO other cases *)
| THIS_COMPONENT
  { ProtoThis }

sgroup:
  | LPAREN;
      LPAREN; NAME; static_group_name = ID; RPAREN;
      LPAREN; ASSIGNMENTS; LPAREN; static_group_assns = list(assignment); RPAREN; RPAREN;
      LPAREN; HOLES; LPAREN; static_group_holes = list(port); RPAREN; RPAREN;
      static_group_attrs = attrs_clause;
    RPAREN
    { { static_group_attrs;
        static_group_name;
        static_group_assns;
        static_group_holes;
        static_latency = failwith "couldn't parse latency of a static group" } }
