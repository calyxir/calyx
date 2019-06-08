#lang racket
(require graph
         "port.rkt"
         "component.rkt"
         "constraint.rkt"
         racket/format)
(require (for-syntax racket/base
                     syntax/parse))
(provide define/module)

(define-syntax-rule (connect u uf v vf)
  (lambda (c)
    (connect! c u uf v vf)))

(define-syntax-rule (create-module name mod)
  (lambda (c)
    (add-submod! c name mod)))

(define-syntax-rule (split port-name split-pt name1 name2)
  (lambda (c)
    (split! c port-name split-pt name1 name2)))

(define-syntax-rule (constant n width u uport)
  (lambda (c)
    (add-submod! c n (make-constant n width))
    (connect! c n 'inf# u uport)))

(define-syntax-rule (in-hole var-name u uport)
  (lambda(c)
    (add-in-hole! c var-name u uport)))
(define-syntax-rule (const-hole var-name n width)
  (lambda (c)
    (add-submod! c n (make-constant n width))
    (add-in-hole! c var-name n 'inf#)))
(define-syntax-rule (out-hole var-name u uport)
  (lambda(c)
    (add-out-hole! c var-name u uport)))

(define-syntax-rule (gen-proc name (in ...))
  (keyword-lambda (in ...)
    (let ([inputs (make-hash)])
      (hash-set! inputs 'in in) ...
      (compute (name) inputs))))

(define-syntax-rule (add-eq-constr left right)
  (lambda (c)
    (add-constraint! c (equal-constraint left right))))

(define-syntax-rule (add-when-constr left right con)
  (lambda (c)
    (add-constraint! c (cond-constraint left right con))))

(define-syntax-rule (add-unless-constr left right con)
  (lambda (c)
    (let ([ncon (if (= con 1) 0 1)])
      (add-constraint! c (cond-constraint left right ncon)))))

;; TODO: factor out the patterns properly
(define-syntax (define/module stx)
  (define-syntax-class stmt
    #:description "connecting components and instantiating modules"
    #:literals (-> = new const)
    #:datum-literals (@ split &)
    #:attributes (fun)
    ;; port patterns
    (pattern (u:id @ uport:id -> v:id @ vport:id)
             #:with fun #'(connect 'u 'uport 'v 'vport))
    (pattern (u:id -> v:id @ vport:id)
             #:with fun #'(connect 'u 'inf# 'v 'vport))
    (pattern (u:id @ uport:id -> v:id)
             #:with fun #'(connect 'u 'uport 'v 'inf#))
    (pattern (u:id -> v:id)
             #:with fun #'(connect 'u 'inf# 'v 'inf#))

    ;; const patterns
    (pattern (const n:nat : w:nat -> u:id @ uport:id)
             #:with fun #'(constant n w 'u 'uport))
    (pattern (const n:nat : w:nat -> u:id)
             #:with fun #'(constant n w 'u 'inf#))
    (pattern (const n:nat : w:nat -> hole var:id)
             #:with fun #'(const-hole var n w))

    ;; hole patterns
    (pattern (u:id @ uport:id -> hole var:id)
             #:with fun #'(in-hole 'var 'u 'uport))
    (pattern (u:id -> hole var:id)
             #:with fun #'(in-hole 'var 'u 'inf#))
    (pattern (hole var:id -> u:id @ uport:id)
             #:with fun #'(out-hole 'var 'u 'uport))
    (pattern (hole var:id -> u:id)
             #:with fun #'(out-hole 'var 'u 'inf#))

    ;; create module pattern
    (pattern (name:id = new mod:id)
             #:with fun #'(create-module 'name (mod)))

    ;; split port pattern
    (pattern (n1:id & n2:id = split pt:nat var:id)
             #:with fun #'(split 'var pt 'n1 'n2)))

  (define-syntax-class portdecl
    #:description "ports"
    #:datum-literals (:)
    (pattern (name:id : width:nat)))

  (define-syntax-class constraint
    #:description "the constraint language for futil"
    #:literals (when)
    (pattern (out:id = in:id (when con:id))
             #:with fun #'(add-when-constr 'out 'in 'con))
    ;; (pattern (out:id = in:id (unless con:id))
    ;;          #:with fun #'(add-unless-constr 'out 'in 'con))
    (pattern (out:id = in:id)
             #:with fun #'(add-eq-constr 'out 'in)))

  (syntax-parse stx
    [(_ name (i1:portdecl ...) (o1:portdecl ...) (stmt:stmt ...) constraint:constraint ...)
     #:fail-when (check-duplicate-identifier
                  (syntax->list #'(i1.name ... o1.name ...)))
     "duplicate variable name"

     #'(begin
         (define (name)
           (let ([c (default-component
                      'name
                      (list (port 'i1.name i1.width) ...)
                      (list (port 'o1.name o1.width) ...)
                      (gen-proc name (i1.name ...))
                      )])
             (stmt.fun c) ...
             (constraint.fun c) ...
             c))
         (name))]))
;; (syntax->datum
;;  (expand
;;   '(define/module test ((a : 32) (c : 1)) ((out : 32))
;;      ([a -> hole a]
;;       [c -> hole c]
;;       [hole out -> out]))
;;   )
;;  )


;; (syntax->datum
;;  (expand
;;   '(define/module add1 ((a : 32)) ((out : 32))
;;      ([add = new comp/add]
;;       [a -> add @ right]
;;       [const 1 : 32 -> add @ left]
;;       [add @ out -> out]))
;;  ))

;; (syntax->datum
;;  (expand
;;   '(define/module mux4 ((a : 32) (b : 32) (c : 32) (d : 32) (control : 2)) ((out : 32))
;;     [con1 & con2 = split 1 control]
;;     [dup = new dup]
;;     [con1 -> dup]
;;     [mux1 = new mux]
;;     [mux2 = new mux]
;;     [mux3 = new mux]
;;     [a -> mux1 @ left]
;;     [b -> mux1 @ right]
;;     [c -> mux2 @ left]
;;     [d -> mux2 @ right]
;;     [dup @ out1 -> mux1 @ control]
;;     [dup @ out2 -> mux2 @ control]
;;     [con2 -> mux3 @ control]
;;     [mux1 @ out -> mux3 @ left]
;;     [mux2 @ out -> mux3 @ right]
;;     [mux3 @ out -> out])
;;   )
;;  )
