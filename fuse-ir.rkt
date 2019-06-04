#lang racket
(require graph
         "component.rkt")
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

(define-syntax (define/module stx)

  (define-syntax-class stmt
    #:description "connecting components and instantiating modules"
    #:literals (-> = new)
    #:datum-literals (@ split &)
    #:attributes (fun)
    (pattern (u:id @ uport:id -> v:id @ vport:id)
             #:with fun #'(connect 'u 'uport 'v 'vport))
    (pattern (u:id -> v:id @ vport:id)
             #:with fun #'(connect 'u 'inf# 'v 'vport))
    (pattern (u:id @ uport:id -> v:id)
             #:with fun #'(connect 'u 'uport 'v 'inf#))
    (pattern (u:id -> v:id)
             #:with fun #'(connect 'u 'inf# 'v 'inf#))
    (pattern (name:id = new mod:id)
             #:with fun #'(create-module 'name (mod)))
    (pattern (n1:id & n2:id = split pt:nat var:id)
             #:with fun #'(split 'var pt 'n1 'n2)))

  (define-syntax-class portdecl
    #:description "ports"
    #:datum-literals (:)
    (pattern (name:id : width:nat)))

  (syntax-parse stx
    [(_ name (i1:portdecl ...) (o1:portdecl ...) stmt:stmt ...)
     #:fail-when (check-duplicate-identifier
                  (syntax->list #'(i1.name ... o1.name ...)))
     "duplicate variable name"

     #'(define (name)
         (let ([c (default-component
                    'name
                    (list (port 'i1.name i1.width) ...)
                    (list (port 'o1.name o1.width) ...))])
           (stmt.fun c) ...
           c))]))

;; (define (repeat f n x)
;;   (if (= n 0)
;;       x
;;       (repeat f (- n 1) (f x))))
;; (require macro-debugger/stepper)
;; (syntax->datum
;;  (repeat expand-once 10 '(define/module splitter32 ((in : 32)) ((out-l : 16) (out-r : 16))
;;                  [in-l & in-r = split 16 in]
;;                  [in-l -> out-l]
;;                  [in-r -> out-r])))

;; (syntax->datum (expand #'(define/module myadd (l r) (o)
;;                            ([adder = new add]
;;                             [l -> adder @ left]
;;                             [r -> adder @ right]
;;                             [adder @ out -> o]))))
