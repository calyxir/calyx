{ 
  open Parser 
  exception ParseError of string
}

let id = ['a'-'z' 'A'-'Z' '_'] ['a'-'z' 'A'-'Z' '0'-'9' '_' '.']*

rule tokens = parse 
(* i.e., 1'd1 *)
| ['0'-'9']+"'d"['0'-'9']+ as i { INT i } 
| "("             { LPAREN }
| ")"             { RPAREN }
| "components"    { COMPONENTS }
| "entrypoint"    { ENTRYPOINT }
| "name"          { NAME }
| "signature"     { SIGNATURE }
| "cells"         { CELLS }
| "ports"         { PORTS }
| "prototype"     { PROTOTYPE }
| "param_binding" { PARAM_BINDING }
| "reference"     { REFERENCE }
| "groups"        { GROUPS }
| "static_groups" { STATIC_GROUPS }
| "comb_groups"   { COMB_GROUPS }
| "continuous_assignments" {CONT_ASSNS}
| "dst"           { DST }
| "src"           { SRC }
| "guard"         { GUARD }
| "attributes"    { ATTRIBUTES }
| "span"          { SPAN }
| "attrs"         { ATTRS }
| "control"       { CONTROL }
| "Seq"           { SEQ }
| "Enable"        { ENABLE }
| "stmts"         { STMTS }
| "is_comb"       { IS_COMB }
| "true"          { TRUE }
| "false"         { FALSE }
| "num"           { NUM }
| "input"         { INPUT }
| "output"        { OUTPUT }
| "inout"         { INOUT }
| "width"         { WIDTH }
| "holes"         { HOLES }
| "parent"        { PARENT }
| "direction"     { DIRECTION }
| "assignments"   { ASSIGNMENTS }
| eof             { EOF }
| id as x         { ID x }
| _ { raise (ParseError (Printf.sprintf "At offset %d: unexpected character.\n" (Lexing.lexeme_start lexbuf))) }