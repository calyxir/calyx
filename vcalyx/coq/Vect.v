(** * Vectors *)

(** This vector type indexes the tuple types [unit], [T * unit], [T *
    T * unit], etc., as a fixpoint. The approach in the Coq standard
    library uses an inductive type. We use a fixpoint so that for
    particular values of [n] the type [vect T n] is _definitionally
    equal_ to the corresponding tuple type [T * T * ... * unit],
    rather than merely isomorphic. *)
Fixpoint vect T n : Type :=
  match n with
  | 0 => unit
  | S n => vect T n * T
  end.

Definition bitvec n : Type :=
  vect bool n.
