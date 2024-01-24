From stdpp Require Import
     base
     numbers.
Require Import VCalyx.Value.
Definition value_lift_binop (f: N -> N -> N) : value -> value -> value :=
  fun u v =>
    match u, v with
    | Bot, _
    | _, Bot => Bot
    | V u, V v => V (f u v)
    | Top, _
    | _, Top => Top
    end.

Definition value_lt : value -> value -> value :=
  value_lift_binop (fun n m => bool_to_N (N.ltb n m)).

Definition value_or : value -> value -> value :=
  value_lift_binop N.lor.

Definition value_add : value -> value -> value :=
  value_lift_binop N.add.

Definition value_div_quotient : value -> value -> value :=
  value_lift_binop N.div.

Definition value_div_remainder : value -> value -> value :=
  value_lift_binop (fun a b => (N.div_eucl a b).2).
