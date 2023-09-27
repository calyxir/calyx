open Core
open Vcalyx
open Lexing

let load_mem obj : Extr.state =
  let open Yojson.Safe.Util in
  let mem_data = obj
                 |> member "data" 
                 |> to_list
                 |> List.map ~f:to_int in
  Extr.StateMemD1 (false,
                   {is_signed = false;
                    numeric_type = Bitnum;
                    width = 32},
                   mem_data)

let load_mems file =
  let obj = Yojson.Safe.from_file file in
  match obj with
  | `Assoc kvs ->
    List.map ~f:(fun (k, v) -> (k, load_mem v)) kvs
  | _ -> failwith "unexpected JSON object"

let dump_mem =
  function
  | Extr.StateMemD1 (_, _, mem_data) ->
    `List (List.map ~f:(fun i -> `Int i) mem_data)
  | _ -> failwith "unexpected @external state to dump"

let dump_mems chan mems =
  let kvs = List.map ~f:(fun (k, v) -> (k, dump_mem v)) mems in
  Yojson.Safe.to_channel chan (`Assoc kvs)

(* from https://dev.realworldocaml.org/parsing-with-ocamllex-and-menhir.html *)
let print_position outx lexbuf =
  let pos = lexbuf.lex_curr_p in
  fprintf outx "%s:%d:%d" pos.pos_fname pos.pos_lnum
    (pos.pos_cnum - pos.pos_bol + 1)

let parse_with_error lexbuf =
  try Some (Parser.main Lexer.tokens lexbuf) with
  (* | SyntaxError msg ->
    fprintf stderr "%a: %s\n" print_position lexbuf msg;
    None *)
  | Parser.Error ->
    fprintf stderr "%a: syntax error\n" print_position lexbuf;
    None

let parse_context lexbuf source_location : Extr.context =
  match parse_with_error lexbuf with
  | Some ctx -> ctx
  | None -> failwith (Printf.sprintf "Error parsing %s." source_location)

let interp_exn ctx mems_initial =
  match Extr.interp_with_mems ctx mems_initial 100000 with
  | Inl mems_final -> mems_final
  | Inr msg -> failwith msg

let vcx_parse : Command.t =
  let open Command.Let_syntax in
  Command.basic ~summary:"interpret a Calyx program with Coq semantics"
    [%map_open
      let source_arg = anon (maybe ("prog.futils" %: string))
      and data_arg = flag "-d" (optional string) ~doc:"JSON data for memories, etc" in
      fun () ->
        let mems_initial =
          match data_arg with
          | Some data_arg -> load_mems data_arg
          | None -> []
        in (* todo use this *)
        let source_name = 
          match source_arg with
          | Some source_location -> source_location
          | None -> "<stdin>" in
        let source_chan =
          match source_arg with
          | Some source_location -> In_channel.create source_location
          | None -> In_channel.stdin in
        let source_lexbuf = Lexing.from_channel source_chan in
        Lexing.set_filename source_lexbuf source_name;
        let ctx = parse_context source_lexbuf source_name in
        In_channel.close source_chan;
        (* need to print out the result appropriately and include # of cycles... *)
        let mems_final = interp_exn ctx mems_initial in
        dump_mems Out_channel.stdout mems_final]

let vcx_cmd : Command.t =
  Command.group ~summary:"vcx: the vcalyx command-line interface"
    [ ("parse", vcx_parse) ]

let () = Command_unix.run ~version:"dev" vcx_cmd
