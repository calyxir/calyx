#lang racket/base

(require racket/list
         racket/format
         "../src/futil.rkt")

(generate-json
 "pipeline.data"
 (random 1 20)
 (A 10)
 (B 10))

;; pipeline-for (let i = 0..10) {
;;   let a = A[i];
;;   ---
;;   let b = a + 10;
;;   let c = b * a;
;;   let d = c / b;
;; }

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

(define/module main () ()
  ([A = new comp/memory1d]
   [B = new comp/memory1d]
   [const A-data #f : 32 -> A @ data-in]

   [i = new counter]
   [const i-en 1 : 32 -> i @ en]
   [const i-res 1 : 32 -> i @ res]

   [min-3 = new comp/trunc-sub]
   [i @ out -> min-3 @ left]
   [const three 3 : 32 -> min-3 @ right]

   [stop = new comp/trunc-sub]
   [const stop-val 13 : 32 -> stop @ left]
   [i @ out -> stop @ right]

   [a0 = new comp/reg]
   [i @ out -> A @ addr]
   [A @ out -> a0 @ in]
   [a1 = new comp/reg]
   [a0 @ out -> a1 @ in]

   [b = new comp/reg]
   [add = new comp/add]
   [a0 @ out -> add @ left]
   [const ten 10 : 32 -> add @ right]
   [add @ out -> b @ in]

   [c = new comp/reg]
   [mult = new comp/mult]
   [b @ out -> mult @ left]
   [a1 @ out -> mult @ right]
   [mult @ out -> c @ in]

   [div = new comp/div]
   [c @ out -> div @ left]
   [const two 2 : 32 -> div @ right]
   [min-3 @ out -> B @ addr]
   [div @ out -> B @ data-in])
  [(!! i i-en i-res stop stop-val)]
  [(while (stop out)
     ([(i-en i-res)]
      [(!! i-en i stop stop-val)]))]
  [(mem-print B)])

(require "../src/visualizer.rkt")
;; (plot-component (main))

(void
 (plot-compute
  (main) '()
  #:memory (json->memory "pipeline.data")))
