From stdpp Require Import
     base
     strings
     numbers.
Require Import Coq.Classes.EquivDec.
Require Import VCalyx.Exception.

Inductive value := 
(* Top: more than 1 assignment to this port has occurred *)
| Top 
(* If only 1 assignment has occurred, this value is in port.in *)
| V (val: N)
(* Bottom: no assignment to this port has occurred *)
| Bot.
Scheme Equality for value.
#[export] Instance value_EqDec : EqDec value eq :=
  value_eq_dec.

Definition expect_V (v: value) : exn N :=
  match v with
  | V val => mret val
  | _ => err "expect_V"
  end.

Definition is_one (v: value) :=
  v ==b (V 1%N).

Definition is_nonzero (v: value) :=
  v <>b (V 0%N).

Definition bool_to_N (b: bool) : N :=
  if b then 1%N else 0%N.

Definition bool_to_value (b: bool) : value :=
  V (bool_to_N b).
