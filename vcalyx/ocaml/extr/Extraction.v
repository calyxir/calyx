(*! Extraction to OCaml !*)
Require Export Coq.extraction.Extraction.
From Coq.extraction Require Import
     ExtrOcamlBasic
     ExtrOcamlString
     ExtrOcamlNatInt
     ExtrOcamlZInt.
From VCalyx Require
     Syntax
     Parse.

(* This will extract all the listed identifiers and all their
transitive dependencies. *)
Extraction "extr.ml"
           VCalyx.Syntax.context
           VCalyx.Parse.parse_context.

