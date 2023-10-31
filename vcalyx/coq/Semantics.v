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
Require Import VCalyx.Value.
Require Import VCalyx.Arith.

Inductive numtype :=
| Bitnum
| FixedPoint.
    
Record mem_fmt := { is_signed: bool;
                    numeric_type: numtype;
                    width: nat; }.

Definition mem_data := list N.

Inductive state : Type :=
(* std_reg *)
| StateReg (write_done: value) (val: value)
(* std_mem_d1 *)
| StateMemD1 (write_done: value) (fmt: mem_fmt) (contents: mem_data)
| StateDiv (div_done: value) (quotient remainder: value)
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
  | StateDiv _ _ _ => err "get_mem_d1_state: got div"
  | StateComb => err "get_mem_d1_state: got comb"
  end.

Definition is_mem_state_bool (st: state) : bool :=
  match st with
  | StateMemD1 _ _ _ => true
  | _ => false
  end.

Definition get_div_state (st: state) :=
  match st with
  | StateDiv div_done quot rem => mret (div_done, quot, rem)
  | _ => err "get_div_state"
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
  (* map from group name to active flag + values of its holes *)
  Definition group_map : Type := ident_map (bool * val_map).

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

  Definition load_cells_groups (c: context) : cell_env * group_env := 
    foldl load_comp (empty, empty) c.(ctx_comps).

  Definition allocate_val_map (c: cell) : val_map :=
    foldl (fun σ p => <[p.(port_name) := Bot]>σ)
          empty
          (c.(cell_in_ports) ++ c.(cell_out_ports)).

  Definition allocate_cell_map (ce: cell_env) : cell_map :=
    fmap allocate_val_map ce.

  (* Initialize go and done holes to undef *)
  Definition allocate_group_map (ge: group_env) : group_map :=
    fmap (fun (g: group) => (false, <["go" := Bot]>(<["done" := Bot]>empty))) ge.

  Definition prim_initial_state (name: ident) : exn state :=
    if decide (name = "std_reg")
    then mret (StateReg Bot Bot)
    else if decide (name = "std_add")
    then mret StateComb
    else if decide (name = "std_lt")
    then mret StateComb
    else if decide (name = "std_or")
    then mret StateComb
    else if decide (name = "std_const")
    then mret StateComb
    else if decide (name = "std_div_pipe")
    then mret (StateDiv Bot Bot Bot)
    else err ("prim_initial_state: " +:+ name +:+ " is unimplemented").

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
                        (γ !! parent ≫= (!!) port ∘ snd))
    | _ => err "read_port_ref: ports other than PRef unimplemented"
    end.

  Definition write_port_ref (p: port_ref) (v: value) (σ: cell_map) (γ: group_map) : exn (cell_map * group_map) :=
    match p with
    | PRef parent port =>
        if decide (is_Some (σ !! parent))
        then mret (alter (insert port v) parent σ, γ)
        else if decide (is_Some (γ !! parent))
             then mret (σ, alter (fun '(active, holes) => (active, insert port v holes)) parent γ)
             else err "write_port_ref: parent not found in group_map"
    | _ => err "write_port_ref: ports other than PRef unimplemented"
    end.

  Definition set_group_active_bit (b: bool) (g: ident) (γ: group_map) : group_map :=
    alter (fun '(active, holes) => (b, holes)) g γ.

  Definition mark_group_active : ident -> group_map -> group_map :=
    set_group_active_bit true.

  Definition mark_group_inactive : ident -> group_map -> group_map :=
    set_group_active_bit false.

  Definition incorp (lhs rhs: value) : value :=
    match lhs with
    | Bot => rhs
    | _ => Top
    end.
  
  Definition interp_assign
             (ce: cell_env)
             (ρ: state_map)
             (σ: cell_map) 
             (γ: group_map)
             (op: assignment)
    : exn (cell_map * group_map) :=
    σ' ← poke_all_cells ce ρ σ;
    lhs ← read_port_ref op.(dst) σ' γ;
    rhs ← read_port_ref op.(src) σ' γ;
    '(σ'', γ') ← write_port_ref op.(dst) rhs σ' γ;
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

  Definition is_done (γ: group_map) (g: ident) : bool :=
    match '(_, holes) ← γ !! g;
          holes !! "done" with
    | Some v => is_one v
    | None => false
    end.

  Definition cycle_group (ce: cell_env) (ρ: state_map) (σ: cell_map) (γ: group_map) (g: group) : exn (state_map * cell_map * group_map) :=
    ρ ← tick_all_cells ce ρ σ;
    '(σ, γ) ← poke_group ce ρ σ γ g;
    inl (ρ, σ, γ).

  Definition cycle_groups (ce: cell_env) (ρ: state_map) (σ: cell_map) (γ: group_map) (gs: list group) : exn (state_map * cell_map * group_map) :=
    foldl (fun res g =>
             '(ρ, σ, γ) ← res;
             cycle_group ce ρ σ γ g)
          (mret (ρ, σ, γ)) gs.

  (* Two size functions for getting a handle on nested recursion *)
  Fixpoint control_size (ctrl: control) : nat :=
    match ctrl with
    | CSeq ctrls _ =>
        1 + (fix controls_size cs :=
               match cs with
               | c :: cs => control_size c + controls_size cs
               | [] => 0
               end) ctrls
    | _ => 1
    end.

  Fixpoint control_size_exn (ctrl: control) : exn nat :=
    match ctrl with
    | CSeq ctrls _ =>
        v ← (fix controls_size_exn cs : exn nat :=
               match cs with
               | c :: cs =>
                   n ← control_size_exn c;
                   m ← controls_size_exn cs;
                   mret $ n + m
               | [] => mret $ 0
               end) ctrls;
       mret (1 + v)
    | _ => mret 1
    end.

  Definition ctrl_is_done (ctrl: control) : bool :=
    match ctrl with
    | CSeq [] _
    | CPar [] _
    | CEmpty _ => true
    | CSeq _ _
    | CPar _ _
    | CIf _ _ _ _ _
    | CWhile _ _ _ _
    | CEnable _ _
    | CWaitGroup _ _
    | CInvoke _ _ _ _ _ _
    | CWaitComp _ _ => false
    end.

  Fixpoint open_control (ctrl: control) : exn control :=
    match ctrl with
    | CEnable group attrs =>
        mret $ CWaitGroup group attrs
    | CSeq ctrls attrs =>
        ctrls' ← (fix open_controls (cs: list control) : exn (list control) :=
                    match cs with
                    | c :: cs =>
                        if ctrl_is_done c
                        then open_controls cs
                        else c ← open_control c;
                             cs ← open_controls cs;
                             mret $ c :: cs
                    | [] => mret $ []
                    end) ctrls;
        mret $ match ctrls' with
               | [] => CEmpty attrs
               | ctrls' => CSeq ctrls' attrs
               end
    | CPar ctrls attrs =>
        if forallb ctrl_is_done ctrls
        then mret $ CEmpty attrs
        else ctrls' ← mapM (open_control) ctrls;
        mret $ CPar ctrls' attrs
    | CWaitGroup g attrs => mret $ CWaitGroup g attrs
    | CWaitComp c attrs => mret $ CWaitComp c attrs
    | CEmpty attrs => mret $ CEmpty attrs
    | CIf cond_port cond_group then_ctrl else_ctrl attrs =>
        mret $ CIf cond_port cond_group then_ctrl else_ctrl attrs
    | CWhile cond_port cond_group body_ctrl attrs =>
        mret $ CWhile cond_port cond_group body_ctrl attrs
    | CInvoke _ _ _ _ _ _ => err "open_control: CInvoke unimplemented"
    end.

  Fixpoint close_control γ (ctrl: control) {struct ctrl} : exn control :=
    match ctrl with
    | CEnable group attrs =>
      mret $ CEnable group attrs
    | CSeq stmts attrs =>
        stmts' ← (fix close_controls (stmts: list control) : exn (list control) :=
           match stmts with
           | stmt :: stmts =>
               if ctrl_is_done stmt
               then close_controls stmts
               else stmt' ← close_control γ stmt;
                    mret $ stmt' :: stmts
           | [] => mret $ []
           end) stmts;
        mret $ match stmts' with
               | [] => CEmpty attrs
               | ctrls' => CSeq stmts' attrs
               end
    | CPar stmts attrs =>
        stmts' ← mapM (close_control γ) stmts;
        mret $ CPar stmts' attrs
    | CWaitGroup group attrs =>
        if is_done γ group
        then mret $ CEmpty attrs
        else mret $ CWaitGroup group attrs
    | CWaitComp comp _ => err "close_control: CWaitComp unimplemented"
    | CEmpty attrs => mret $ CEmpty attrs
    | CIf cond_port cond tru fls attrs =>
        err "close_control: CIf unimplemented"
    | CWhile cond_port cond body attrs =>
        err "close_control: CWhile unimplemented"
    | CInvoke _ _ _ _ _ _ => err "close_control: CInvoke unimplemented"
    end.

  Fixpoint cycle_active_cells (ce: cell_env) (ge: group_env) (ρ: state_map) (σ: cell_map) (γ: group_map) (ctrl: control) : exn (state_map * cell_map * group_map) :=
      match ctrl with
      | CWaitGroup group _ =>
        g ← lift_opt ("cycle_active_cells: group " +:+ group +:+ " not found in group_env")
                    (ge !! group);
        cycle_group ce ρ σ γ g
      | CSeq (ctrl :: ctrls) attrs =>
        cycle_active_cells ce ge ρ σ γ ctrl
      | CSeq [] _ => mret (ρ, σ, γ)
      | CIf cond_port cond_group then_ctrl else_ctrl _ =>
        (* n.b. we don't implement with right now *)
        port_val ← read_port_ref cond_port σ γ;
        if is_nonzero port_val
        then cycle_active_cells ce ge ρ σ γ then_ctrl
        else cycle_active_cells ce ge ρ σ γ else_ctrl
      | CEnable _ _
      | CInvoke _ _ _ _ _ _
      | CEmpty _ => mret (ρ, σ, γ)
      | CPar ctrls attrs => err "cycle_active_cells: CPar unimplemented"
      | CWhile _ _ _ _ => err "cycle_active_cells: CWhile unimplemented"
      | CWaitComp _ _ => err "cycle_active_cells: CWaitComp unimplemented"
      end.

  Definition find_entrypoint (name: ident) (comps: list comp) :=
    lift_opt ("find_entrypoint: " +:+ name +:+ " not found")
             (List.find (is_entrypoint name) comps).

  Definition load_context (c: context) (mems: state_map) :=
    main ← find_entrypoint c.(ctx_entrypoint) c.(ctx_comps);
    let '(ce, ge) := load_cells_groups c in
    let σ := allocate_cell_map ce in
    let γ := allocate_group_map ge in
    ρ ← allocate_state_map ce mems;
    mret (ce, ge, ρ, σ, γ, main.(comp_control)).

  Definition tick_control (ce: cell_env) (ge: group_env) (ρ: state_map) (σ: cell_map) (γ: group_map) (ctrl: control) : exn (control * state_map * cell_map * group_map) :=
    ctrl ← open_control ctrl;
    '(ρ, σ, γ) ← cycle_active_cells ce ge ρ σ γ ctrl;
    ctrl ← close_control γ ctrl;
    mret (ctrl, ρ, σ, γ).

  Fixpoint interp_control (ce: cell_env) (ge: group_env) (ρ: state_map) (σ: cell_map) (γ: group_map) (ctrl: control) (gas: nat) : exn (state_map * cell_map * group_map) :=
    match gas with
    | 0 => err "interp_control: out of gas"
    | S gas => if ctrl_is_done ctrl
              then mret (ρ, σ, γ)
              else '(ctrl, ρ, σ, γ) ← tick_control ce ge ρ σ γ ctrl;
                   interp_control ce ge ρ σ γ ctrl gas
    end.

  Definition interp_context (c: context) (mems: state_map) (gas: nat) : exn (state_map * cell_map * group_map) :=
    '(ce, ge, ρ, σ, γ, ctrl) ← load_context c mems;
    interp_control ce ge ρ σ γ ctrl gas.

  Definition extract_mems (ρ: state_map) : list (ident * state) :=
    List.filter (fun '(name, st) => is_mem_state_bool st) (map_to_list ρ).

End Semantics.

Definition assoc_list K V := list (K * V).
#[export] Instance assoc_list_FMap (K: Type) : FMap (assoc_list K) :=
  fun V V' f => List.map (fun '(k, v) => (k, f v)).

#[export] Instance assoc_list_Lookup : forall V, Lookup string V (assoc_list string V) :=
  fun _ needle haystack =>
    match List.find (fun '(k, v) => if string_eq_dec k needle then true else false) haystack with
    | Some (_, v) => Some v
    | None => None
    end.

#[export] Instance assoc_list_Empty: forall V, Empty (assoc_list string V) :=
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

#[export] Instance assoc_list_PartialAlter: ∀ V : Type, PartialAlter string V (assoc_list string V) :=
  assoc_list_partial_alter.

#[export] Instance assoc_list_OMap: OMap (assoc_list string) :=
  fun V B f m =>
    [].

#[export] Instance assoc_list_Merge: Merge (assoc_list string) :=
  fun _ _ _ _ _ _ => [].

#[export] Instance assoc_list_FinMapToList: forall V, FinMapToList string V (assoc_list string V) :=
  fun _ => id.

#[export] Instance assoc_list_finmap: FinMap string (assoc_list string).
Admitted.

Definition calyx_prims : prim_map (assoc_list string) :=
  [
    ("std_reg",
      {| prim_sem_poke st inputs :=
        '(write_done, v) ← get_reg_state st;
        mret (<["done" := write_done]>(<["out" := v]>inputs));
        prim_sem_tick st inputs :=
        '(_, val_old) ← get_reg_state st;
        write_en ← lift_opt "std_reg tick: write_en missing"
                 (inputs !! "write_en");
        if is_one write_en
        then val_in ← lift_opt "std_reg tick: in missing" (inputs !! "in");
          mret (StateReg (V 1%N) val_in)
        else mret (StateReg (V 0%N) val_old)
      |});
    ("std_mem_d1",
      {| prim_sem_poke st inputs :=
        '(write_done, fmt, contents) ← get_mem_d1_state st;
        addr ← lift_opt "std_mem_d1 poke: addr0 missing" (inputs !! "addr0");
        mem_val ← match addr with
                  | Top => mret Top
                  | V idx => V <$> lift_opt "std_mem_d1 poke: out of bounds access" (contents !! (N.to_nat idx))
                  | Bot => mret Bot
                  end;
        mret (<["done" := write_done]>(<["read_data" := mem_val]>inputs));
        prim_sem_tick st inputs :=
        '(_, fmt, contents) ← get_mem_d1_state st;
        write_en ← lift_opt "std_mem_d1 tick: write_en missing"
                 (inputs !! "write_en");
        if is_one write_en
        then val_in ← lift_opt "std_mem_d1 tick: write_data missing" (inputs !! "write_data");
          val ← expect_V val_in;
          addr ← lift_opt "st_mem_d1 tick: addr0 missing" (inputs !! "addr0");
          idx ← expect_V addr;
          mret (StateMemD1 (V 1%N) fmt (<[N.to_nat idx := val]>contents))
        else mret (StateMemD1 (V 0%N) fmt contents)
      |});
    ("std_const",
      {| prim_sem_poke st inputs := mret inputs;
         prim_sem_tick st inputs := mret st;
      |});
    ("std_lt",
      {| prim_sem_poke st inputs :=
           val_left ← lift_opt "std_lt poke: left missing" (inputs !! "left");
           val_right ← lift_opt "std_lt poke: right missing" (inputs !! "right");
           let val_out := value_lt val_left val_right in
           mret (<["out" := val_out]>inputs);
         prim_sem_tick st inputs := mret st;
      |});
    ("std_or",
      {| prim_sem_poke st inputs :=
           val_left ← lift_opt "std_or poke: left missing" (inputs !! "left");
           val_right ← lift_opt "std_or poke: right missing" (inputs !! "right");
           let val_out := value_or val_left val_right in
           mret (<["out" := val_out]>inputs);
         prim_sem_tick st inputs := mret st;
      |});
    ("std_add",
      {| prim_sem_poke st inputs :=
           val_left ← lift_opt "std_add poke: left missing" (inputs !! "left");
           val_right ← lift_opt "std_add poke: right missing" (inputs !! "right");
           let val_out := value_add val_left val_right in
           mret (<["out" := val_out]>inputs);
         prim_sem_tick st inputs := mret st;
      |});
    ("std_div_pipe",
      {| prim_sem_poke st inputs :=
           '(div_done, div_quotient, div_remainder) ← get_div_state st;
           mret (<["done" := div_done]>
                   (<["out_quotient" := div_quotient]>
                      (<["out_remainder" := div_remainder]>
                         inputs)));
         prim_sem_tick st inputs :=
           '(div_done, div) ← get_div_state st;
           go ← lift_opt "std_div_pipe tick: go missing"
                         (inputs !! "go");
           if is_one go
           then val_left ← lift_opt "std_div_pipe poke: left missing" (inputs !! "left");
                val_right ← lift_opt "std_div_pipe poke: right missing" (inputs !! "right");
                let val_out_quotient := value_div_quotient val_left val_right in
                let val_out_remainder := value_div_remainder val_left val_right in
                mret (StateDiv (V 1%N) val_out_quotient val_out_remainder)
           else mret (StateDiv (V 0%N) Bot Bot)
      |})
  ].

(* interp_context instantiated with the gmap finite map data structure *)
Definition interp_with_mems (c: context) (mems: list (ident * state)) (gas: nat) :=
  let mems := list_to_map mems in
  '(ρ, σ, γ) ← interp_context (assoc_list ident) calyx_prims c mems gas;
  mret (extract_mems _ ρ).

Definition find_prim s := calyx_prims !! s.
