open Extr
open Core
(* open Vcalyx *)

(* module JsonTest = struct
     let spec = Command.Spec.(empty)
     let run () = Vcalyx.from_json;
   end

   let json_test : Command.t =
     let open Command.Let_syntax in
     Command.basic_spec ~summary:"test json parsing"
     JsonTest.spec
     JsonTest.run *)
let vcx_parse : Command.t =
  let open Command.Let_syntax in
  Command.basic ~summary:"interpret a Calyx program with Coq semantics"
    [%map_open
      let source_location = anon ("prog.futils" %: string) in
      fun () ->
        let source_chan = In_channel.create source_location in
        let source_str = In_channel.input_all source_chan in
        In_channel.close source_chan;
        match parse_context source_str with
        | Inl _ ->
          Printf.eprintf "Could not parse %s.\n" source_location;
          Printf.eprintf
            "Add a pretty printer for CeresDeserialize.error to see what \
             happened.\n"
        | Inr _ -> Printf.printf "Successfully parsed %s.\n" source_location]

let vcx_cmd : Command.t =
  Command.group ~summary:"vcx: the vcalyx command-line interface"
    [ ("parse", vcx_parse) ]
(* ("json-test", json_test)  *)

let () = Command_unix.run ~version:"dev" vcx_cmd
