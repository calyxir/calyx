#lang racket
(require "port.rkt"
         "component.rkt"
         "futil.rkt"
         "futil-prims.rkt"
         "dis-graphs.rkt")

(require "futil.rkt" "futil-prims.rkt" "dis-graphs.rkt")
(require "futil-prims.rkt")
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
  [(add) (id)]
  [(id)]
  [])

(component-control (triv))
(compute (triv) '((a . 1) (b . 2)))

;; add -- id
;; (define (triv-p)
;;   (define comp (triv))
;;   (define control
;;     (list
;;      (control-pair '(add id) '())
;;      (control-pair '(id) '())
;;      (control-pair '() '())))
;;   (set-component-control! comp control)
;;   comp)
;; (compute (triv-p) '((a . 30) (b . 2)))
;; (plot (triv))

;; (define/module add4 ((a : 32) (b : 32) (c : 32) (d : 32)) ((out : 32))
;;   ([add1 = new comp/add]
;;    [add2 = new comp/add]
;;    [add3 = new comp/add]
;;    [a -> add1 @ left]
;;    [b -> add1 @ right]
;;    [c -> add2 @ left]
;;    [d -> add2 @ right]
;;    [add1 @ out -> add3 @ left]
;;    [add2 @ out -> add3 @ right]
;;    [id = new id]
;;    [add3 @ out -> id @ in]
;;    [id @ out -> out]))
;; (compute-step (add4) (input-hash '((a . 1) (b . 2) (c . 3) (d . 4))) '(a b c d))
;; (plot (add4))

(define/module mux ((a : 32) (b : 32) (c : 1)) ((out : 32))
  ([a -> out]
   [b -> out])
  [(if (c inf#) a b)])
(component-control (mux))
(compute (mux) '((a . 20) (b . 10) (c . 1)))
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


