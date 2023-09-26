(*! Extraction to OCaml !*)
Set Warnings "-extraction-reserved-identifier".
From VCalyx Require
     IRSyntax
     Parse
     Semantics.
Require Coq.extraction.Extraction.
From Coq.extraction Require Import
     ExtrOcamlBasic
     ExtrOcamlNativeString
     ExtrOcamlNatInt
     ExtrOcamlZInt.

Extract Constant VCalyx.Parse.oops => "(fun _ -> failwith ""oops!"")".

(* This will extract all the listed identifiers and all their
transitive dependencies. *)
Extraction "extr.ml"
           VCalyx.Semantics.interp_context
           VCalyx.IRSyntax.context
           VCalyx.IRSyntax.is_in
           VCalyx.IRSyntax.is_out.
