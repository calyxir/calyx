open Extr
let () =
  let c: control = CEmpty () in
  match c with
  | CEmpty tt -> Vcalyx.f tt
  | _ -> ()
