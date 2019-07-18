#lang racket
(require "component.rkt"
         "futil.rkt"
         "futil-prims.rkt"
         "dis-graphs.rkt")

(require "futil.rkt" "futil-prims.rkt")
(define/module triv ((a : 32) (b : 32)) ((out : 32))
  ([add = new comp/add]
   [id = new comp/id]
   [a -> add @ left]
   [b -> add @ right]
   [add @ out -> id @ in]
   [id @ out -> out])
  [(add)]
  [(id)])

;; [(add)]
;; [(id)]

;; (component-control (triv))
;; (convert-graph (triv) (list-ref (car (compute (triv) '((a . 1) (b . 2)))) 1))
(plot (triv) (list-ref (car (compute (triv) '((a . 1) (b . 2)))) 2) '(a))


(animate (triv) '((a . 1) (b . 2)))


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
   [id = new comp/id]
   [add3 @ out -> id @ in]
   [id @ out -> out])
  [(a) (b) (c) (d)]
  [(add3) (add2) (add1)]
  [(id)])

(plot (add4) )
(animate (add4) '((a . 1) (b . 2) (c . 3) (d . 4)))

(define/module times4 ((a : 32)) ((out : 32))
  ([add = new comp/add]
   [a -> add @ left]
   [const 0 : 32 -> add @ right]
   [id = new id]
   [add @ out -> id @ in]
   [id @ out -> add @ right]
   [id @ out -> out])
  []
  [(if (add out) 0 id)]
  [(if (add out) 0 id)]
  [(if (add out) 0 id)]
  [(if (add out) 0 id)])

(while (wire port)
  [()])

(define/module decr ((in : 32)) ((out : 32))
  ([sub = new comp/sub]
   [const one 1 : 32 -> sub @ right]
   [in -> sub @ left]
   [sub @ out -> out])
  [])

(ast-tuple-state (compute (decr) '((in . 1))))

(animate (test) '((in . 10) (other . 20)))

(require "futil.rkt" "futil-prims.rkt" "dis-graphs.rkt")
(define/module counter2.0 ((in : 32)) ((out : 32))
  ([sub = new comp/sub]
   [reg = new comp/reg]
   [in -> sub @ left]
   [const decr 1 : 32 -> sub @ right]
   [sub @ out -> reg @ in]
   [reg @ out -> sub @ left]
   [reg @ out -> out])
  [(ifen (in inf#)
         ([])
         ([(in)]))])
(component-control (counter2.0))
(plot-compute (counter2.0) '((in . 10)))

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
(plot-compute (mult) '((a . 10) (b . 7)))

(require "futil.rkt" "futil-prims.rkt" "dis-graphs.rkt")
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

;; [(a) (b) ...] means, (merge (ast-step a) (ast-step b) ...)
;; (merge ...) merges inactive modules by merging lists and removing duplicates
;;             merges state by merging hashs and failing if two states write different vals to the same wire
;;             merges memory by
