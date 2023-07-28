(*! Extraction to OCaml !*)
From VCalyx Require
     IRSyntax
     Parse.
Require Export Coq.extraction.Extraction.
From Coq.extraction Require Import
     ExtrOcamlBasic
     ExtrOcamlNativeString
     ExtrOcamlNatInt.

Extract Constant VCalyx.Parse.oops => "(fun _ -> failwith ""oops!"")".

(* This will extract all the listed identifiers and all their
transitive dependencies. *)
Extraction "extr.ml"
           VCalyx.IRSyntax.context
           VCalyx.IRSyntax.is_in
           VCalyx.IRSyntax.is_out
           VCalyx.Parse.parse_context.
