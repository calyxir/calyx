From stdpp Require Import
     base
     fin_maps
     numbers
     list
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

Definition expect_V (v: value) : option N :=
  match v with
  | V val => Some val
  | _ => None
  end.

Definition is_high (v: value) :=
  v ==b (V 1%N).

Definition bool_to_value (b: bool) : value :=
  if b then V 1%N else V 0%N.

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
(* std_mem_d1 *)
| StateMemD1 (write_done: bool) (fmt: mem_fmt) (contents: mem_data).

Definition get_reg_state (st: state) :=
  match st with
  | StateReg write_done v => Some (write_done, v)
  | _ => None
  end.

Definition get_mem_d1_state (st: state) :=
  match st with
  | StateMemD1 write_done fmt contents => Some (write_done, fmt, contents)
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

  Record prim_sem :=
    { prim_sem_poke: state -> val_map -> option val_map;
      prim_sem_tick: state -> val_map -> option state }.
  
  (* An environment collecting all defined primitives *)
  Definition prim_map : Type := ident_map prim_sem.

  (* An environment collecting all defined groups *)
  Definition group_env : Type := ident_map group.
  (* map from group name to values of its holes *)
  Definition group_map : Type := ident_map val_map.

  Open Scope stdpp_scope.
  Definition calyx_prims : prim_map :=
    list_to_map 
      [("std_reg",
         {| prim_sem_poke st inputs :=
              '(write_done, v) ← get_reg_state st;
              Some (<["done" := bool_to_value write_done]>(<["out" := v]>inputs));
            prim_sem_tick st inputs :=
              '(_, val_old) ← get_reg_state st;
              write_en ← inputs !! "write_en";
              if is_high write_en
              then val_in ← inputs !! "in";
                   Some (StateReg true val_in)
              else Some (StateReg false val_old)
         |});
       ("std_mem_d1",
         {| prim_sem_poke st inputs :=
              '(write_done, fmt, contents) ← get_mem_d1_state st;
              addr ← inputs !! "addr0";
              mem_val ← match addr with
                        | Z => Some Z
                        | V idx => V <$> contents !! (N.to_nat idx)
                        | X => Some X
                        end;
              Some (<["done" := bool_to_value write_done]>(<["read_data" := mem_val]>inputs));
           prim_sem_tick st inputs :=
              '(_, fmt, contents) ← get_mem_d1_state st;
              write_en ← inputs !! "write_en";
              if is_high write_en
              then val_in ← inputs !! "write_data";
                   val ← expect_V val_in;
                   addr ← inputs !! "addr0";
                   idx ← expect_V addr;
                   Some (StateMemD1 true fmt (<[N.to_nat idx := val]>contents))
              else Some (StateMemD1 false fmt contents)
         |})].
  
  Definition is_entrypoint (entrypoint: ident) (c: comp) : bool :=
    bool_decide (entrypoint = c.(comp_name)).

  (* LOADING AND ALLOCATION *)
  Definition load_group (ge: group_env) (g: group) : group_env :=
    <[g.(group_name) := g]>ge.

  Definition load_groups (ge: group_env) (c: comp) :=
    foldl load_group ge c.(comp_groups).

  Definition load_cell (ce: cell_env) (c: cell) : cell_env :=
    <[c.(cell_name) := c]>ce.

  Definition load_cells (ce: cell_env) (c: comp) :=
    foldl load_cell ce c.(comp_cells).

  Definition load_comp : cell_env * group_env -> comp -> cell_env * group_env :=
    fun '(ce, ge) (c: comp) =>
      (load_cells ce c, load_groups ge c).

  Definition load_context (c: context) : cell_env * group_env := 
    foldl load_comp (empty, empty) c.(ctx_comps).

  Definition allocate_val_map (c: cell) : val_map :=
    foldl (fun σ p => <[p.(port_name) := X]>σ)
          empty
          (c.(cell_in_ports) ++ c.(cell_out_ports)).

  Definition allocate_cell_map (ce: cell_env) : cell_map :=
    fmap allocate_val_map ce.

  (* Initialize go and done holes to undef *)
  Definition allocate_group_map (ge: group_env) : group_map :=
    fmap (fun (g: group) => <["go" := X]>(<["done" := X]>empty)) ge.

  Definition prim_initial_state (name: ident) : option state :=
    if decide (name = "std_reg")
    then Some (StateReg false X)
    else None.

  Definition allocate_state_for_cell (c: cell) (ρ: state_map) : option state_map :=
    match c.(cell_proto) with
    | ProtoPrim prim_name bindings is_comb =>
        st ← prim_initial_state prim_name;
        mret (<[c.(cell_name) := st]>ρ)
    | _ => None
    end.

  Definition allocate_state_map (ce: cell_env) : option state_map :=
    map_fold (fun name (c: cell) (ρ0: option state_map) =>
                ρ ← ρ0;
                allocate_state_for_cell c ρ) (Some empty) ce.

  (* COMBINATIONAL UPDATES *)
  Definition poke_prim (prim: ident) (param_binding: list (ident * N)) (st: state) (inputs: val_map) : option val_map := 
    fns ← calyx_prims !! prim;
    fns.(prim_sem_poke) st inputs.
  
  Definition poke_cell (c: cell) (ρ: state_map) (σ: cell_map) : option cell_map :=
    match c.(cell_proto) with
    | ProtoPrim prim param_binding _ =>
        st ← ρ !! c.(cell_name);
        vs ← σ !! c.(cell_name);
        vs' ← poke_prim prim param_binding st vs;
        Some (<[c.(cell_name) := vs']>σ)
    | _ => None (* unimplemented *)
    end.

  Definition tick_prim (prim: ident) (param_binding: list (ident * N)) (st: state) (inputs: val_map) : option state := 
    fns ← calyx_prims !! prim;
    fns.(prim_sem_tick) st inputs.

  Definition tick_cell (c: cell) (ρ: state_map) (σ: cell_map) : option state_map :=
    match c.(cell_proto) with
    | ProtoPrim prim param_binding _ =>
        st ← ρ !! c.(cell_name);
        vs ← σ !! c.(cell_name);
        st' ← tick_prim prim param_binding st vs;
        Some (<[c.(cell_name) := st']>ρ)
    | _ => None (* unimplemented *)
    end.

  Definition poke_all_cells (ce: cell_env) (ρ: state_map) (σ: cell_map) : option cell_map :=
    map_fold (fun _ cell σ_opt =>
                σ ← σ_opt;
                poke_cell cell ρ σ)
             (Some σ)
             ce.

  (* Update the state, invalidate outgoing wires *)
  Definition tick_all_cells (ce: cell_env) (ρ: state_map) (σ: cell_map) : option state_map :=
    map_fold (fun _ cell ρ_opt =>
                ρ ← ρ_opt;
                tick_cell cell ρ σ)
             (Some ρ)
             ce.

  Definition catch {X} (c1 c2: option X) : option X :=
    match c1 with
    | Some x => Some x
    | None => c2
    end.

  Definition read_port_ref (p: port_ref) (σ: cell_map) (γ: group_map) : option value :=
    match p with
    | PRef parent port =>
        catch (σ !! parent ≫= (!!) port)
              (γ !! parent ≫= (!!) port)
    | _ => None (* TODO *)
    end.

  Definition write_port_ref (p: port_ref) (v: value) (σ: cell_map) (γ: group_map) : option (cell_map * group_map) :=
    match p with
    | PRef parent port =>
        if decide (is_Some (σ !! parent))
        then mret (alter (insert port v) parent σ, γ)
        else if decide (is_Some (γ !! parent))
             then mret (σ, alter (insert port v) parent γ)
             else None
    | _ => None (* TODO *)
    end.
  
  Definition interp_assign
             (ce: cell_env)
             (ρ: state_map)
             (σ: cell_map) 
             (γ: group_map)
             (op: assignment)
    : option (cell_map * group_map) :=
    σ' ← poke_all_cells ce ρ σ;
    v ← read_port_ref op.(src) σ' γ;
    '(σ'', γ') ← write_port_ref op.(dst) v σ' γ;
    mret (σ'', γ').

  Definition poke_group ce ρ σ γ (g: group) : option (cell_map * group_map) := 
    (* there is probably a monad sequencing operation that should be used here *)
    (* n.b. this defintion using foldl assumes the assignments are
            already in dataflow order and will not require iteration
            to reach a fixed point. *)
    foldl (fun res op =>
             '(σ, γ) ← res;
             interp_assign ce ρ σ γ op)
          (Some (σ, γ))
          g.(group_assns).

  Definition is_done (γ: group_map) (g: group) : bool :=
    match holes ← γ !! g.(group_name);
          holes !! "done" with
    | Some v => is_high v
    | None => false
    end.

  Fixpoint interp_group (ce: cell_env) (ρ: state_map) (σ: cell_map) (γ: group_map) (g: group) (gas: nat) : option (state_map * cell_map * group_map) :=
    ρ ← tick_all_cells ce ρ σ;
    '(σ, γ) ← poke_group ce ρ σ γ g;
    if is_done γ g
    then Some (ρ, σ, γ)
    else match gas with
         | S gas => interp_group ce ρ σ γ g gas
         | O => None
         end.

  Definition interp_control (ce: cell_env) (ge: group_env) (ρ: state_map) (σ: cell_map) (γ: group_map) (ctrl: control) (gas: nat) :=
    match ctrl with
    | CEnable group _ =>
        g ← ge !! group;
        interp_group ce ρ σ γ g gas
    | _ => None
    end.

  Definition interp_context (c: context) (gas: nat) :=
    main ← List.find (is_entrypoint c.(ctx_entrypoint)) c.(ctx_comps);
    let '(ce, ge) := load_context c in
    let σ := allocate_cell_map ce in
    let γ := allocate_group_map ge in
    ρ ← allocate_state_map ce;
    interp_control ce ge ρ σ γ main.(comp_control) gas.

End Semantics.
