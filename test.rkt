#lang racket
(require "ast.rkt"
         "futil.rkt"
         "futil-prims.rkt"
         "dis-graphs.rkt")

(define/module decr ((in : 32)) ((out : 32))
  ([sub = new comp/trunc-sub]
   [const one 1 : 32 -> sub @ right]
   [in -> sub @ left]
   [sub @ out -> out])
  [])

;; (ast-tuple-state (compute (decr) '((in . 1))))

;; (require "futil.rkt" "futil-prims.rkt" "dis-graphs.rkt")
(define/module counter2.0 ((in : 32)) ((out : 32))
  ([sub = new comp/trunc-sub]
   [reg = new comp/reg]
   [in -> sub @ left]
   [const decr 1 : 32 -> sub @ right]
   [sub @ out -> reg @ in]
   [reg @ out -> sub @ left]
   [reg @ out -> out])
  [(ifen (in inf#)
         ([])
         ([(in)]))])
;; (component-control (counter2.0))
;; (plot-compute (counter2.0) '((in . 10)))

(define/module consumer ((n : 32)) ((out : 32))
  ([counter = new counter2.0]
   [viz = new comp/id]
   [n -> counter @ in]
   [counter @ out -> viz @ in]
   [const on 1 : 32 -> out])
  [(on)]
  [(while (counter out)
     ([(n on)]))]
  [(n)])
(plot-compute (consumer) '((n . 10)))

(define/module mult ((a : 32) (b : 32)) ((out : 32))
  ([counter = new counter2.0]
   [add = new comp/add]
   [reg = new comp/reg]
   [viz = new comp/id]

   [b -> counter @ in]
   [counter @ out -> viz @ in]

   [const zero 0 : 32 -> add @ left]
   [a -> add @ right]
   [add @ out -> reg @ in]
   [reg @ out -> add @ left]
   [reg @ out -> out])
  []
  [(while (counter out) ([(b zero)]))])
(plot-compute (mult) '((a . 3) (b . 4)))

;; (require "futil.rkt" "futil-prims.rkt" "dis-graphs.rkt")
(define/module simp ((a : 32) (b : 32)) ((out : 32))
  ([add = new comp/add]
   [a -> add @ left]
   [b -> add @ right]
   [add @ out -> out])
  [(a)]
  [(b)]
  []
  [(a) (b)])

(plot-compute (simp) '((a . 10) (b . 20)))

(plot (simp) (ast-tuple-history
              (compute (simp) '((a . 10) (b . 20)))))
