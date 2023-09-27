From stdpp Require Import
     base
     strings.

Definition exn (A: Type) : Type :=
  A + string.
#[global]
Instance exn_FMap : FMap exn :=
  fun _ _ f x =>
    match x with
    | inl a => inl (f a)
    | inr exn => inr exn
    end.

#[global]
Instance exn_MRet : MRet exn :=
  fun _ => inl.

#[global]
Instance exn_MBind : MBind exn :=
  fun _ _ k c =>
    match c with
    | inl a => k a
    | inr exn => inr exn
    end.

Definition lift_opt {A} (msg: string) (o: option A) : exn A :=
  match o with
  | Some a => inl a
  | None => inr msg
  end.
