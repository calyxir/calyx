#lang racket
(require "component.rkt"
         "futil.rkt"
         "futil-prims.rkt"
         "dis-graphs.rkt")

(require "futil.rkt" "futil-prims.rkt" "dis-graphs.rkt")
(define/module id ((in : 32)) ((out : 32))
  ([in -> out])
  [])

(define/module triv ((a : 32) (b : 32)) ((out : 32))
  ([add = new comp/add]
   [id = new id]
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
   [id = new id]
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

(animate (times4) '((a . 10)))

(define/module loop ((a : 32)) ((out : 32))
  ([id1 = new id]
   [id2 = new id]
   [id3 = new id]
   [a -> id1 @ in]
   [id1 @ out -> id2 @ in]
   [id2 @ out -> id3 @ in]
   [id3 @ out -> id1 @ in]
   [id2 @ out -> out])
  [(a)]
  [(id1)]
  [(id2)]
  [(id3)]
  [(id1)]
  [(id2)]
  [(id3)])

(remove* '(id1 id2) '(id1 id2 id1 id2 a))

(component-control (loop))

(plot (loop))

(animate (loop) '((a . 10)))
(plot (times4))


;; (plot (add4))

;; (add2 -- add3)
;; (compute-step (add4) (input-hash '((a . 1) (b . 2) (c . 3) (d . 4))) '(a b c d))
;; (plot (add4))

;; (define/module mux ((a : 32) (b : 32) (c : 1)) ((out : 32))
;;   ([a -> out]
;;    [b -> out])
;;   [(if (c inf#) a b)])
;; (compute (mux) '((a . 20) (b . 10) (c . 1)))
;; (define (mux-p)
;;   (define comp (mux))
;;   (define control
;;     (list
;;      (cons '() (list (constr '(c . inf#) 'a 'b)))))
;;   (set-component-control! comp control)
;;   comp)
;; (plot (mux))
;; (get-edges (convert-graph (mux)))

;; (compute-step (mux) (input-hash '((a . 2) (b . 10) (c . 0))) '() (list (constr '(c . inf#) 'a 'b)))
;; (compute (mux-p) (input-hash '((a . 2) (b . 10) (c . 1))))

;; (define/module times4 ((a : 32)) ((out : 32))
;;   ([add = new comp/add]
;;    [a -> add @ left]
;;    [a -> add @ right]
;;    [add @ out -> out]
;;    [add @ out -> add @ right]))
;; (plot (times4))

;; (define in (input-hash (times4) '((a . 10))))
;; (define-values (h1 res) (compute-step (times4) in '() '()))
;; (define-values (h2 res2) (compute-step (times4) h1 '() (list (constr '(add . out) ))))
;; (define-values (h3 res3) (compute-step (times4) h2 '() '()))


