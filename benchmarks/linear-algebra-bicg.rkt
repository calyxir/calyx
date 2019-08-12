#lang racket/base

(require racket/list
         racket/format
         "../src/futil.rkt")

;; (generate-json
;;  "linear-algebra-bicg.data"
;;  (random 1 20)
;;  (A 8 8)
;;  (s 8)
;;  (q 8)
;;  (p 8)
;;  (r 8))

(define/module main () ()
  ([N = new comp/reg]
   [M = new comp/reg]
   [const N-val 8 : 32 -> N @ in]
   [const M-val 8 : 32 -> M @ in]

   [A = new comp/memory2d]
   [const A-data #f : 32 -> A @ data-in]
   [s = new comp/memory1d]
   [q = new comp/memory1d]
   [p = new comp/memory1d]
   [const p-data #f : 32 -> p @ data-in]
   [r = new comp/memory1d]
   [const r-data #f : 32 -> r @ data-in]

   [i = new comp/counter-up]
   [const i-en 1 : 32 -> i @ en]
   [M @ out -> i @ in]
   [const s-data 0 : 32 -> s @ data-in]
   [i-buf = new comp/id]
   [i @ out -> i-buf @ in]
   [i-buf @ out -> s @ addr]

   [N @ out -> i @ in]
   [i @ out -> q @ addr]
   [const q-data 0 : 32 -> q @ data-in]

   [j = new comp/counter-up]
   [const j-en 1 : 32 -> j @ en]
   [M @ out -> j @ in]

   [s0 = new comp/reg]
   [j @ out -> s @ addr]
   [s @ out -> s0 @ in]

   [q0 = new comp/reg]
   [i @ out -> q @ addr]
   [q @ out -> q0 @ in]

   [A_i_j = new comp/reg]
   [i @ out -> A @ addr1]
   [j @ out -> A @ addr2]
   [A @ out -> A_i_j @ in]

   [add1 = new comp/add]
   [mult1 = new comp/mult]
   [i @ out -> r @ addr]
   [r @ out -> mult1 @ right]
   [A_i_j @ out -> mult1 @ left]
   [mult1 @ out -> add1 @ left]
   [s0 @ out -> add1 @ right]
   [add1 @ out -> s @ data-in]

   [add2 = new comp/add]
   [mult2 = new comp/mult]
   [A_i_j @ out -> mult2 @ left]
   [j @ out -> p @ addr]
   [p @ out -> mult2 @ right]
   [mult2 @ out -> add2 @ right]
   [q0 @ out -> add2 @ left]
   [add2 @ out -> q @ data-in])
  [(!! N N-val M M-val)]
  [(!! i i-en M)]                                  ; let i = 0..M
  [(while (i stop)
     ([(!! s i i-buf s-data)]                      ; s[i] := 0
      [(!! i i-en)]))]                             ; i++

  [(!! i i-en N)]                                  ; let i = 0..N
  [(while (i stop)
     ([(!! q q-en i q-data)]                       ; q[i] := 0

      [(!! j j-en M)]                              ; let j = 0..M
      [(while (j stop)
         ([(!! s0 s j                              ; let s0 = s[j]
               q0 q i                              ; let q0 = q[i]
               A_i_j A i j)]                       ; let A_i_j = A[i][j]
          [(!! s j s0 add1 r i mult1 A_i_j         ; s[j] := s0 + r[i] * A_i_j
               q q0 p add2 mult2)]                 ; q[i] := q0 + A_i_j * p[j]
          [(!! j j-en)])                           ; j++
         )]

      [(!! i i-en)]))]
  [(mem-print s)]
  [(mem-print q)])

;; (require "../src/visualizer.rkt")
;; (plot-component (main))
(define fn (benchmark-data-path "linear-algebra-bicg.data"))

(void
 (compute (main) '()
               #:memory (json->memory fn)))
