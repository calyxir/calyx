(** * Parsing Calyx Syntax from S-expressions *)
(** This file defines a parser (deserializer) which turns an
    s-expression format for Calyx programs into Calyx ASTs, as defined
    in the Syntax module. We use the [coq-ceres] library. *)

From stdpp Require Import list strings.
From VCalyx Require Import Syntax.
From Ceres Require Import Ceres.

Definition oops : forall {A}, () -> A.
Admitted.

Global Instance Deserialize_direction : Deserialize direction :=
    Deser.match_con "direction"
                    [ ("input", Input);
                      ("output", Output) ]
                    [].

Global Instance Deserialize_port_def : Deserialize port_def :=
  fun _ _ => oops ().

Global Instance Deserialize_prim : Deserialize prim :=
  fun _ _ => oops ().

Global Instance Deserialize_extern : Deserialize extern :=
  fun _ _ => oops ().

Global Instance Deserialize_proto : Deserialize proto :=
  fun _ _ => oops ().

Global Instance Deserialize_cell : Deserialize cell :=
  fun _ _ => oops ().

Global Instance Deserialize_port_ref : Deserialize port_ref :=
  fun _ _ => oops ().

Global Instance Deserialize_num : Deserialize num :=
  fun _ _ => oops ().

Global Instance Deserialize_atom : Deserialize atom :=
  fun _ _ => oops ().

Global Instance Deserialize_guard_cmp : Deserialize guard_cmp :=
    Deser.match_con "guard_cmp"
                    [ ("eq", Eq);
                      ("neq", Neq);
                      ("gt", Gt);
                      ("lt", Lt);
                      ("geq", Geq);
                      ("leq", Leq) ]
                    [].

Global Instance Deserialize_guard_expr : Deserialize guard_expr :=
  fun _ _ => oops ().

Global Instance Deserialize_wire : Deserialize wire :=
  fun _ _ => oops ().

Global Instance Deserialize_control : Deserialize control :=
  fun _ _ => oops ().

Global Instance Deserialize_group : Deserialize group :=
  fun _ _ => oops ().

Global Instance Deserialize_comp : Deserialize comp :=
  fun _ _ => oops ().

Global Instance Deserialize_context : Deserialize context :=
  fun _ _ => oops ().

(* Entry point for the parser *)
Definition parse_context (s: string) : error + context :=
  from_string s.
