(*! Extraction to OCaml !*)
From VCalyx Require
     Syntax
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
           VCalyx.Syntax.context
           VCalyx.Parse.parse_context.
