
(** val option_map :
    ('a1 -> 'a2) -> 'a1 option -> 'a2 option **)

let option_map f = function
| Some a -> Some (f a)
| None -> None

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

(** val eqb : bool -> bool -> bool **)

let eqb b1 b2 =
  if b1 then b2 else if b2 then false else true

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

module Pos =
 struct
  (** val succ : positive -> positive **)

  let rec succ = function
  | XI p -> XO (succ p)
  | XO p -> XI p
  | XH -> XO XH

  (** val add :
      positive -> positive -> positive **)

  let rec add x y =
    match x with
    | XI p ->
      (match y with
       | XI q -> XO (add_carry p q)
       | XO q -> XI (add p q)
       | XH -> XO (succ p))
    | XO p ->
      (match y with
       | XI q -> XI (add p q)
       | XO q -> XO (add p q)
       | XH -> XI p)
    | XH ->
      (match y with
       | XI q -> XO (succ q)
       | XO q -> XI q
       | XH -> XO XH)

  (** val add_carry :
      positive -> positive -> positive **)

  and add_carry x y =
    match x with
    | XI p ->
      (match y with
       | XI q -> XI (add_carry p q)
       | XO q -> XO (add_carry p q)
       | XH -> XI (succ p))
    | XO p ->
      (match y with
       | XI q -> XO (add_carry p q)
       | XO q -> XI (add p q)
       | XH -> XO (succ p))
    | XH ->
      (match y with
       | XI q -> XI (succ q)
       | XO q -> XO (succ q)
       | XH -> XI XH)

  (** val mul :
      positive -> positive -> positive **)

  let rec mul x y =
    match x with
    | XI p -> add y (XO (mul p y))
    | XO p -> XO (mul p y)
    | XH -> y

  (** val of_uint_acc :
      uint -> positive -> positive **)

  let rec of_uint_acc d acc =
    match d with
    | Nil -> acc
    | D0 l ->
      of_uint_acc l (mul (XO (XI (XO XH))) acc)
    | D1 l ->
      of_uint_acc l
        (add XH (mul (XO (XI (XO XH))) acc))
    | D2 l ->
      of_uint_acc l
        (add (XO XH) (mul (XO (XI (XO XH))) acc))
    | D3 l ->
      of_uint_acc l
        (add (XI XH) (mul (XO (XI (XO XH))) acc))
    | D4 l ->
      of_uint_acc l
        (add (XO (XO XH))
          (mul (XO (XI (XO XH))) acc))
    | D5 l ->
      of_uint_acc l
        (add (XI (XO XH))
          (mul (XO (XI (XO XH))) acc))
    | D6 l ->
      of_uint_acc l
        (add (XO (XI XH))
          (mul (XO (XI (XO XH))) acc))
    | D7 l ->
      of_uint_acc l
        (add (XI (XI XH))
          (mul (XO (XI (XO XH))) acc))
    | D8 l ->
      of_uint_acc l
        (add (XO (XO (XO XH)))
          (mul (XO (XI (XO XH))) acc))
    | D9 l ->
      of_uint_acc l
        (add (XI (XO (XO XH)))
          (mul (XO (XI (XO XH))) acc))

  (** val of_uint : uint -> n **)

  let rec of_uint = function
  | Nil -> N0
  | D0 l -> of_uint l
  | D1 l -> Npos (of_uint_acc l XH)
  | D2 l -> Npos (of_uint_acc l (XO XH))
  | D3 l -> Npos (of_uint_acc l (XI XH))
  | D4 l -> Npos (of_uint_acc l (XO (XO XH)))
  | D5 l -> Npos (of_uint_acc l (XI (XO XH)))
  | D6 l -> Npos (of_uint_acc l (XO (XI XH)))
  | D7 l -> Npos (of_uint_acc l (XI (XI XH)))
  | D8 l ->
    Npos (of_uint_acc l (XO (XO (XO XH))))
  | D9 l ->
    Npos (of_uint_acc l (XI (XO (XO XH))))
 end

module N =
 struct
  (** val succ : n -> n **)

  let succ = function
  | N0 -> Npos XH
  | Npos p -> Npos (Pos.succ p)
 end

