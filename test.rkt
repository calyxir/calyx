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
   [const 1 : 32 -> sub @ right]
   [in -> sub @ left]
   [sub @ out -> out])
  []
  [])

(compute (decr) '((in . 0)))

(define/module test ((in : 32) (other : 32)) ((out : 32))
  ([reg = new comp/reg]
   [in -> reg @ in]
   [other -> reg @ in]
   [reg @ out -> out])
  [(in) (other)]
  []
  [(other)]
  [])
(animate (test) '((in . 10) (other . 20)))

(require "futil.rkt" "futil-prims.rkt")
(define/module counter ((n : 32)) ((out : 32))
  ([sub = new comp/sub]
   [reg = new comp/reg]
   [n -> reg @ in]
   [sub @ out -> reg @ in]
   [reg @ out -> sub @ left]
   [const decr 1 : 32 -> sub @ right]
   [const on 1 : 32 -> out]
   [const off 0 : 32 -> out])
  [(on)]
  [(while (sub out)
     ([(a) (b) (c)]
      [(d) (e) (if (a b) ([(c)]) ([(d)]))]))]
  [(n) (on)]
  [(n) (on)]
  [(n) (on)]
  [(n) (on)]
  [(n) (on)]
  [(n) (if (sub out)
           ([(n) (out)]
            [(hi)])
           ([(bye) (there)]))])

(component-control (counter))

(datum->syntax 1)

(component-control (counter))
(compute (counter) '((n . 5)))
(animate (counter) '((n . 5)))

(struct test-struct (x y z))
(struct other-struct (a b c))

(define-syntax (foo-struct stx)
  (define-syntax-class make-struct
    #:datum-literals (make)
    (pattern (make name:id a b c)
             #:with val #'(test-struct a b c)))
  (syntax-parse stx
    [(_ ms:make-struct)
     #'(ms.val)]))
(use-struct 1 2 3)

;; the input to transform can't use memory if module is not active

