From stdpp Require Import
     numbers
     fin_maps
     strings
     option.
Require Import VCalyx.IRSyntax.

Inductive value := 
(* Top: more than 1 assignment to this port has occurred *)
| Z
(* If only 1 assignment has occurred, this value is in port.in *)
| V (val: N)
(* Bottom: no assignment to this port has occurred *)
| X.

(* Maybe we will eventually want the internal states of cells to be
   more complicated than this but for registers this will do just
   fine. *)
Definition state : Type := value.

Section Semantics.
  Context (imap: Type -> Type) `{FinMap ident imap}.
  (* TODO put the computations in here *)
  (* map from cell names to port names to values *)
  Definition val_map : Type := imap value.
  Definition cell_map : Type := imap val_map.
  Definition st : Type := imap state.
  Definition cell_env : Type := imap cell.

  (* Poking an object (whether it is a prim or a cell) asks it to
     recompute its inputs, but does not step the clock or reset
     the values of ports. *)
  Definition poke_prim (prim: ident) (param_binding: list (ident * N)) (ports: imap value) : imap value :=
    (* no op *)
    ports.

  Definition poke_cell (c: cell) (ρ: st) (σ: cell_map) : option cell_map :=
    match c.(cell_proto) with
    | ProtoPrim prim param_binding _ =>
      (* "The function alter f k m should update the value at key k
         using the function f, which is called with the original
         value." docs:
         https://plv.mpi-sws.org/coqdoc/stdpp/stdpp.base.html *)
      mret (alter (poke_prim prim param_binding) c.(cell_name) σ)
    | _ => None (* unimplemented *)
    end.

  Definition read_port (p: port) (σ: cell_map) : option value :=
    match p.(parent) with
    | PCell cell =>
      lookup cell σ ≫= lookup p.(port_name)
    | _ => None
    end.

  Definition write_port (p: port) (v: value) (σ: cell_map) : option cell_map :=
    match p.(parent) with
    | PCell cell =>
      mret (alter (insert p.(port_name) v) cell σ)
    | _ => None
    end.
  
  Definition interp_assign
             (ce: cell_env)
             (ρ: st)
             (op: assignment)
             (σ: cell_map) 
              : option cell_map :=
    match op.(src).(parent) with
    | PCell cell =>
      c ← ce !! cell;
      σ' ← poke_cell c ρ σ;
      v ← read_port op.(src) σ';
      write_port op.(dst) v σ'
    | _ =>
      None
    end.
  
  Definition program : Type :=
    cell_env * list assignment.

  (* The interpreter *)
  Definition interp
             (program: program)
             (σ: env)
             (ρ: st)
    : option env :=
    let (ce, assigns) := program in 
    foldr (fun op res => res ≫= interp_assign ce ρ op)
          (Some σ)
          assigns.
  
End Semantics.
