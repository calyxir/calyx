#lang racket
(require "port.rkt"
         "component.rkt"
         "futil.rkt"
         "futil-prims.rkt")

;; (define/module prog2 () ((out : 32))
;;   ([foo1 = new foo]
;;    [foo2 = new foo]
;;    [foo3 = new foo]
;;    [const 1 : 32 -> hole c1]
;;    [const 2 : 32 -> hole c2]
;;    [const 3 : 32 -> hole c3]
;;    [hole in-a -> foo1 @ in]
;;    [hole in-b -> foo2 @ in]
;;    [hole in-c -> foo3 @ in]
;;    [foo1 @ out -> hole out-a]
;;    [foo2 @ out -> hole out-b]
;;    [foo3 @ out -> hole out-c]
;;    [hole d -> out])
;;   ((seq (par [in-a = c1]
;;              [in-b = c2])
;;         [in-c = c3])
;;    [(if out-c
;;         [d = out-a]
;;         [in-b = out-b])]))

;; [d = out-a (if out-c)]
;; [in-b = out-b (ifnot out-c)]

;; (define/module prog ((a : 32) (c : 1)) ((out : 32))
;;   ([add1 = new add1]
;;    [add2 = new add2]
;;    [mux = new mux]
;;    [c -> mux @ control];; [c >>= control]
;;    ;; [h = hole]
;;    [a -> add1 @ a]
;;    [a -> add2 @ a]
;;    [add1 @ out -> mux @ right] ;; [add1 @ out >>= one]
;;    [add2 @ out -> mux @ left]  ;; [add2 @ out >>= hole two]
;;    [mux @ out -> out];; [hole out -> out]
;;    )
;;   ;; ([if c
;;   ;;      out = one
;;   ;;      out = two])
;;   )

(require "futil.rkt" "futil-prims.rkt")
(define/module triv ((a : 32) (b : 32) (c : 32)) ((out : 32))
  ([add = new comp/add]
   [add2 = new comp/add]
   [a -> add @ left]
   [b -> add @ right]
   [add @ out -> add2 @ left]
   [c -> add2 @ right]
   [add2 @ out -> out]
   ))
(plot (triv))

;; (tsort (convert-graph (triv)))
;; (get-edges (convert-graph (triv)))
;; (require graph)
;; (define g (convert-graph (triv)))
;; (tsort g) ;; want '((a b) (add) (out))
;; (sequence->list (in-neighbors (transpose g) 'add))

;; (compute (prog) (make-hash '((a . 6) (c . 0))))

;; (syntax->datum (expand '(keyword-lambda (x y z) (+ x y z))))
  ; or
  ;; [out = one]
  ; or
  ;; [out = two]

;; (define/module foo ((in : 32)) ((out : 32))
;;   ([in -> out]))

;; (require graph)
;; (define/module test ((a : 32) (c : 1)) ((out : 32))
;;   ([a -> hole a]
;;    [foo = new foo]
;;    [hole a-in -> foo @ in]
;;    [foo @ out -> hole a-out]
;;    [c -> hole c]
;;    [hole o -> out])
;;   [a-in = a]
;;   [o = a-out (when c)])

;; (define/module test2 ((a : 32) (b : 32) (c : 1)) ((out : 32))
;;   ([a -> hole in-a]
;;    [b -> hole in-b]
;;    [c -> hole con]
;;    [hole out -> out])
;;   [out = a]
;;   [out = b (when c)])

;; (compute (test2) (make-hash '((a . 10) (b . 20) (c . 1))))

;; (follow-holes (test) 'foo)

;; (map (lambda (x)
;;        (match x
;;          [(cons u (cons (cons v _) _))
;;           (println (~v u v))]))
;;  (follow-holes (test) 'out))
;; (component-holes (test))

;; (stabilize (test) (make-hash '((a . 20) (c . 1))) 'out)
;; (component-holes (test))

;; (component-constraints (test))
;; (hash-ref (component-holes (test)) 'a)
;; (stabilize (test) (make-hash '((a . 20) (c . 1))) 'a)

;; (component-constraints (test))
;; (get-neighs (test) 'out)
;; (component-holes (test))
;; (component-constraints (test))

;; (println "here")
;; (get-neighs (test) 'foo)
;; (stabilize (test) (make-hash '((a . 20) (c . 1))) 'out)

;; (map (lambda (x) (relevant-constraints (test) x)) '(a-in))
;; (require "constraint.rkt")

;; (component-constraints (test))

;; (define/module my-add ((a : 32)) ((out : 32))
;;   ([add = new comp/add]
;;    [a -> add @ left]
;;    [const 1 : 32 -> add @ right]
;;    [add @ out -> out]))
;; (get-neighs (my-add) 'add)

;; (hash-ref (component-holes (test)) 'a-in)


;; (require "constraint.rkt")
;; (map s-hole-pair
;;      (map (lambda (i) (hash-ref (component-holes (test)) i))
;;           (flatten (map get-dependencies (relevant-constraints (test) 'out)))))
;; (relevant-constraints (test) 'out)
;; (hash-ref (component-holes (test)) 'a)

;; Algo to stabilize node
;; 1) get neighbors
;;    a) look at connected neighbors
;;    b) look at all constraints to figure out which holes to look at
;; 2) stabilize all neighbors
;;    ...

;; (get-vertices (component-graph (test)))
;; (get-neighs (test) 'out)
;; (compute (test) (make-hash '((a . 10) (c . 1))))

;; (define/module test2 ((a : 32)) ((out : 32))
;;   ()
;;   )