(** val rev_append :
    'a1 list -> 'a1 list -> 'a1 list **)

let rec rev_append l l' =
  match l with
  | [] -> l'
  | a :: l0 -> rev_append l0 (a :: l')

(** val rev' : 'a1 list -> 'a1 list **)

let rev' l =
  rev_append l []

module Z =
 struct
  (** val opp : z -> z **)

  let opp = function
  | Z0 -> Z0
  | Zpos x0 -> Zneg x0
  | Zneg x0 -> Zpos x0

  (** val of_N : n -> z **)

  let of_N = function
  | N0 -> Z0
  | Npos p -> Zpos p

  (** val of_uint : uint -> z **)

  let of_uint d =
    of_N (Pos.of_uint d)

  (** val of_int : signed_int -> z **)

  let of_int = function
  | Pos d0 -> of_uint d0
  | Neg d0 -> opp (of_uint d0)
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
              cell_proto : proto; 
              cell_ref : bool }

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

type comp = { comp_attrs : attributes;
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

(** val uint_of_char :
    char -> uint option -> uint option **)

let uint_of_char a = function
| Some d0 ->
  (* If this appears, you're using Ascii internals. Please don't *)
 (fun f c ->
  let n = Char.code c in
  let h i = (n land (1 lsl i)) <> 0 in
  f (h 0) (h 1) (h 2) (h 3) (h 4) (h 5) (h 6) (h 7))
    (fun b b0 b1 b2 b3 b4 b5 b6 ->
    if b
    then if b0
         then if b1
              then if b2
                   then None
                   else if b3
                        then if b4
                             then if b5
                                  then None
                                  else if b6
                                       then None
                                       else 
                                       Some (D7
                                       d0)
                             else None
                        else None
              else if b2
                   then None
                   else if b3
                        then if b4
                             then if b5
                                  then None
                                  else if b6
                                       then None
                                       else 
                                       Some (D3
                                       d0)
                             else None
                        else None
         else if b1
              then if b2
                   then None
                   else if b3
                        then if b4
                             then if b5
                                  then None
                                  else if b6
                                       then None
                                       else 
                                       Some (D5
                                       d0)
                             else None
                        else None
              else if b2
                   then if b3
                        then if b4
                             then if b5
                                  then None
                                  else if b6
                                       then None
                                       else 
                                       Some (D9
                                       d0)
                             else None
                        else None
                   else if b3
                        then if b4
                             then if b5
                                  then None
                                  else if b6
                                       then None
                                       else 
                                       Some (D1
                                       d0)
                             else None
                        else None
    else if b0
         then if b1
              then if b2
                   then None
                   else if b3
                        then if b4
                             then if b5
                                  then None
                                  else if b6
                                       then None
                                       else 
                                       Some (D6
                                       d0)
                             else None
                        else None
              else if b2
                   then None
                   else if b3
                        then if b4
                             then if b5
                                  then None
                                  else if b6
                                       then None
                                       else 
                                       Some (D2
                                       d0)
                             else None
                        else None
         else if b1
              then if b2
                   then None
                   else if b3
                        then if b4
                             then if b5
                                  then None
                                  else if b6
                                       then None
                                       else 
                                       Some (D4
                                       d0)
                             else None
                        else None
              else if b2
                   then if b3
                        then if b4
                             then if b5
                                  then None
                                  else if b6
                                       then None
                                       else 
                                       Some (D8
                                       d0)
                             else None
                        else None
                   else if b3
                        then if b4
                             then if b5
                                  then None
                                  else if b6
                                       then None
                                       else 
                                       Some (D0
                                       d0)
                             else None
                        else None)
    a
| None -> None

module NilEmpty =
 struct
  (** val uint_of_string :
      string -> uint option **)

  let rec uint_of_string s =
    (* If this appears, you're using String internals. Please don't *)
 (fun f0 f1 s ->
    let l = String.length s in
    if l = 0 then f0 () else f1 (String.get s 0) (String.sub s 1 (l-1)))

      (fun _ -> Some Nil)
      (fun a s0 ->
      uint_of_char a (uint_of_string s0))
      s
 end

module NilZero =
 struct
  (** val uint_of_string :
      string -> uint option **)

  let uint_of_string s =
    (* If this appears, you're using String internals. Please don't *)
 (fun f0 f1 s ->
    let l = String.length s in
    if l = 0 then f0 () else f1 (String.get s 0) (String.sub s 1 (l-1)))

      (fun _ -> None)
      (fun _ _ -> NilEmpty.uint_of_string s)
      s

  (** val int_of_string :
      string -> signed_int option **)

  let int_of_string s =
    (* If this appears, you're using String internals. Please don't *)
 (fun f0 f1 s ->
    let l = String.length s in
    if l = 0 then f0 () else f1 (String.get s 0) (String.sub s 1 (l-1)))

      (fun _ -> None)
      (fun a s' ->
      if (=) a '-'
      then option_map (fun x -> Neg x)
             (uint_of_string s')
      else option_map (fun x -> Pos x)
             (uint_of_string s))
      s
 end

(** val compcomp :
    comparison -> comparison -> comparison **)

let compcomp x y =
  match x with
  | Eq -> y
  | x0 -> x0

(** val compb : bool -> bool -> comparison **)

let compb x y =
  if x
  then if y then Eq else Gt
  else if y then Lt else Eq

(** val eqb_ascii : char -> char -> bool **)

let eqb_ascii a b =
  (* If this appears, you're using Ascii internals. Please don't *)
 (fun f c ->
  let n = Char.code c in
  let h i = (n land (1 lsl i)) <> 0 in
  f (h 0) (h 1) (h 2) (h 3) (h 4) (h 5) (h 6) (h 7))
    (fun a0 a1 a2 a3 a4 a5 a6 a7 ->
    (* If this appears, you're using Ascii internals. Please don't *)
 (fun f c ->
  let n = Char.code c in
  let h i = (n land (1 lsl i)) <> 0 in
  f (h 0) (h 1) (h 2) (h 3) (h 4) (h 5) (h 6) (h 7))
      (fun b0 b1 b2 b3 b4 b5 b6 b7 ->
      if if if if if if if eqb a0 b0
                        then eqb a1 b1
                        else false
                     then eqb a2 b2
                     else false
                  then eqb a3 b3
                  else false
               then eqb a4 b4
               else false
            then eqb a5 b5
            else false
         then eqb a6 b6
         else false
      then eqb a7 b7
      else false)
      b)
    a

(** val ascii_compare :
    char -> char -> comparison **)

let ascii_compare a b =
  (* If this appears, you're using Ascii internals. Please don't *)
 (fun f c ->
  let n = Char.code c in
  let h i = (n land (1 lsl i)) <> 0 in
  f (h 0) (h 1) (h 2) (h 3) (h 4) (h 5) (h 6) (h 7))
    (fun a0 a1 a2 a3 a4 a5 a6 a7 ->
    (* If this appears, you're using Ascii internals. Please don't *)
 (fun f c ->
  let n = Char.code c in
  let h i = (n land (1 lsl i)) <> 0 in
  f (h 0) (h 1) (h 2) (h 3) (h 4) (h 5) (h 6) (h 7))
      (fun b0 b1 b2 b3 b4 b5 b6 b7 ->
      compcomp (compb a7 b7)
        (compcomp (compb a6 b6)
          (compcomp (compb a5 b5)
            (compcomp (compb a4 b4)
              (compcomp (compb a3 b3)
                (compcomp (compb a2 b2)
                  (compcomp (compb a1 b1)
                    (compb a0 b0))))))))
      b)
    a

(** val leb_ascii : char -> char -> bool **)

let leb_ascii a b =
  match ascii_compare a b with
  | Gt -> false
  | _ -> true

(** val string_elem : char -> string -> bool **)

let rec string_elem c s =
  (* If this appears, you're using String internals. Please don't *)
 (fun f0 f1 s ->
    let l = String.length s in
    if l = 0 then f0 () else f1 (String.get s 0) (String.sub s 1 (l-1)))

    (fun _ -> false)
    (fun c' s0 ->
    if eqb_ascii c c'
    then true
    else string_elem c s0)
    s

(** val _string_reverse :
    string -> string -> string **)

let rec _string_reverse r s =
  (* If this appears, you're using String internals. Please don't *)
 (fun f0 f1 s ->
    let l = String.length s in
    if l = 0 then f0 () else f1 (String.get s 0) (String.sub s 1 (l-1)))

    (fun _ -> r)
    (fun c s0 ->
    _string_reverse
      ((* If this appears, you're using String internals. Please don't *)
  (fun (c, s) -> String.make 1 c ^ s)

      (c, r)) s0)
    s

(** val string_reverse : string -> string **)

let string_reverse =
  _string_reverse ""

(** val is_printable : char -> bool **)

let is_printable c =
  (&&) (leb_ascii ' ' c) (leb_ascii c '~')

(** val is_whitespace : char -> bool **)

let is_whitespace c =
  (* If this appears, you're using Ascii internals. Please don't *)
 (fun f c ->
  let n = Char.code c in
  let h i = (n land (1 lsl i)) <> 0 in
  f (h 0) (h 1) (h 2) (h 3) (h 4) (h 5) (h 6) (h 7))
    (fun b b0 b1 b2 b3 b4 b5 b6 ->
    if b
    then if b0
         then false
         else if b1
              then if b2
                   then if b3
                        then false
                        else if b4
                             then false
                             else if b5
                                  then false
                                  else if b6
                                       then false
                                       else true
                   else false
              else false
    else if b0
         then if b1
              then false
              else if b2
                   then if b3
                        then false
                        else if b4
                             then false
                             else if b5
                                  then false
                                  else if b6
                                       then false
                                       else true
                   else false
         else if b1
              then false
              else if b2
                   then false
                   else if b3
                        then false
                        else if b4
                             then if b5
                                  then false
                                  else if b6
                                       then false
                                       else true
                             else false)
    c

(** val is_digit : char -> bool **)

let is_digit c =
  if leb_ascii '0' c
  then leb_ascii c '9'
  else false

(** val is_upper : char -> bool **)

let is_upper c =
  if leb_ascii 'A' c
  then leb_ascii c 'Z'
  else false

(** val is_lower : char -> bool **)

let is_lower c =
  if leb_ascii 'a' c
  then leb_ascii c 'z'
  else false

(** val is_alphanum : char -> bool **)

let is_alphanum c =
  if if is_upper c then true else is_lower c
  then true
  else is_digit c

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

(** val is_atom_char : char -> bool **)

let is_atom_char c =
  if is_alphanum c
  then true
  else string_elem c "'=-+*/:!?@#$%^&_<>.,|~"

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

(** val set_cur_token :
    'a1 parser_state_ -> 'a2 -> 'a2 parser_state_ **)

let set_cur_token i u =
  { parser_done = i.parser_done; parser_stack =
    i.parser_stack; parser_cur_token = u }

type parser_state = partial_token parser_state_

(** val initial_state : parser_state **)

let initial_state =
  { parser_done = []; parser_stack = [];
    parser_cur_token = NoToken }

(** val new_sexp :
    atom sexp_ list -> symbol list -> atom sexp_
    -> 'a1 -> 'a1 parser_state_ **)

let new_sexp d s e t =
  match s with
  | [] ->
    { parser_done = (e :: d); parser_stack = [];
      parser_cur_token = t }
  | _ :: _ ->
    { parser_done = d; parser_stack = ((Exp
      e) :: s); parser_cur_token = t }

(** val next_str :
    parser_state -> loc -> string -> escape ->
    loc -> char -> (error, parser_state) sum **)

let next_str i p0 tok e p c =
  let { parser_done = d; parser_stack = s;
    parser_cur_token = _ } = i
  in
  let ret = fun tok' e' -> Inr { parser_done =
    d; parser_stack = s; parser_cur_token =
    (StrToken (p0, tok', e')) }
  in
  (match e with
   | EscBackslash ->
     if eqb_ascii 'n' c
     then ret
            ((* If this appears, you're using String internals. Please don't *)
  (fun (c, s) -> String.make 1 c ^ s)

            ('\n', tok)) EscNone
     else if eqb_ascii '\\' c
          then ret
                 ((* If this appears, you're using String internals. Please don't *)
  (fun (c, s) -> String.make 1 c ^ s)

                 ('\\', tok)) EscNone
          else if eqb_ascii '"' c
               then ret
                      ((* If this appears, you're using String internals. Please don't *)
  (fun (c, s) -> String.make 1 c ^ s)

                      ('"', tok)) EscNone
               else Inl (UnknownEscape (p, c))
   | EscNone ->
     if eqb_ascii '\\' c
     then ret tok EscBackslash
     else if eqb_ascii '"' c
          then Inr
                 (new_sexp d s (Atom_ (Str
                   (string_reverse tok)))
                   NoToken)
          else if is_printable c
               then ret
                      ((* If this appears, you're using String internals. Please don't *)
  (fun (c, s) -> String.make 1 c ^ s)

                      (c, tok)) EscNone
               else Inl (InvalidStringChar (c,
                      p)))

(** val _fold_stack :
    atom sexp_ list -> loc -> atom sexp_ list ->
    symbol list -> (error, parser_state) sum **)

let rec _fold_stack d p r = function
| [] -> Inl (UnmatchedClose p)
| s0 :: s1 ->
  (match s0 with
   | Open _ ->
     Inr (new_sexp d s1 (List r) NoToken)
   | Exp e -> _fold_stack d p (e :: r) s1)

(** val next' :
    'a1 parser_state_ -> loc -> char -> (error,
    parser_state) sum **)

let next' i p c =
  if eqb_ascii '(' c
  then Inr { parser_done = i.parser_done;
         parser_stack = ((Open
         p) :: i.parser_stack);
         parser_cur_token = NoToken }
  else if eqb_ascii ')' c
       then _fold_stack i.parser_done p []
              i.parser_stack
       else if eqb_ascii '"' c
            then Inr
                   (set_cur_token i (StrToken
                     (p, "", EscNone)))
            else if eqb_ascii ';' c
                 then Inr
                        (set_cur_token i Comment)
                 else if is_whitespace c
                      then Inr
                             (set_cur_token i
                               NoToken)
                      else Inl (InvalidChar (c,
                             p))

(** val next_comment :
    parser_state -> char -> (error,
    parser_state) sum **)

let next_comment i c =
  if eqb_ascii '\n' c
  then Inr { parser_done = i.parser_done;
         parser_stack = i.parser_stack;
         parser_cur_token = NoToken }
  else Inr i

(** val raw_or_num : string -> atom **)

let raw_or_num s =
  let s0 = string_reverse s in
  (match NilZero.int_of_string s0 with
   | Some n0 -> Num (Z.of_int n0)
   | None -> Raw s0)

(** val next :
    parser_state -> loc -> char -> (error,
    parser_state) sum **)

let next i p c =
  match i.parser_cur_token with
  | NoToken ->
    if is_atom_char c
    then Inr
           (set_cur_token i (SimpleToken (p,
             ((* If this appears, you're using String internals. Please don't *)
  (fun (c, s) -> String.make 1 c ^ s)

             (c, "")))))
    else next' i p c
  | SimpleToken (_, tok) ->
    if is_atom_char c
    then Inr
           (set_cur_token i (SimpleToken (p,
             ((* If this appears, you're using String internals. Please don't *)
  (fun (c, s) -> String.make 1 c ^ s)

             (c, tok)))))
    else let i' =
           new_sexp i.parser_done i.parser_stack
             (Atom_ (raw_or_num tok)) ()
         in
         next' i' p c
  | StrToken (p0, tok, e) ->
    next_str i p0 tok e p c
  | Comment -> next_comment i c

(** val _done_or_fail :
    atom sexp_ list -> symbol list -> (error,
    atom sexp_ list) sum **)

let rec _done_or_fail r = function
| [] -> Inr (rev' r)
| s0 :: s1 ->
  (match s0 with
   | Open p -> Inl (UnmatchedOpen p)
   | Exp _ -> _done_or_fail r s1)

(** val eof :
    parser_state -> loc -> (error, atom sexp_
    list) sum **)

let eof i _ =
  match i.parser_cur_token with
  | SimpleToken (_, tok) ->
    let i0 =
      new_sexp i.parser_done i.parser_stack
        (Atom_ (raw_or_num tok)) ()
    in
    _done_or_fail i0.parser_done i0.parser_stack
  | StrToken (p0, _, _) ->
    Inl (UnterminatedString p0)
  | _ ->
    _done_or_fail i.parser_done i.parser_stack

(** val parse_sexps_ :
    parser_state -> loc -> string -> (error
    option * loc) * parser_state **)

let rec parse_sexps_ i p s =
  (* If this appears, you're using String internals. Please don't *)
 (fun f0 f1 s ->
    let l = String.length s in
    if l = 0 then f0 () else f1 (String.get s 0) (String.sub s 1 (l-1)))

    (fun _ -> ((None, p), i))
    (fun c s0 ->
    match next i p c with
    | Inl e -> (((Some e), p), i)
    | Inr i0 -> parse_sexps_ i0 (N.succ p) s0)
    s

(** val parse_sexp :
    string -> (error, atom sexp_) sum **)

let parse_sexp s =
  let (p0, i) = parse_sexps_ initial_state N0 s
  in
  let (e, p) = p0 in
  (match rev' i.parser_done with
   | [] ->
     (match e with
      | Some e0 -> Inl e0
      | None ->
        (match eof i p with
         | Inl e0 -> Inl e0
         | Inr l ->
           (match l with
            | [] -> Inl EmptyInput
            | r :: _ -> Inr r)))
   | r :: _ -> Inr r)

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

(** val _from_sexp :
    'a1 deserialize -> 'a1 fromSexp **)

let _from_sexp deserialize0 =
  deserialize0

(** val from_sexp :
    'a1 deserialize -> atom sexp_ -> (error0,
    'a1) sum **)

let from_sexp h =
  _from_sexp h []

(** val from_string :
    'a1 deserialize -> string -> (error0, 'a1)
    sum **)

let from_string h s =
  match parse_sexp s with
  | Inl e -> Inl (ParseError e)
  | Inr x -> from_sexp h x

(** val oops : unit -> 'a1 **)

let oops = (fun _ -> failwith "oops!")

(** val deserialize_context :
    context deserialize **)

let deserialize_context _ _ =
  oops ()

(** val parse_context :
    string -> (error0, context) sum **)

let parse_context s =
  from_string deserialize_context s
