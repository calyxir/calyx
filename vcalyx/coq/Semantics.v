From stdpp Require Import numbers.
Require Import VCalyx.Syntax.
Require Import VCalyx.Vect.

Inductive value := 
  (* Top: more than 1 assignment to this port has occurred *)
  | Z
  (* If only 1 assignment has occurred, this value is in port.in *)
  | V (val: N)
  (* Bottom: no assignment to this port has occurred *)
  | X.

Record port := 
  Port {
    port_id: ident;
    port_width: nat
  }.

Record port_val := 
  PortVal {
    port_name: ident;
    port_value: nat
  }.

(* Internal state as well as the parameters when initialized *)
Record cell := 
  Cell {
    cell_name: ident;
    width: nat;
    in_ports: list port;
    out_ports: list port;
    (* The function that computes the operation done by the component *)
    compute: list port_val -> list port_val
  }.

Inductive expr := 
| Val (v: value)
| PortExp (loc: cell * port)
(* TODO make op + arg type *)
(* arg type looks like vec (ar 0) expr *)
| Op (o: unit) (args: unit).

(* https://docs.calyxir.org/lang/ref.html?highlight=guard#guards *)
Inductive guard_exp := 
| True
| False 
(* if the guard is an expr like reg0.out && reg1.out *)
| Def (loc: expr).

Record assignment := 
  Assign {
    lval: cell * ident;
    rval: cell * ident; 
    assign_guard: guard_exp
  }.

Definition cell_env := list cell.

Definition port_env := cell -> list port_val.

(* Updates cell ports *)
Definition update : cell -> port_env -> list port_val -> port_env.
Admitted.

(* The interpreter *)
Definition interp : cell_env -> port_env -> list assignment -> port_env.
Admitted.
