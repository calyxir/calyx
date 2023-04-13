Require Import Coq.Classes.EquivDec.
From stdpp Require Import base numbers fin_maps option.
Require Import String.
Require Import VCalyx.IRSyntax.
Local Open Scope string_scope.

Inductive value := 
(* Top: more than 1 assignment to this port has occurred *)
| Z
(* If only 1 assignment has occurred, this value is in port.in *)
| V (val: N)
(* Bottom: no assignment to this port has occurred *)
| X.

Scheme Equality for value.
Check value_eq_dec.
Global Instance value_EqDec : EqDec value eq :=
  value_eq_dec.

Check (Z == Z).
Check (Z ==b Z).

Section Semantics.
Variable (ident_map: Type -> Type).
Context `{FinMap ident ident_map}.
Definition five := 5.
Definition my_emp: ident_map value :=
  empty.

Definition val_map : Type := ident_map value.

Definition prim_map : Type := ident_map (val_map -> option val_map).
Open Scope stdpp_scope.
Definition calyx_prims : prim_map := list_to_map 
 [("std_reg", fun inputs => Some inputs)].
Definition calyx_prims : prim_map := list_to_map 
 [("std_reg", fun inputs =>
    wen ← (inputs !! "std_reg.write_en");
    if wen ==b (V 1%N)
    then v 	← inputs !! "std_reg.in";
         Some (<["std_reg.done" := wen]><["std_reg.out" := v]>inputs)
    else None)]. 
(* TODO add semantic cells *)

(* TODO put the computations in here *)
Definition prim_compute (prim: prim) (inputs: list port_val) : list port_val := 
  match prim.(prim_name) with 
  (* | "std_reg" => 
    let v := List.find (fun p => (eqb p.(port_ref).(port_name) "std_reg.write_en")) inputs in 
    if (eqb v 1%N) then 
    let v' := List.find (fun p => (eqb p.(port_ref).(port_name) "std_reg.in")) inputs in 
    [PortVal {|
      port_ref: List.find (fun p => (eqb p.(port_name) "std_reg.done")) prim.(prim_ports)
      port_value: v'.(port_value)
     |};
     PortVal {|
      port_ref: List.find (fun p => (eqb p.(port_name) "std_reg.out")) prim.(prim_ports)
      port_value: v.(port_value)
     |} 
    ] else [] *)
  | _ => []
  end.

Definition cell_env := list cell.

Definition port_env := cell -> list port_val.

(* Updates cell ports *)
Definition update :
  cell ->
  port_env ->
  list port_val ->
  port_env.
Admitted.

Definition interp_assign :
  cell_env ->
  assignment ->
  port_env ->
  port_env.
Admitted.

Definition program := (cell_env * list assignment)%type.

(* The interpreter *)
Definition interp
  (program: program)
  (pe: port_env)
  : port_env :=
  let (ce, assigns) := program in 
  fold_right (interp_assign ce) pe assigns.

End Semantics.

Check my_emp.