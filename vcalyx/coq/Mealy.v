Require VCalyx.Vect.
From stdpp Require Import streams.
(** * Mealy Machines *)

(** Definition of a Mealy machine: a map [S -> (O x S) ^ I].  In the
    textbook definition each of the three parameters would need to be
    finite; we unbundle this requirement. *)
Definition mealy (S I O: Type) :=
  S -> I -> O * S.

(** First component of the transition map: the output observation. *)
Definition obs {S I O: Type} (m: mealy S I O) : S -> I -> O :=
    fun s i => fst (m s i).
  
(** Second component of the transition map: the next state. *)
Definition step {S I O: Type} (m: mealy S I O) : S -> I -> S :=
    fun s i => snd (m s i).

(** A [tuple_mealy] is a Mealy machine where inputs and outputs are
    tuples [V^n] and [V^m], represented as untyped lists. *)
Definition tuple_mealy (V S: Type) (n m : nat) :=
  mealy S (list V) (list V).

(* Sequential composition of two mealy machines, where the second
   machine consumes all the outputs of the first machine as inputs. *)
Definition seq (S1 S2 X Y Z: Type) (m1: mealy S1 X Y) (m2: mealy S2 Y Z) : mealy (S1 * S2) X Z :=
  (* destructuring bindings '(pat : typ) are indicated with a tick
     mark ' and allow you to put anything you'd put in a let
     expression or the left hand side of a pattern match into a
     function parameter. The type annotations here are optional
     because Coq can infer them from the return type of this function
     [mealy (S1 * S2) X Z]. *)
  fun '((s1, s2): S1 * S2) (x: X) =>
    let (y, s1') := m1 s1 x in
    let (z, s2') := m2 s2 y in
    (z, (s1', s2')).

(* Parallel composition of two mealy machines. *)
Definition par (S1 S2 U V X Y: Type) (m1: mealy S1 U V) (m2: mealy S2 X Y) : mealy (S1 * S2) (U * X) (V * Y) :=
  fun '((s1, s2): S1 * S2) '((u, x): U * X) =>
    let (v, s1') := m1 s1 u in
    let (y, s2') := m2 s2 x in
    ((v, y), (s1', s2')).

(** * Mealy Machine Semantics: Stream Functions *)
Section StreamFnInterp.
  Variable (S I O : Type).
  Variable (m: mealy S I O).

  (** Interpret a Mealy machine as a stream function. This requires
      you to provide an initial state [cur]. *)
  CoFixpoint interp (cur: S) : streams.stream I -> streams.stream O :=
    fun input =>
      match input with
      | i :.: rest =>
          obs m cur i :.: interp (step m cur i) rest
      end.

End StreamFnInterp.

    
