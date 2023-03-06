From stdpp Require Import numbers.
Require Import VCalyx.IRSyntax.

Inductive value := 
  (* Top: more than 1 assignment to this port has occurred *)
  | Z
  (* If only 1 assignment has occurred, this value is in port.in *)
  | V (val: N)
  (* Bottom: no assignment to this port has occurred *)
  | X.

Record port_val := 
  PortVal {
    port_ref: port;
    port_value: value
  }.

(* TODO put the computations in here *)
Definition prim_compute (prim: prim) (inputs: list port_val) : list port_val := 
  match prim.(prim_name) with 
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
