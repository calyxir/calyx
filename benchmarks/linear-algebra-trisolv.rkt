#lang racket/base

(require racket/path
         racket/list
         racket/format
         "../src/futil.rkt"
         "../src/vizualizer.rkt")

;; I don't think that this module is possible to define at the moment
;; because a submodule has no way of changing the state of the parent module
;; XXXX XXXX XXXX XXXX
;; (define/module j_lt_i ((j : 32) (i : 32)) )

;; registers are local variables
;; I think that I need global variables

(define/module counter ((en : 32) (res : 32)) ((out : 32))
  ([add = new comp/add]
   [reg = new comp/reg]

   [const one 1 : 32 -> add @ left]
   [const zero 0 : 32 -> reg @ in]
   [reg @ out -> add @ right]
   [add @ out -> reg @ in]
   [reg @ out -> out])
  [(!! reg res)]
  [(ifen (res inf#)
         ([(!! zero reg out)])
         ([(ifen (reg out)
                 ([(ifen (en inf#)
                         ([(!! one add reg out)])
                         ([(!! reg out)]))])
                 ([(!! zero reg out)]))]))]
  [(!! reg out)])

; XXX changed? should really be the timestamps changing

(define/module main ((n : 32)) ()
  (; memory
   [L = new comp/memory2d]
   [x = new comp/memory1d]
   [b = new comp/memory1d]

   ; buffers
   [iL_buf = new comp/id]
   [jx_buf = new comp/id]

   ; registers
   [x_j = new comp/reg]
   [x_i = new comp/reg]

   ; increments
   [i = new comp/counter-up]
   [j = new counter]

   ; mathz
   [mult = new comp/mult]
   [div = new comp/div]
   [i-min-j = new comp/trunc-sub]

   ; i connections
   [n -> i @ in]
   [const i-en 1 : 32 -> i @ en]
   [i @ out -> x @ addr]
   [i @ out -> b @ addr]
   [i @ out -> L @ addr1]
   [i @ out -> iL_buf @ in]
   [iL_buf @ out -> L @ addr2]
   [i @ out -> i-min-j @ left]
   ;; [i-stop = new comp/id]
   ;; [i @ stop -> i-stop @ in]

   ; j connections
   [const j-en 1 : 32 -> j @ en]
   [const j-res 1 : 32 -> j @ res]
   [j @ out -> jx_buf @ in]
   [jx_buf @ out -> x @ addr]
   [j @ out -> L @ addr2]
   [j @ out -> i-min-j @ right]

   ; b connections
   [const b-data #f : 32 -> b @ data-in]
   [b @ out -> x @ data-in]

   ; x_j register
   [x @ out -> x_j @ in]

   ; x_i register
   [x @ out -> x_i @ in]

   ; mult connections
   [x_j @ out -> mult @ left]
   [L @ out -> mult @ right]
   [mult @ out -> x @ data-in]

   ; div connections
   [x_i @ out -> div @ left]
   [L @ out -> div @ right]
   [div @ out -> x @ data-in]

   ; L connections
   [const L-data #f : 32 -> L @ data-in])

  [(!! n i-en i)]
  [(while (i stop)
     ([(!! i x b)]                                   ; x[i] := b[i]; and i++;
      ; do loop here
      [(!! i j j-en j-res i-min-j)]                  ; let j = 0
      [(while (i-min-j out)
         ([(!! j jx_buf x x_j)]                      ; let x_j = x[j];
          [(!! x i L j x_j mult)]                    ; x[i] := L[i][j] * x_j;
          [(!! j-en j i-min-j i)]))]                 ; j := j + 1;
      [(!! x_i i x)]                                 ; let x_i = x[i]
      [(!! x x_i i iL_buf L div)]                    ; x[i] := x_i / L[i][i];
      [(!! i-en i)]                                  ; increment i (from for loop)
      ))]
  [(mem-print x)])

(define fn
  (if (= 0 (vector-length (current-command-line-arguments)))
      (simplify-path
       (build-path (current-directory) ".." "benchmarks" "linear-algebra-trisolv.data"))
      (build-path (current-directory) (vector-ref (current-command-line-arguments) 0))))
(void
 (compute
  (main) '((n . 9))
  #:memory (json->memory fn)))
