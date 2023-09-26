open Core
open Vcalyx
open Lexing
open Yojson

let load_mem (obj : Safe.t) : Extr.state =
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

let parse_and_print lexbuf source_location =
  match parse_with_error lexbuf with
  | Some _ ->
    Printf.printf "Successfully parsed %s.\n" source_location
    (* TODO: print AST back out *)
  | None ->
    exit (-1)

let vcx_parse : Command.t =
  let open Command.Let_syntax in
  Command.basic ~summary:"interpret a Calyx program with Coq semantics"
    [%map_open
      let source_arg = anon (maybe ("prog.futils" %: string))
      and data_arg = flag "-d" (optional string) ~doc:"JSON data for memories, etc" in
      fun () ->
        let _ = data_arg in (* todo use this *)
        let source_name = 
          match source_arg with
          | Some source_location -> source_location
          | None -> "<stdin>" in
        let source_chan =
          match source_arg with
          | Some source_location -> In_channel.create source_location
          | None -> In_channel.stdin in
        let source_str = Lexing.from_channel source_chan in
        source_str.lex_curr_p <-
          { source_str.lex_curr_p with pos_fname = source_name };
        parse_and_print source_str source_name;
        In_channel.close source_chan]

let vcx_cmd : Command.t =
  Command.group ~summary:"vcx: the vcalyx command-line interface"
    [ ("parse", vcx_parse) ]

let () = Command_unix.run ~version:"dev" vcx_cmd
