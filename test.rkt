#lang racket
(require "component.rkt"
         "futil.rkt"
         "futil-prims.rkt")

;; (define/module myadd ((lft : 32) (rgt : 32)) ((out : 32))
;;   [adder = new comp/add]
;;   [lft -> adder @ left]
;;   [rgt -> adder @ right]
;;   [adder @ out -> out])
;; (plot (myadd))

;; (define/module add4 ((a : 32) (b : 32) (c : 32) (d : 32)) ((out : 32))
;;   [add1 = new comp/add]
;;   [add2 = new comp/add]
;;   [add3 = new comp/add]
;;   [a -> add1 @ left]
;;   [b -> add1 @ right]
;;   [c -> add2 @ left]
;;   [d -> add2 @ right]
;;   [add1 @ out -> add3 @ left]
;;   [add2 @ out -> add3 @ right]
;;   [add3 @ out -> out])
;; (plot (add4))

;; (define/module add4v2 ((a : 32) (b : 32) (c : 32) (d : 32)) ((out : 32))
;;   [add1 = new comp/add]
;;   [add2 = new comp/add]
;;   [add3 = new comp/add]
;;   [a -> add1 @ left]
;;   [b -> add1 @ right]
;;   [add1 @ out -> add2 @ left]
;;   [c -> add2 @ right]
;;   [add2 @ out -> add3 @ left]
;;   [d -> add3 @ right]
;;   [add3 @ out -> out])
;; (plot (add4v2))

;; (define/module mux ((left : 32) (right : 32) (control : 1)) ((out : 32))
;;   [left -> out]
;;   [right -> out])

;; (define/module dup32 ((in : 32)) ((out1 : 32) (out2 : 32))
;;   [in -> out1]
;;   [in -> out2])

;; (define/module dup2 ((in : 2)) ((out1 : 2) (out2 : 2))
;;   [in -> out1]
;;   [in -> out2])

;; (define/module dup1 ((in : 1)) ((out1 : 1) (out2 : 1))
;;   [in -> out1]
;;   [in -> out2])

;; (define/module mux4 ((a : 32) (b : 32) (c : 32) (d : 32) (control : 2)) ((out : 32))
;;   [con1 & con2 = split 1 control]
;;   [dup = new dup1]
;;   [con1 -> dup @ in]
;;   [mux1 = new mux]
;;   [mux2 = new mux]
;;   [mux3 = new mux]
;;   [a -> mux1 @ left]
;;   [b -> mux1 @ right]
;;   [c -> mux2 @ left]
;;   [d -> mux2 @ right]
;;   [dup @ out1 -> mux1 @ control]
;;   [dup @ out2 -> mux2 @ control]
;;   [con2 -> mux3 @ control]
;;   [mux1 @ out -> mux3 @ left]
;;   [mux2 @ out -> mux3 @ right]
;;   [mux3 @ out -> out])
;; (plot (mux4))

;; (define/module smallAdd4 ((a : 32) (b : 32) (c : 32) (d : 32)) ((out : 32))
;;   [add = new comp/add]
;;   [mux = new mux4]
;;   [con = new const2]
;;   [dup = new dup32]
;;   [a -> mux @ a]
;;   [b -> mux @ b]
;;   [c -> mux @ c]
;;   [d -> mux @ d]
;;   [const -> mux @ control]
;;   [dup @ out1 -> add @ left]
;;   [mux @ out -> add @ right]
;;   [add @ out -> dup @ in]
;;   [dup @ out2 -> out])
;; (plot (smallAdd4))

;; (define/module splitter32 ((in : 32)) ((out-l : 20) (out-r : 12))
;;   [in-l & in-r = split 20 in]
;;   [in-l -> out-l]
;;   [in-r -> out-r])
;; (plot (splitter32))

;; (define/module joiner32 ((in-l : 16) (in-r : 16)) ((out : 32))
;;   [out-l & out-r = split 16 out]
;;   [in-l -> out-l]
;;   [in-r -> out-r])

;; (define/module plus1_2-32 ((a : 32) (control : 1)) ((out : 32))
;;   [add = new comp/add]
;;   [a -> add @ left]
;;   [mux = new mux]
;;   [const 1 : 32 -> mux @ right]
;;   [const 2 : 32 -> mux @ left]
;;   [control -> mux @ control]
;;   [mux @ out -> add @ right]
;;   [add @ out -> out])

;; (define/module add1 ((a : 32)) ((out : 32))
;;   ([add = new comp/add]
;;    [a -> add @ right]
;;    [const 1 : 32 -> add @ left]
;;    [add @ out -> out]))
;; (define/module add2 ((a : 32)) ((out : 32))
;;   ([add = new comp/add]
;;    [a -> add @ right]
;;    [const 2 : 32 -> add @ left]
;;    [add @ out -> out]))
;; (define/module prog ((a : 32)) ((out : 32))
;;   ([add1 = new add1]
;;    [a -> add1 @ a]
;;    [h = hole]
;;    [add1 @ out -> h out]
;;    [add2 = new add2]
;;    [h in -> add2 @ a]
;;    [add2 @ out -> out])
;;   ([in = out]))

  ; or
  ;; [in = 5]

