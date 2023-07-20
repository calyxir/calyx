From stdpp Require Import
     strings
     fin_maps.
From Coq Require Import
     Numbers.BinNums.

(** * Calyx Syntax *)
(** This file defines the syntax of Calyx. It is mostly based on the
    contents of ast.rs in the calyx repo and the Calyx language
    documentation. *)

(** Calyx identifiers are strings. *)
Definition ident := string.
(** TODO: The attributes type should be a finite map from strings to
    64 bit integers. *)
Definition attrs: Type :=
  unit.

(** Directions for ports. *)
Inductive direction :=
| Input
| Output.

(** Port definitions. *)
Record port_def :=
  PortDef {
      port_name: ident;
      port_width: N;
      port_dir: direction;
      port_attrs: attrs;
    }.

(** Collections of port definitions. *)
Definition port_defs :=
  list port_def.

(** Primitives. *)
Record prim :=
  Prim {
      prim_name: ident;
      prim_comb: bool;
      prim_attrs: attrs;
      prim_ports: port_defs
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
| ProtoThis
| ProtoConst (val: N) (width: N).

(** Cells. *)
Record cell :=
  Cell {
      (* name of this cell. *)
      cell_name: ident;
      (* ports *)
      cell_in_ports: list port_def;
      cell_out_ports: list port_def;
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

(** Atoms. *)
Inductive atom :=
| AtPort (port: port_ref)
| AtLit (b: num).

(** Comparisons that can be used in guard expressions. *)
Inductive guard_cmp :=
| Eq
| Neq
| Gt
| Lt
| Geq
| Leq.

(** Guard expressions. *)
Inductive guard_expr :=
| GAnd (e1 e2: guard_expr)
| GOr (e1 e2: guard_expr)
| GNot (e: guard_expr)
| GCompOp (op: guard_cmp) (a1 a2: atom)
| GAtom (a: atom).

(** Wires, a.k.a. continuous guarded assignments. *)
Record wire :=
  Wire {
      wire_guard: guard_expr; (* Guard for the wire. *)
      wire_src: atom; (* Source of the wire. *)
      wire_dst: port_ref; (* Guarded destinations of the wire. *)
      wire_attrs: attrs;
    }.

Definition wires :=
  list wire.

(** Control statements. Each constructor has its own attributes [attrs]. *)
Inductive control :=
| CSeq (stmts: list control)
      (attrs: attrs)
| CPar (stmts: list control)
      (attrs: attrs)
| CIf (cond_port: port_ref)
     (cond: option ident)
     (tru: control)
     (fls: control)
     (attrs: attrs)
| CWhile (cond_port: port_ref)
        (cond: option ident)
        (body: control)
        (attrs: attrs)
| CEnable (comp: ident)
         (atrs: attrs)
| CInvoke (comp: ident)
         (inputs: list (ident * atom))
         (outputs: list (ident * atom))
         (attrs: attrs)
         (comb_group: option ident)
         (ref_cells: list (ident * ident))
| CEmpty (attrs: attrs).

(** Groups. *)
Record group :=
  Group { group_attrs: attrs;
          group_name: ident;
          group_wires: list wire;
          group_is_comb: bool; }.

(** Components. *)
Record comp :=
  Comp { comp_attrs: attrs; 
         comp_name: ident;
         (* aka signature *)
         comp_ports: port_defs;
         comp_cells: cells;
         comp_groups: list group;
         (* aka continuous assignments *)
         comp_wires: wires;
         comp_control: control;
         comp_is_comb: bool }.

Record context :=
  Context { ctx_comps: list comp;
            ctx_entrypoint: ident; }.
