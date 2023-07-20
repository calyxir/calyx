
val option_map :
  ('a1 -> 'a2) -> 'a1 option -> 'a2 option

type ('a, 'b) sum =
| Inl of 'a
| Inr of 'b

type comparison =
| Eq
| Lt
| Gt

type uint =
| Nil
| D0 of uint
| D1 of uint
| D2 of uint
| D3 of uint
| D4 of uint
| D5 of uint
| D6 of uint
| D7 of uint
| D8 of uint
| D9 of uint

type signed_int =
| Pos of uint
| Neg of uint

val eqb : bool -> bool -> bool

type positive =
| XI of positive
| XO of positive
| XH

type n =
| N0
| Npos of positive

type z =
| Z0
| Zpos of positive
| Zneg of positive

module Pos :
 sig
  val succ : positive -> positive

  val add : positive -> positive -> positive

  val add_carry :
    positive -> positive -> positive

  val mul : positive -> positive -> positive

  val of_uint_acc : uint -> positive -> positive

  val of_uint : uint -> n
 end

module N :
 sig
  val succ : n -> n
 end

val rev_append : 'a1 list -> 'a1 list -> 'a1 list

val rev' : 'a1 list -> 'a1 list

module Z :
 sig
  val opp : z -> z

  val of_N : n -> z

  val of_uint : uint -> z

  val of_int : signed_int -> z
 end

type ident = string

type attribute =
| Toplevel
| Go
| Done
| Clk
| Reset
| Nointerface
| External
| Static of int
| Inline
| Stable
| Share
| StateShare
| Bound of int
| Generated
| WriteTogether of int
| ReadTogether of int
| Data

type attributes = attribute list

type direction =
| Input
| Output
| InOut

type port_parent =
| PCell of ident
| PGroup of ident
| PStaticGroup of ident

type port = { port_name : ident; port_width : 
              n; port_dir : direction;
              parent : port_parent;
              port_attribute : attributes }

type ports = port list

type proto =
| ProtoPrim of ident * (ident * n) list * bool
| ProtoComp of ident
| ProtoThis
| ProtoConst of n * n

type cell = { cell_name : ident;
              cell_in_ports : port list;
              cell_out_ports : port list;
              cell_proto : proto; cell_ref : 
              bool }

type cells = cell list

type port_ref =
| PComp of ident * ident
| PThis of ident
| PHole of ident * ident

type guard_cmp =
| Eq0
| Neq
| Gt0
| Lt0
| Geq
| Leq

type guard_expr =
| GAnd of guard_expr * guard_expr
| GOr of guard_expr * guard_expr
| GNot of guard_expr
| GCompOp of guard_cmp * port * port
| GPort of port
| GTrue

type assignment = { dst : port; src : port;
                    assign_guard : guard_expr;
                    attrs : attributes }

type assignments = assignment list

type control =
| CSeq of control list * attributes
| CPar of control list * attributes
| CIf of port_ref * ident option * control
   * control * attributes
| CWhile of port_ref * ident option * control
   * attributes
| CEnable of ident * attributes
| CInvoke of ident * (ident * port) list
   * (ident * port) list * attributes
   * ident option * (ident * ident) list
| CEmpty of attributes

type group = { group_attrs : attributes;
               group_name : ident;
               group_assns : assignments;
               group_holes : ports }

type comb_group = { comb_group_attrs : attributes;
                    comb_group_name : ident;
                    comb_group_assns : assignments }

type comp = { comp_attrs : attribute;
              comp_name : ident;
              comp_sig : cell;
              comp_cells : cells;
              comp_groups : group list;
              comp_comb_groups : comb_group list;
              comp_cont_assns : assignments;
              comp_control : control;
              comp_is_comb : bool }

type context = { ctx_comps : comp list;
                 ctx_entrypoint : ident }

val uint_of_char :
  char -> uint option -> uint option

module NilEmpty :
 sig
  val uint_of_string : string -> uint option
 end

module NilZero :
 sig
  val uint_of_string : string -> uint option

  val int_of_string : string -> signed_int option
 end

val compcomp :
  comparison -> comparison -> comparison

val compb : bool -> bool -> comparison

val eqb_ascii : char -> char -> bool

val ascii_compare : char -> char -> comparison

val leb_ascii : char -> char -> bool

val string_elem : char -> string -> bool

val _string_reverse : string -> string -> string

val string_reverse : string -> string

val is_printable : char -> bool

val is_whitespace : char -> bool

val is_digit : char -> bool

val is_upper : char -> bool

val is_lower : char -> bool

val is_alphanum : char -> bool

type 'a sexp_ =
| Atom_ of 'a
| List of 'a sexp_ list

type atom =
| Num of z
| Str of string
| Raw of string

type loc = n

type error =
| UnmatchedClose of loc
| UnmatchedOpen of loc
| UnknownEscape of loc * char
| UnterminatedString of loc
| EmptyInput
| InvalidChar of char * loc
| InvalidStringChar of char * loc

val is_atom_char : char -> bool

type symbol =
| Open of loc
| Exp of atom sexp_

type escape =
| EscBackslash
| EscNone

type partial_token =
| NoToken
| SimpleToken of loc * string
| StrToken of loc * string * escape
| Comment

type 't parser_state_ = { parser_done : 
                          atom sexp_ list;
                          parser_stack : 
                          symbol list;
                          parser_cur_token : 
                          't }

val set_cur_token :
  'a1 parser_state_ -> 'a2 -> 'a2 parser_state_

type parser_state = partial_token parser_state_

val initial_state : parser_state

val new_sexp :
  atom sexp_ list -> symbol list -> atom sexp_
  -> 'a1 -> 'a1 parser_state_

val next_str :
  parser_state -> loc -> string -> escape -> loc
  -> char -> (error, parser_state) sum

val _fold_stack :
  atom sexp_ list -> loc -> atom sexp_ list ->
  symbol list -> (error, parser_state) sum

val next' :
  'a1 parser_state_ -> loc -> char -> (error,
  parser_state) sum

val next_comment :
  parser_state -> char -> (error, parser_state)
  sum

val raw_or_num : string -> atom

val next :
  parser_state -> loc -> char -> (error,
  parser_state) sum

val _done_or_fail :
  atom sexp_ list -> symbol list -> (error, atom
  sexp_ list) sum

val eof :
  parser_state -> loc -> (error, atom sexp_
  list) sum

val parse_sexps_ :
  parser_state -> loc -> string -> (error
  option * loc) * parser_state

val parse_sexp :
  string -> (error, atom sexp_) sum

type loc0 = int list

type message =
| MsgApp of message * message
| MsgStr of string
| MsgSexp of atom sexp_

type error0 =
| ParseError of error
| DeserError of loc0 * message

type 'a fromSexp =
  loc0 -> atom sexp_ -> (error0, 'a) sum

type 'a deserialize = 'a fromSexp

val _from_sexp : 'a1 deserialize -> 'a1 fromSexp

val from_sexp :
  'a1 deserialize -> atom sexp_ -> (error0, 'a1)
  sum

val from_string :
  'a1 deserialize -> string -> (error0, 'a1) sum

val oops : unit -> 'a1

val deserialize_context : context deserialize

val parse_context :
  string -> (error0, context) sum
