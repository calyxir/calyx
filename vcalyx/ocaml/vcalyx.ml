(* open Yojson.Basic *)

let f () = Printf.printf "\nhello world\n"
(*
   let raise_invalid_arg str json =
     Invalid_argument (Printf.sprintf "%s: %s" str (to_string json)) |> raise

   type format = { numeric_type : string; is_signed : bool; width : int }
   [@@deriving show]

   type mem = { name : string; data : data; format : format }
   [@@deriving show]

   type mems = mem list

   let unwrap_int (json : t) : int =
     match json with `Int v -> v | _ -> failwith "unsupported type"

   let unwrap_bool (json : t) : bool =
     match json with `Bool v -> v | _ -> failwith "unsupported type"

   let unwrap_string (json : t) : string =
     match json with `String v -> v | _ -> failwith "unsupported type"

   let parse_data mem =
     match mem with
     | `List lst -> List.map unwrap_int lst
     | _ -> raise_invalid_arg "mem not a list" mem

   let parse_all_data data : data =
     match data with
     | `List mems -> List.map parse_data mems
     | _ -> raise_invalid_arg "unexpected memory type" data

   let parse_format fmt =
     let numeric_type = fmt |> Util.member "numeric_type" |> unwrap_string in
     let is_signed = fmt |> Util.member "is_signed" |> unwrap_bool in
     let width = fmt |> Util.member "width" |> unwrap_int in
     { numeric_type; is_signed; width }

   let parse_mem (json : string * t) : mem =
     match json with
     | name, `Assoc [ ("data", data); ("format", format) ] ->
       {name; data = parse_all_data data; format = parse_format format}
     | _ -> raise_invalid_arg "unexpected memory arg" (`Assoc [ json ])

   let json_helper (json : t) : mem list =
     match json with
     | `Assoc lst -> List.map parse_mem lst
     | _ -> raise_invalid_arg "unexpected input" json

   let from_json file =
     let (json : t) = from_file file in
     json_helper json

   let to_json_data data =
     let json_data = (List.map data ~f:(List.map d ~f:(fun d -> `Int d)))

   let to_json_mem mem =
     `Assoc [(mem.name, `Assoc [ ("data", to_json_data mem.data); ("format", to_json_format mem.format) ] )]

   let to_json mems =
     let json_mems = List.map to_json_mem mems *)

(*
     type t = [
   | `Null
   | `Bool of bool
   | `Int of int
   | `Float of float
   | `String of string
   | `Assoc of (string * t) list
   | `List of t list
    ] *)
