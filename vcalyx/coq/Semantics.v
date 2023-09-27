From stdpp Require Import
     base
     fin_maps
     gmap
     numbers
     list
     strings
     option.
Require Import Coq.Classes.EquivDec.
Require Import VCalyx.IRSyntax.
Require Import VCalyx.Exception.

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

Definition expect_V (v: value) : exn N :=
  match v with
  | V val => mret val
  | _ => err "expect_V"
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
| StateMemD1 (write_done: bool) (fmt: mem_fmt) (contents: mem_data)
(* A primitive with no internal state *)
| StateComb.

Definition get_reg_state (st: state) :=
  match st with
  | StateReg write_done v => mret (write_done, v)
  | _ => err "get_reg_state"
  end.

Definition get_mem_d1_state (st: state) :=
  match st with
  | StateMemD1 write_done fmt contents => mret (write_done, fmt, contents)
  | StateReg _ _ => err "get_mem_d1_state: got reg"
  | StateComb => err "get_mem_d1_state: got comb"
  end.

Definition is_mem_state_bool (st: state) : bool :=
  match st with
  | StateMemD1 _ _ _ => true
  | _ => false
  end.

Section Semantics.
  (* ENVIRONMENTS AND STORES *)
  (* Definitions of types of finite maps used in the semantics. *)
  Context (ident_map: Type -> Type).
  Context `{FinMap ident ident_map}.
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
    { prim_sem_poke: state -> val_map -> exn val_map;
      prim_sem_tick: state -> val_map -> exn state }.
  
  (* An environment collecting all defined primitives *)
  Definition prim_map : Type := ident_map prim_sem.

  (* An environment collecting all defined groups *)
  Definition group_env : Type := ident_map group.
  (* map from group name to values of its holes *)
  Definition group_map : Type := ident_map val_map.

  Variable (calyx_prims : prim_map).

  Open Scope stdpp_scope.
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

  Definition prim_initial_state (name: ident) : exn state :=
    if decide (name = "std_reg")
    then mret (StateReg false X)
    else if decide (name = "std_add")
    then mret StateComb
    else if decide (name = "std_or")
    then mret StateComb
    else if decide (name = "std_const")
    then mret StateComb
    else err ("prim_initial_state: " +:+ name +:+ " is not a std_reg, unimplemented").

  Definition allocate_state_for_cell (c: cell) (ρ: state_map) : exn state_map :=
    match c.(cell_proto) with
    | ProtoPrim prim_name bindings is_comb =>
        st ← prim_initial_state prim_name;
        mret (<[c.(cell_name) := st]>ρ)
    | ProtoThis
    | ProtoConst _ _
    | ProtoComp _ => mret ρ (* TODO FIX *)
    end.

  Definition allocate_state_map (ce: cell_env) (initial: state_map) : exn state_map :=
    map_fold (fun name (c: cell) (ρ0: exn state_map) =>
                ρ ← ρ0;
                match initial !! c.(cell_name) with
                | Some st__init => mret (<[c.(cell_name) := st__init]>ρ)
                | None => allocate_state_for_cell c ρ
                end)
             (inl empty)
             ce.

  (* COMBINATIONAL UPDATES *)
  Definition poke_prim (prim: ident) (param_binding: list (ident * N)) (st: state) (inputs: val_map) : exn val_map := 
    fns ← lift_opt ("poke_prim: " +:+ prim +:+ " not found")
                   (calyx_prims !! prim);
    fns.(prim_sem_poke) st inputs.
  
  Definition poke_cell (c: cell) (ρ: state_map) (σ: cell_map) : exn cell_map :=
    match c.(cell_proto) with
    | ProtoPrim prim param_binding _ =>
        st ← lift_opt "poke_cell" (ρ !! c.(cell_name));
        vs ← lift_opt "poke_cell" (σ !! c.(cell_name));
        vs' ← poke_prim prim param_binding st vs;
        mret (<[c.(cell_name) := vs']>σ)
    | ProtoComp c => err "tick_cell: ProtoComp unimplemented"
    | ProtoThis => err "tick_cell: ProtoThis unimplemented"
    | ProtoConst val width =>
        let vs := <["out" := V val]>empty in
        mret (<[c.(cell_name) := vs]>σ)
    end.

  Definition tick_prim (prim: ident) (param_binding: list (ident * N)) (st: state) (inputs: val_map) : exn state := 
    fns ← lift_opt ("tick_prim: " +:+ prim +:+ " not found")
                   (calyx_prims !! prim);
    match fns.(prim_sem_tick) st inputs with 
    | inl ok => inl ok
    | inr error => inr ("tick_prim for " +:+ prim +:+ ": " +:+ error)
    end.

  Definition tick_cell (c: cell) (ρ: state_map) (σ: cell_map) : exn state_map :=
    match c.(cell_proto) with
    | ProtoPrim prim param_binding _ =>
        st ← lift_opt ("tick_cell: " +:+ c.(cell_name) +:+ " not found in state_map")
                      (ρ !! c.(cell_name));
        vs ← lift_opt ("tick_cell: " +:+ c.(cell_name) +:+ " not found in cell_map")
                      (σ !! c.(cell_name));
        st' ← tick_prim prim param_binding st vs;
        mret (<[c.(cell_name) := st']>ρ)
    | ProtoComp c => err "tick_cell: ProtoComp unimplemented"
    | ProtoThis => err "tick_cell: ProtoThis unimplemented"
    | ProtoConst _ _ => mret ρ
    end.

  Definition poke_all_cells (ce: cell_env) (ρ: state_map) (σ: cell_map) : exn cell_map :=
    map_fold (fun _ cell σ_opt =>
                σ ← σ_opt;
                poke_cell cell ρ σ)
             (inl σ)
             ce.

  (* Update the state, invalidate outgoing wires *)
  Definition tick_all_cells (ce: cell_env) (ρ: state_map) (σ: cell_map) : exn state_map :=
    (*
    err (map_fold (fun key cell acc =>
                     let prim := match cell.(cell_proto) with
                                 | ProtoPrim prim param_binding _ => prim
                                 | _ => "<no prim>"
                                 end in
                key +:+ ":" +:+ prim +:+ " " +:+ acc)
             ""
             ce). *)
    map_fold (fun _ cell ρ_opt =>
                ρ ← ρ_opt;
                tick_cell cell ρ σ)
             (inl ρ)
             ce.

  Definition catch {X} (c1 c2: option X) : option X :=
    match c1 with
    | Some x => Some x
    | None => c2
    end.

  Definition read_port_ref (p: port_ref) (σ: cell_map) (γ: group_map) : exn value :=
    match p with
    | PRef parent port =>
        lift_opt "read_port_ref: port not found"
                 (catch (σ !! parent ≫= (!!) port)
                        (γ !! parent ≫= (!!) port))
    | _ => err "read_port_ref: ports other than PRef unimplemented"
    end.

  Definition write_port_ref (p: port_ref) (v: value) (σ: cell_map) (γ: group_map) : exn (cell_map * group_map) :=
    match p with
    | PRef parent port =>
        if decide (is_Some (σ !! parent))
        then mret (alter (insert port v) parent σ, γ)
        else if decide (is_Some (γ !! parent))
             then mret (σ, alter (insert port v) parent γ)
             else err "write_port_ref: parent not found in group_map"
    | _ => err "write_port_ref: ports other than PRef unimplemented"
    end.
  
  Definition interp_assign
             (ce: cell_env)
             (ρ: state_map)
             (σ: cell_map) 
             (γ: group_map)
             (op: assignment)
    : exn (cell_map * group_map) :=
    σ' ← poke_all_cells ce ρ σ;
    v ← read_port_ref op.(src) σ' γ;
    '(σ'', γ') ← write_port_ref op.(dst) v σ' γ;
    mret (σ'', γ').

  Definition poke_group ce ρ σ γ (g: group) : exn (cell_map * group_map) := 
    (* there is probably a monad sequencing operation that should be used here *)
    (* n.b. this defintion using foldl assumes the assignments are
            already in dataflow order and will not require iteration
            to reach a fixed point. *)
    foldl (fun res op =>
             '(σ, γ) ← res;
             interp_assign ce ρ σ γ op)
          (mret (σ, γ))
          g.(group_assns).

  Definition is_done (γ: group_map) (g: group) : bool :=
    match holes ← γ !! g.(group_name);
          holes !! "done" with
    | Some v => is_high v
    | None => false
    end.

  Fixpoint interp_group (ce: cell_env) (ρ: state_map) (σ: cell_map) (γ: group_map) (g: group) (gas: nat) : exn (state_map * cell_map * group_map) :=
    ρ ← tick_all_cells ce ρ σ;
    '(σ, γ) ← poke_group ce ρ σ γ g;
    if is_done γ g
    then inl (ρ, σ, γ)
    else match gas with
         | S gas => interp_group ce ρ σ γ g gas
         | O => err "interp_group: out of gas"
         end.

  Definition interp_control (ce: cell_env) (ge: group_env) (ρ: state_map) (σ: cell_map) (γ: group_map) (ctrl: control) (gas: nat) : exn _:=
    match ctrl with
    | CEnable group _ =>
        g ← lift_opt ("interp_control: group " +:+ group +:+ " not found in group_env")
                     (ge !! group);
        interp_group ce ρ σ γ g gas
    | _ => err "interp_control: control was not a single CEnable"
    end.

  Definition find_entrypoint (name: ident) (comps: list comp) :=
    lift_opt ("find_entrypoint: " +:+ name +:+ " not found")
             (List.find (is_entrypoint name) comps).

  Definition interp_context (c: context) (mems: state_map) (gas: nat) : exn (state_map * cell_map * group_map) :=
    main ← find_entrypoint c.(ctx_entrypoint) c.(ctx_comps);
    let '(ce, ge) := load_context c in
    let σ := allocate_cell_map ce in
    let γ := allocate_group_map ge in
    ρ ← allocate_state_map ce mems;
    interp_control ce ge ρ σ γ main.(comp_control) gas.

  Definition extract_mems (ρ: state_map) : list (ident * state) :=
    List.filter (fun '(name, st) => is_mem_state_bool st) (map_to_list ρ).

End Semantics.

Definition assoc_list K V := list (K * V).
Instance assoc_list_FMap (K: Type) : FMap (assoc_list K) :=
  fun V V' f => List.map (fun '(k, v) => (k, f v)).

Instance assoc_list_Lookup : forall V, Lookup string V (assoc_list string V) :=
  fun _ needle haystack =>
    match List.find (fun '(k, v) => if string_eq_dec k needle then true else false) haystack with
    | Some (_, v) => Some v
    | None => None
    end.

Instance assoc_list_Empty: forall V, Empty (assoc_list string V) :=
  fun _ => [].

Fixpoint assoc_list_partial_alter (V: Type) (f: option V -> option V) (k: string) (l: assoc_list string V) : assoc_list string V :=
  match l with
  | [] =>
      match f None with
      | Some fv => [(k, fv)]
      | None => []
      end
  | (k', v)::l =>
      if string_eq_dec k k'
      then match f (Some v) with
           | Some fv => (k, fv)::l
           | None => l
           end
      else (k', v)::assoc_list_partial_alter V f k l
  end.

Instance assoc_list_PartialAlter: ∀ V : Type, PartialAlter string V (assoc_list string V) :=
  assoc_list_partial_alter.

Instance assoc_list_OMap: OMap (assoc_list string) :=
  fun V B f m =>
    [].

Instance assoc_list_Merge: Merge (assoc_list string) :=
  fun _ _ _ _ _ _ => [].

Instance assoc_list_FinMapToList: forall V, FinMapToList string V (assoc_list string V) :=
  fun _ => id.

Instance assoc_list_finmap: FinMap string (assoc_list string).
Admitted.

Definition calyx_prims : prim_map (assoc_list string) :=
  [
    ("std_reg",
         {| prim_sem_poke st inputs :=
              '(write_done, v) ← get_reg_state st;
              mret (<["done" := bool_to_value write_done]>(<["out" := v]>inputs));
            prim_sem_tick st inputs :=
              '(_, val_old) ← get_reg_state st;
              write_en ← lift_opt "std_reg tick: write_en missing"
                                  (inputs !! "write_en");
              if is_high write_en
              then val_in ← lift_opt "std_reg tick: in missing" (inputs !! "in");
                   mret (StateReg true val_in)
              else mret (StateReg false val_old)
         |});
       ("std_mem_d1",
         {| prim_sem_poke st inputs :=
              '(write_done, fmt, contents) ← get_mem_d1_state st;
              addr ← lift_opt "std_mem_d1 poke: addr0 missing" (inputs !! "addr0");
              mem_val ← match addr with
                        | Z => mret Z
                        | V idx => V <$> lift_opt "std_mem_d1 poke: out of bounds access" (contents !! (N.to_nat idx))
                        | X => mret X
                        end;
              mret (<["done" := bool_to_value write_done]>(<["read_data" := mem_val]>inputs));
           prim_sem_tick st inputs :=
              '(_, fmt, contents) ← get_mem_d1_state st;
              write_en ← lift_opt "std_mem_d1 tick: write_en missing"
                                  (inputs !! "write_en");
              if is_high write_en
              then val_in ← lift_opt "std_mem_d1 tick: write_data missing" (inputs !! "write_data");
                   val ← expect_V val_in;
                   addr ← lift_opt "st_mem_d1 tick: addr0 missing" (inputs !! "addr0");
                   idx ← expect_V addr;
                   mret (StateMemD1 true fmt (<[N.to_nat idx := val]>contents))
              else mret (StateMemD1 false fmt contents)
         |});
       ("std_const", {|
           prim_sem_poke st inputs := mret inputs;
           prim_sem_tick st inputs := mret st;
       |})].

(* interp_context instantiated with the gmap finite map data structure *)
Definition interp_with_mems (c: context) (mems: list (ident * state)) (gas: nat) :=
  let mems := list_to_map mems in
  '(ρ, σ, γ) ← interp_context (assoc_list ident) calyx_prims c mems gas;
  mret (extract_mems _ ρ).

Definition find_prim s := calyx_prims !! s.
Eval vm_compute in find_prim "".
