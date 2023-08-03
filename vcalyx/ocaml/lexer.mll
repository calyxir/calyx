{ 
  open Parser 
  exception SyntaxError of string
}

let id = ['a'-'z' 'A'-'Z' '_'] ['a'-'z' 'A'-'Z' '0'-'9' '_' '.']*
let whitespace = [' ' '\t']+
let newline = '\r' | '\n' | "\r\n"

rule tokens = parse 
(* i.e., 1'd1 *)
| ['0'-'9']+ as i { INT (int_of_string i) } 
| whitespace      { tokens lexbuf }
| newline         { Lexing.new_line lexbuf; tokens lexbuf }
| "("             { LPAREN }
| ")"             { RPAREN }
| "components"    { COMPONENTS }
| "entrypoint"    { ENTRYPOINT }
| "name"          { NAME }
| "signature"     { SIGNATURE }
| "cells"         { CELLS }
| "ports"         { PORTS }
| "prototype"     { PROTOTYPE }
| "reference"     { REFERENCE }
| "groups"        { GROUPS }
| "static_groups" { STATIC_GROUPS }
| "comb_groups"   { COMB_GROUPS }
| "continuous_assignments" {CONT_ASSNS}
| "dst"           { DST }
| "src"           { SRC }
| "guard"         { GUARD }
| "attributes"    { ATTRIBUTES }
| "control"       { CONTROL }
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
| _ { raise (SyntaxError (Printf.sprintf "At offset %d: unexpected character %s" (Lexing.lexeme_start lexbuf) (Lexing.lexeme lexbuf))) }