open Extr
open Core

let vcx_cmd : Command.t =
  let open Command.Let_syntax in
  Command.basic ~summary:"vcx: the vcalyx command-line interface"
    [%map_open
     let source_location = anon ("prog.futils" %: string) in
     fun () ->
     begin
       let source_chan = In_channel.create source_location in
       let source_str = In_channel.input_all source_chan in
       In_channel.close source_chan;
       match parse_context source_str with
       | Inl _ ->
          Printf.eprintf "Could not parse %s.\n" source_location;
          Printf.eprintf "Add a pretty printer for CeresDeserialize.error to see what happened.\n"
       | Inr _ ->
          Printf.printf "Successfully parsed %s.\n" source_location
     end]

let () = Command_unix.run ~version:"dev" vcx_cmd
