open Core
open Vcalyx
open Lexing

(* from https://dev.realworldocaml.org/parsing-with-ocamllex-and-menhir.html *)
let print_position outx lexbuf =
  let pos = lexbuf.lex_curr_p in
  fprintf outx "%s:%d:%d" pos.pos_fname pos.pos_lnum
    (pos.pos_cnum - pos.pos_bol + 1)

let parse_with_error lexbuf =
  try Parser.main Lexer.tokens lexbuf with
  (* | SyntaxError msg ->
    fprintf stderr "%a: %s\n" print_position lexbuf msg;
    None *)
  | Parser.Error ->
    fprintf stderr "%a: syntax error\n" print_position lexbuf;
    exit (-1)

let rec parse_and_print source_str source_location =
  match parse_with_error source_str with
  | Some _ ->
    Printf.printf "Successfully parsed %s.\n" source_location;
    parse_and_print source_str source_location
  | None -> ()

let vcx_parse : Command.t =
  let open Command.Let_syntax in
  Command.basic ~summary:"interpret a Calyx program with Coq semantics"
    [%map_open
      let source_location = anon ("prog.futils" %: string) in
      fun () ->
        let source_chan = In_channel.create source_location in
        let source_str = Lexing.from_channel source_chan in
        source_str.lex_curr_p <-
          { source_str.lex_curr_p with pos_fname = source_location };
        parse_and_print source_str source_location;
        In_channel.close source_chan]

let vcx_cmd : Command.t =
  Command.group ~summary:"vcx: the vcalyx command-line interface"
    [ ("parse", vcx_parse) ]

let () = Command_unix.run ~version:"dev" vcx_cmd
