From stdpp Require Import
     base
     numbers
     fin_maps
     strings
     option.
Require Import Coq.Classes.EquivDec.
Require Import VCalyx.IRSyntax.

Inductive value := 
(* Top: more than 1 assignment to this port has occurred *)
| Z
(* If only 1 assignment has occurred, this value is in port.in *)
| V (val: N)
(* Bottom: no assignment to this port has occurred *)
| X.
Scheme Equality for value.
Global Instance value_EqDec : EqDec value eq :=
  value_eq_dec.

(* Testing out the eqdec instance *)
Eval compute in (Z == Z).
Eval compute in (Z ==b V 12).

Definition state : Type. (* TODO *)
Admitted.

Section Semantics.
  Context (ident_map: Type -> Type)
          `{FinMap ident ident_map}.
  (* TODO put the computations in here *)
  (* map from cell names to port names to values *)
  Definition val_map : Type := ident_map value.
  Definition cell_map : Type := ident_map val_map.
  Definition state_map : Type := ident_map state.

  Definition cell_env : Type := ident_map cell.
  Definition prim_map : Type := ident_map (val_map -> option val_map).

  Definition five := 5.
  Definition my_emp: ident_map value :=
    empty.

  Open Scope stdpp_scope.
  Definition calyx_prims : prim_map :=
    list_to_map 
      [("std_reg",
         fun inputs =>
           wen ← (inputs !! "std_reg.write_en");
           if wen ==b (V 1%N)
           then v ← inputs !! "std_reg.in";
                Some (<["std_reg.done" := wen]>(<["std_reg.out" := v]>inputs))
           else None)]. 
  
(* TODO put the computations in here *)
  Definition poke_prim (prim: ident) (param_binding: list (ident * N)) (inputs: val_map) : option val_map := 
    fn ← calyx_prims !! prim;
    fn inputs.
  
  Definition poke_cell (c: cell) (ρ: state_map) (σ: cell_map) : option cell_map :=
    match c.(cell_proto) with
    | ProtoPrim prim param_binding _ =>
        old ← σ !! c.(cell_name);
        new ← poke_prim prim param_binding old;
        Some (<[c.(cell_name) := new]>σ)
    | _ => None (* unimplemented *)
    end.

  Definition poke_all_cells (ce: cell_env) (ρ: state_map) (σ: cell_map) : option cell_map :=
    map_fold (fun _ cell σ_opt =>
                σ ← σ_opt;
                poke_cell cell ρ σ)
             (Some σ)
             ce.

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
             (ρ: state_map)
             (op: assignment)
             (σ: cell_map) 
              : option cell_map :=
    match op.(src).(parent) with
    | PCell cell =>
      c ← ce !! cell;
      σ' ← poke_all_cells ce ρ σ;
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
             (σ: cell_map)
             (ρ: state_map)
    : option cell_map :=
    let (ce, assigns) := program in 
    foldr (fun op res => res ≫= interp_assign ce ρ op)
          (Some σ)
          assigns.
  
End Semantics.
