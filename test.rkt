#lang racket
(require "port.rkt"
         "component.rkt"
         "futil.rkt"
         "futil-prims.rkt"
         "dis-graphs.rkt")

(require "futil.rkt" "futil-prims.rkt" "dis-graphs.rkt")
(define/module pre_id ((in : 32)) ((out : 32))
  ([in -> out]))

(define (id)
  (define comp (pre_id))
  (set-component-proc! comp
                       (keyword-lambda (in)
                                       [out = in]))
  comp)

(define/module triv ((a : 32) (b : 32)) ((out : 32))
  ([add = new comp/add]
   [id = new id]
   [control a1 = add]
   [control id = id]
   [a -> add @ left]
   [b -> add @ right]
   [add @ out -> id @ in]
   [id @ out -> out]))
(compute (triv) (input-hash '((a . 30) (b . 2))) '(a))
(plot (triv))

(define/module add4 ((a : 32) (b : 32) (c : 32) (d : 32)) ((out : 32))
  ([add1 = new comp/add]
   [add2 = new comp/add]
   [add3 = new comp/add]
   [a -> add1 @ left]
   [b -> add1 @ right]
   [c -> add2 @ left]
   [d -> add2 @ right]
   [add1 @ out -> add3 @ left]
   [add2 @ out -> add3 @ right]
   [id = new id]
   [add3 @ out -> id @ in]
   [id @ out -> out]))
(compute (add4) (input-hash '((a . 1) (b . 2) (c . 3) (d . 4))) '(a b c d))
(plot (add4))

(define/module mux ((a : 32) (b : 32) (c : 1)) ((out : 32))
  ([a -> out]
   [b -> out]))
(plot (mux))
(get-edges (convert-graph (mux)))

;; (compute (mux) (input-hash '((a . 1) (b . 10) (c . 1))) '(out) '((constr 'c 'a 'b)))
