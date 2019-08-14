#lang racket/base

(require futil)

;; (generate-json
;;  "linear-algebra-atax.data"
;;  (random 1 10)
;;  (A 8 8)
;;  (x 8)
;;  (y 8)
;;  (tmp 8))

(define/module main () ()
  (; decls
   [A = new comp/memory2d]   ; |A| = M x N
   [x = new comp/memory1d]   ; |x| = N
   [y = new comp/memory1d]   ; |y| = N
   [tmp = new comp/memory1d] ; |tmp| = M

   ; adder and mult
   [add = new comp/add]
   [mult = new comp/mult]

   [i0 = new comp/counter-up]
   [const N0 8 : 32 -> i0 @ in]
   [const i0-en 1 : 32 -> i0 @ en]
   [const y-data 0 : 32 -> y @ data-in]
   [i0 @ out -> y @ addr]

   [i1 = new comp/counter-up]
   [const M0 8 : 32 -> i1 @ in]
   [const i1-en 1 : 32 -> i1 @ en]

   [j0 = new comp/counter-up]
   [const N1 8 : 32 -> j0 @ in]
   [const j0-en 1 : 32 -> j0 @ en]

   [tmp-t = new comp/reg]
   [tmp @ out -> tmp-t @ in]
   [tmp-t @ out -> add @ left]
   [A @ out -> mult @ left]
   [x @ out -> mult @ right]
   [mult @ out -> add @ right]
   [add-buf = new comp/id]
   [add @ out -> add-buf @ in]
   [add-buf @ out -> tmp @ data-in]

   [j1 = new comp/counter-up]
   [const N2 8 : 32 -> j1 @ in]
   [const j1-en 1 : 32 -> j1 @ en]
   [y-y0 = new comp/reg]
   [y @ out -> y-y0 @ in]
   [y-y0 @ out -> add @ left]
   [tmp-buf = new comp/id]
   [tmp @ out -> tmp-buf @ in]
   [tmp-buf @ out -> mult @ right]
   [add @ out -> y @ data-in]

   ; array connections
   [i1 @ out -> A @ addr1]
   [j0 @ out -> A @ addr2]
   [j1 @ out -> A @ addr2]
   [const A-data #f : 32 -> A @ data-in]
   [const x-data #f : 32 -> x @ data-in]

   [i1 @ out -> tmp @ addr]
   [const tmp-data 0 : 32 -> tmp @ data-in]

   [j0 @ out -> x @ addr]

   [j1 @ out -> y @ addr])
  [(!! N0 i0 i0-en)]                                  ; init i = 0..N
  [(while (i0 @ stop)
     ([(!! i0 y-data y)]                              ; y[i] := 0.0
      [(!! i0 i0-en)]))]                              ; i++

  [(!! M0 i1 i1-en)]                                  ; init i = 0..M
  [(while (i1 @ stop)
     ([(!! tmp i1 tmp-data)]                          ; tmp[i] := 0.0

      [(!! N1 j0 j0-en)]                              ; init j = 0..N
      [(while (j0 @ stop)
         ([(!! tmp-t tmp i1)]                         ; let t = tmp[i]
          [(!! tmp i1 tmp-t add add-buf A j0 mult x)] ; tmp[i] := t + A[i][j] * x[j]
          [(!! j0 j0-en)]))]                          ; j++

      [(!! N2 j1 j1-en)]                              ; init j = 0..N
      [(while (j1 @ stop)
         ([(!! y-y0 y j1)]                            ; let y0 = y[j]
          [(!! y j1 y-y0 add A i1 mult tmp tmp-buf)]  ; y[j] := y0 + A[i][j] * tmp[i]
          [(!! j1 j1-en)]))]                          ; j++

      [(!! i1 i1-en)])                                ; i++
     )]

  [(mem-print y)])

(parse-cmdline (main))
