%{
  open Core
  open Extr
%}

%token <n> INT 
%token <string> ID
%token COMPONENTS ENTRYPOINT
%token NAME SIGNATURE CELLS GROUPS STATIC_GROUPS COMB_GROUPS CONT_ASSNS CONTROL IS_COMB ATTRIBUTES ATTRS
%token TRUE FALSE DST SRC SEQ ENABLE STMTS GUARD PORTS PROTOTYPE PARAM_BINDING REFERENCE SPAN
%token THISCOMPONENT
%token PRIMITIVE
%token LPAREN RPAREN EOF

%start <Extr.context> main
%%

main: 
| LPAREN; LPAREN; COMPONENTS; LPAREN; comps = list(component); RPAREN; RPAREN; 
LPAREN; ENTRYPOINT; entry = ID; RPAREN; RPAREN; EOF
  { {ctx_comps = comps; ctx_entrypoint = entry} }

component: 
| LPAREN; LPAREN; NAME; name = ID; RPAREN; 
LPAREN; SIGNATURE; signature = cell; RPAREN; 
LPAREN; CELLS; LPAREN; cells = list(cell); RPAREN; RPAREN;
LPAREN; GROUPS; LPAREN; groups = list(group); RPAREN; RPAREN; 
LPAREN; STATIC_GROUPS; LPAREN; sgroups = list(sgroup); RPAREN; RPAREN; 
LPAREN; COMB_GROUPS; LPAREN; cgroups = list(cgroup); RPAREN; RPAREN; 
LPAREN; CONT_ASSNS; LPAREN; assns = list(assignment); RPAREN; RPAREN; 
LPAREN; CONTROL; LPAREN; ctl = list(control); RPAREN; RPAREN; 
LPAREN; ATTRIBUTES; LPAREN; attributes = list(attribute); RPAREN; RPAREN; 
LPAREN; IS_COMB; comb = bool; RPAREN; RPAREN
{ {comp_attrs = attributes; comp_name = name; comp_sig = signature; 
comp_cells = cells; comp_groups = groups; 
comp_comb_groups = cgroups; comp_cont_assns = assns; comp_control = ctl; 
comp_is_comb = comb} }

cell:
| LPAREN; LPAREN; NAME; name = ID; RPAREN; 
LPAREN; PORTS; LPAREN; ports = list(port); RPAREN; RPAREN;
LPAREN; PROTOTYPE; proto = prototype; RPAREN; 
LPAREN; ATTRIBUTES; LPAREN; attributes = list(attribute); RPAREN; RPAREN; 
LPAREN; REFERENCE; reference = bool; RPAREN; RPAREN
{ let ins = filter ports ~f:(fun port -> port.direction = Input) in 
  let outs = filter ports ~f:(fun port -> port.direction = Output) in 
  {cell_name = name; cell_in_ports = ins; cell_out_ports = outs;
   cell_proto = proto; cell_ref = reference} }

port: 
| LPAREN; LPAREN; NAME; name = ID; RPAREN; 
LPAREN; WIDTH; width = INT; RPAREN;
LPAREN; DIRECTION; dir = direction; RPAREN;
LPAREN; PARENT; par = ID; RPAREN; 
LPAREN; ATTRIBUTES; LPAREN; attributes = list(attribute); RPAREN; RPAREN; RPAREN
{ {port_name = name; port_width = width; port_dir = dir; parent = par; 
   port_attribute = attributes} }

direction: 
| INPUT { Input }
| OUTPUT { Output }
| INOUT { InOut }

group:
| LPAREN; LPAREN; NAME; name = ID; RPAREN;
LPAREN; ASSIGNMENTS; LPAREN; assigments = list(assignment); RPAREN;


assignment: 
| LPAREN; LPAREN; DST; dst = id; RPAREN; LPAREN; SRC; src = id; RPAREN; 
LPAREN; GUARD; LPAREN; guard = guard; RPAREN; RPAREN; 
LPAREN; ATTRIBUTES; LPAREN; attributes = list(attribute); RPAREN; RPAREN; RPAREN 
{ dst = dst; src = src; assign_guard = guard; attrs = attributes }

control: 
| 

attribute:
| 

bool: 
| TRUE { true }
| FALSE { false }