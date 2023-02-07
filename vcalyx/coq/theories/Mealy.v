Require Calyx.Vect.
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
    tuples [V^n] and [V^m] respectively. *)
Definition tuple_mealy (V S: Type) (n m : nat) :=
  mealy S (Vect.vect V n) (Vect.vect V m).

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

    
