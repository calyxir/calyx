From stdpp Require Import
     strings
     fin_maps.
From Coq Require Import
     Numbers.BinNums.

(** * Calyx Syntax *)
(** This file defines the syntax of Calyx. It is mostly based on the
    contents of the calyx/src/ir folder in the calyx repo and the Calyx language
    documentation. *)

(** Calyx identifiers are strings. *)
Definition ident := string.

(** https://docs.calyxir.org/lang/attribute.html?highlight=attribute#attribute *)
(** TODO: Some attribute can only belong to cells, or groups, or components, etc. *)
Inductive attribute :=
| Toplevel
| Go
| Done
| Clk
| Reset
| Nointerface
| External
| Static (cycles: nat)
| Inline
| Stable
| Share
| StateShare
| Bound (iters: nat)
| Generated
| WriteTogether (signals: nat) 
| ReadTogether (signals: nat) 
| Data.

Definition attributes := list attribute.

(** Directions for ports. *)
Inductive direction :=
| Input
| Output
| InOut.

Inductive port_parent :=
| PCell
| PGroup. 

(** Ports. *)
Record port :=
  Port {
      port_name: ident;
      port_width: N;
      port_dir: direction;
      parent: port_parent;
      port_attribute: attributes;
    }.

(** Collections of port definitions. *)
Definition ports :=
  list port.

(** Primitives. *)
Record prim :=
  Prim {
      prim_name: ident;
      prim_params: list ident;
      prim_comb: bool;
      prim_attribute: attributes;
      prim_ports: ports;
      (* for inlined primitives *)
      prim_body: option ident
    }.
    
(** Externs. *)
Record extern :=
  Extern {
      extern_path: string;
      extern_prims: list prim
    }.

(** Cell prototype references. *)
Inductive proto :=
| ProtoPrim (name: ident)
            (param_binding: list (N * ident))
            (is_comb: bool)
| ProtoComp (name: ident)
(* For use when referencing a defined component within itself  *)
| ProtoThis
| ProtoConst (val: N) (width: N).

(** Cells. *)
Record cell :=
  Cell {
      (* name of this cell. *)
      cell_name: ident;
      (* ports *)
      cell_in_ports: list port;
      cell_out_ports: list port;
      (* name of the prototype this cell was built from. *)
      cell_proto: proto;
      (* whether this cell is by-reference or not *)
      cell_ref: bool;
    }.

Definition cells := list cell.

(** Relative references to ports. *)
Inductive port_ref :=
(* refers to the port named [port] on the subcomponent [component]. *)
| PComp (component: ident) (port: ident)
(* refers to the port named [port] on the enclosing component. *)
| PThis (port: ident)
(* group[name] parses into [Hole group name] and is a hole named name
   on the group [group] *)
| PHole (group: ident) (name: ident).

(** Nonnegative integers of a fixed bit-width. *)
Record num :=
  { num_width: positive;
    num_val: N; }.

(** Comparisons that can be used in guard expressions. *)
Inductive guard_cmp :=
| Eq
| Neq
| Gt
| Lt
| Geq
| Leq.

(** Guard expressions. *)
(* https://docs.calyxir.org/lang/ref.html?highlight=guard#guards *)
Inductive guard_expr :=
| GAnd (e1 e2: guard_expr)
| GOr (e1 e2: guard_expr)
| GNot (e: guard_expr)
| GCompOp (op: guard_cmp) (p1 p2: port)
(* Same as p == true *)
| GPort (p: port)
(* The constant true *)
| GTrue.

(* From AST wires *)
Record assignment := 
  Assign {
    dst: cell * ident;
    src: port; 
    assign_guard: guard_expr;
    attrs: attributes;
  }.

Definition assignments :=
  list assignment.

(** Control statements. Each constructor has its own attribute [attribute]. *)
Inductive control :=
| CSeq (stmts: list control)
      (attrs: attributes)
| CPar (stmts: list control)
      (attrs: attributes)
| CIf (cond_port: port_ref)
     (cond: option ident)
     (tru: control)
     (fls: control)
     (attrs: attributes)
| CWhile (cond_port: port_ref)
        (cond: option ident)
        (body: control)
        (attrs: attributes)
| CEnable (comp: ident)
         (atrs: attributes)
| CInvoke (comp: ident)
         (inputs: list (ident * port))
         (outputs: list (ident * port))
         (attrs: attributes)
         (comb_group: option ident)
         (ref_cells: list (ident * ident))
| CEmpty (attrs: attributes).

(** Groups. *)
Record group :=
  Group { group_attrs: attributes;
          group_name: ident;
          group_assns: assignments;
          group_holes: ports; 
  }.

(** Combinational groups. *)
(** A combinational group does not have any holes and 
should only contain assignments that will be combinationally active *)
Record comb_group :=
  CombGroup { comb_group_attrs: attributes;
          comb_group_name: ident;
          comb_group_assns: assignments;
  }.

(** Components. *)
Record comp :=
  Comp { comp_attrs: attribute; 
         comp_name: ident;
         (* aka signature *)
         comp_sig: cell;
         comp_cells: cells;
         comp_groups: list group;
         comp_comb_groups: list comb_group;
         comp_cont_assns: assignments;
         comp_control: control;
         comp_is_comb: bool }.

Record context :=
  Context { ctx_comps: list comp;
            ctx_entrypoint: ident; }.