(define/module add1 ((a : 32)) ((out : 32))
  ([add = new comp/add]
   [a -> add @ right]
   [const 1 : 32 -> add @ left]
   [add @ out -> out]))

(define/module add2 ((a : 32)) ((out : 32))
  ([add = new comp/add]
   [a -> add @ right]
   [const 2 : 32 -> add @ left]
   [add @ out -> out]))

;; (make-hash (map (lambda (x y) `(,x . ,y)) (map port-name (component-ins (comp/add))) '(1 20)))
;; (compute (add1) (make-hash '((a . 20))))
;; (require graph)
;; (stabilize (add1) (make-hash '((a . 20))) 'add)

;; (compute (comp/add) (make-hash '((left . 20) (right . 40))))

(require graph)
(define inputs (make-hash '((a . 20))))
(compute (add2) (make-hash '((a . 20))))

;; (map (lambda (pair)
;;        (println pair)
;;        (match pair
;;          [(cons name (cons (cons v _) _))
;;           (begin
;;             (println (~v name v))
;;             `(,name . ,(stabilize (add2) inputs v))
;;             )
;;           ]))
;;      (get-neighbors (add2) 'add))

;; (map port-name (component-ins (get-submod! (add2) 'add)))
;; (backtrack (add2) '((out . inf#)))
;; (sequence->list (in-neighbors
;;                  (transpose (component-graph (add2)))
;;                  '(add . left)))
;; (backtrack (add2) '((2 . inf#)))
;; (backtrack (add2) '((add . out)))

;; (map (lambda (k) `(add . ,k)) '(left right))

;; (define inputs (make-hash '((a . 20))))
;; (member 'out (map port-name (component-ins (add2))))
;; (sequence->list (in-neighbors (transpose (component-graph (add2))) 'out))

(plot (add2))

(define (mux)
  (default-component
    'mux
    (list (port 'left 32)
          (port 'right 32)
          (port 'control 1))
    (list (port 'out 32))
    (keyword-lambda (left right control)
                    (if (= 1 control)
                        left
                        right))
    #t))
;; (plot (mux))

(define/module test () ((out : 32))
  ([mux = new mux]
   [const 20 : 32 -> mux @ left]
   [const 0 : 1 -> mux @ control]
   [const 40 : 32 -> mux @ right]
   [mux @ out -> out]))
(compute (test) (make-hash))

;; (compute (add1) (make-hash '((a . 4))))

;; (define/module test ((a : 32)) ((out : 32))
;;   ([add1 = new add1]
;;    [a -> add1 @ a]
;;    [add1 @ out -> out]))
;; (compute (test) (make-hash '((a . 10))))

;; (compute (mux) (make-hash '((left . 20) (right . 43) (control . 1))))

(define/module prog ((a : 32) (c : 1)) ((out : 32))
  ([add1 = new add1]
   [add2 = new add2]
   [mux = new mux]
   [c -> mux @ control];; [global c]
   ;; [h = hole]
   [a -> add1 @ a]
   [a -> add2 @ a]
   [add1 @ out -> mux @ right] ;; [add1 @ out -> hole one]
   [add2 @ out -> mux @ left]  ;; [add2 @ out -> hole two]
   [mux @ out -> out];; [hole out -> out]
   )
  ;; ([if c
  ;;      out = one
  ;;      out = two])
  )

;; (compute (prog) (make-hash '((a . 6) (c . 1))))

;; (syntax->datum (expand '(keyword-lambda (x y z) (+ x y z))))
  ; or
  ;; [out = one]
  ; or
  ;; [out = two]

;; (compute (simp) (make-hash))

;; (define (stabilize inputs comp vertex)
;;   (if (member vertex (map port-name (component-ins comp)))
;;       (hash-ref inputs vertex)
;;       (begin
;;         (let* ([lst (sequence->list (in-neighbors
;;                                      (transpose (component-graph comp))
;;                                      vertex))]
;;                [vals (map (lambda (v) (stabilize inputs comp v))
;;                           lst)]
;;                [non-void-vals (filter (lambda (x) (not (void? x))) vals)]
;;                [proc (component-proc (get-submod! comp vertex))])
;;           ;; (print "lst ") (println lst)
;;           ;; (print "nv-vals ") (println non-void-vals)
;;           (apply proc non-void-vals))
;;         )))

;; (define inputs (make-hash))
;; (hash-set! inputs 'x 2)

;; (stabilize inputs (id) 'out)
;; (stabilize (make-hash) (simp) 'out)

;; (plot (simp))
;; (compute (simp) '())

;; (plot (plus1_2-32))

;; (plot (joiner32))

;; ----------------------------------------------

;; (input a b c)
;; (seq
;;  (x = add a b)
;;  --- (x)
;;  (y = add x c))
;; (output y)

;; (mux)
;; (input a b condi)
;; (x = if condi a b)
;; (output x)

;; (input a cond)
;; (if cond
;;     (x = add a 1)
;;     (x = add a 2))
;; (output x)

;; FRP, looks like dynamism is what we want. the (define/module) macro gives us
;; a way to build up static dependency graphs. Now we need a way to take a static dependency
;; graph and overlay `dynamism`

;; (seq cmd1 ...)
;; (par cmd1 ...)

;; (if cond mod1 mod2)
;; (loop )



;; could be explicit about holding values in registers
;; could be implicit: 










