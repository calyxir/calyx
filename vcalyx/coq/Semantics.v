From stdpp Require Import
     base
     numbers
     list
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

Inductive numtype :=
| Bitnum
| FixedPoint.
    
Record mem_fmt := { is_signed: bool;
                    numeric_type: numtype;
                    width: nat; }.

Definition mem_data := list N.

Inductive state : Type :=
(* std_reg *)
| StateReg (write_done: bool) (val: value)
(* std_mem *)
| StateMem (fmt: mem_fmt) (contents: mem_data).

Definition get_reg_state (st: state) :=
  match st with
  | StateReg write_done v => Some (write_done, v)
  | _ => None
  end.

Definition get_mem_state (st: state) :=
  match st with
  | StateMem fmt contents => Some (fmt, contents)
  | _ => None
  end.

Section Semantics.
  (* ENVIRONMENTS AND STORES *)
  (* Definitions of types of finite maps used in the semantics. *)
  Context (ident_map: Type -> Type)
          `{FinMap ident ident_map}.
  (* TODO put the computations in here *)
  (* map from port names to values *)
  Definition val_map : Type := ident_map value.
  (* map from cell names to port names to values *)
  Definition cell_map : Type := ident_map val_map.
  (* map from cell name to state *)
  Definition state_map : Type := ident_map state.

  (* environment collecting all defined cells *)
  Definition cell_env : Type := ident_map cell.
  (* An environment collecting all defined primitives *)
  Definition prim_map : Type := ident_map (state -> val_map -> option (val_map)).

  (* An environment collecting all defined groups *)
  Definition group_env : Type := ident_map group.
  (* map from group name to values of its ports *)
  Definition group_map : Type := ident_map val_map.

  Open Scope stdpp_scope.
  Definition calyx_prims : prim_map :=
    list_to_map 
      [("std_reg",
         fun st inputs =>
           '(_, v) ← get_reg_state st;
           Some (<["out" := v]>inputs))].
  
  Definition poke_prim (prim: ident) (param_binding: list (ident * N)) (st: state) (inputs: val_map) : option val_map := 
    fn ← calyx_prims !! prim;
    fn st inputs.
  
  Definition poke_cell (c: cell) (ρ: state_map) (σ: cell_map) : option (cell_map) :=
    match c.(cell_proto) with
    | ProtoPrim prim param_binding _ =>
        st ← ρ !! c.(cell_name);
        vs ← σ !! c.(cell_name);
        vs' ← poke_prim prim param_binding st vs;
        Some (<[c.(cell_name) := vs']>σ)
    | _ => None (* unimplemented *)
    end.

  Definition poke_all_cells (ce: cell_env) (ρ: state_map) (σ: cell_map) : option cell_map :=
    map_fold (fun _ cell ρσ_opt =>
                σ ← ρσ_opt;
                poke_cell cell ρ σ)
             (Some σ)
             ce.

  Definition read_port (p: port) (σ: cell_map) : option value :=
    lookup p.(parent) σ ≫= lookup p.(port_name).

  Definition read_port_ref (p: port_ref) (σ: cell_map) : option value :=
    match p with
    | PComp comp port => lookup comp σ ≫= lookup port
    | _ => None (* TODO *)
    end.

  Definition write_port (p: port) (v: value) (σ: cell_map) : option cell_map :=
    mret (alter (insert p.(port_name) v) p.(parent) σ).

  Definition write_port_ref (p: port_ref) (v: value) (σ: cell_map) : option cell_map :=
    match p with
    | PComp comp port => mret (alter (insert port v) comp σ)
    | _ => None (* TODO *)
    end.
  
  Definition interp_assign
             (ce: cell_env)
             (ρ: state_map)
             (op: assignment)
             (σ: cell_map) 
    : option cell_map :=
    σ' ← poke_all_cells ce ρ σ;
    v ← read_port_ref op.(src) σ';
    σ'' ← write_port_ref op.(dst) v σ';
    mret σ''.

  Definition interp_group ce σ ρ (g: group) : option cell_map := 
    (* there is probably a monad sequencing operation that should be used here *)
    foldr (fun op res => res ≫= interp_assign ce ρ op)
          (Some σ)
          g.(group_assns).

  Definition is_entrypoint (entrypoint: ident) (c: comp) : bool :=
    bool_decide (entrypoint = c.(comp_name)).

  Definition load_group (g: group) (ge: group_env) : group_env :=
    <[g.(group_name) := g]>ge.

  Definition load_groups (ge: group_env) (c: comp) :=
    foldr load_group ge c.(comp_groups).

  Definition load_cell (c: cell) (ce: cell_env) : cell_env :=
    <[c.(cell_name) := c]>ce.

  Definition load_cells (ce: cell_env) (c: comp) :=
    foldr load_cell ce c.(comp_cells).

  Definition load_comp (c: comp) : cell_env * group_env -> cell_env * group_env :=
    fun '(ce, ge) =>
      (load_cells ce c, load_groups ge c).

  Definition load_context (c: context) : cell_env * group_env := 
    foldr load_comp (empty, empty) c.(ctx_comps).

  Definition interp_control (ce: cell_env) (ge: group_env) σ ρ ctrl :=
    match ctrl with
    | CEnable group _ =>
        g ← ge !! group;
        interp_group ce σ ρ g
    | _ => None
    end.

  Definition allocate_cell_map (ce: cell_env) : cell_map.
  Admitted.

  Definition allocate_state_map (ce: cell_env) : state_map.
  Admitted.
  
  Definition interp_context (c: context) :=
    main ← List.find (is_entrypoint c.(ctx_entrypoint)) c.(ctx_comps);
    let '(ce, ge) := load_context c in
    let σ := allocate_cell_map ce in
    let ρ := allocate_state_map ce in
    interp_control ce ge σ ρ main.(comp_control).
End Semantics.
