#lang racket/base
(require graph
         "port.rkt"
         "component.rkt"
         "ast.rkt"
         "util.rkt"
         racket/format)

(require (for-syntax racket/base
                     syntax/parse))

(provide define/module)

;; simple macro that allows you to pass in components instead of
;; functions for components in some places
(define-syntax-rule (call fun)
  (if (procedure? fun)
      (fun)
      fun))

;; syntax for function that takes in a component and connects (u . uf) to (v . vf)
(define-syntax-rule (connect u uf v vf)
  (lambda (c)
    (connect! c u uf v vf)))

;; syntax for function that takes a component and adds the submodule
(define-syntax-rule (create-module name mod)
  (lambda (c)
    (add-submod! c name mod)))

;; syntax for splitting a port
(define-syntax-rule (split port-name split-pt name1 name2)
  (lambda (c)
    (split! c port-name split-pt name1 name2)))

;; syntax for adding a constant
(define-syntax-rule (constant name n width u uport)
  (lambda (c)
    (add-submod! c name (make-constant n width))
    (connect! c name 'inf# u uport)))

;; syntax that generates the correct computation function
;; that is used for a modules procedure
(define-syntax-rule (gen-proc name (in ...) (out ...))
  (keyword-lambda (sub-mem# in ...)
                  ([res = (let* ([inputs (list (cons 'in in) ...)]
                                 [tup (compute (call name) inputs #:memory sub-mem#)])
                            (cons
                             (ast-tuple-state tup)
                             (ast-tuple-memory tup)))])
                  [sub-mem# => (cdr res)]
                  [out => (hash-ref (car res) '(out . inf#))] ...))

;; TODO: factor out the patterns properly
(define-syntax (define/module stx)
  (define-splicing-syntax-class stmt
    #:description "connecting components and instantiating modules"
    #:datum-literals (@ split & control new -> = const)
    #:attributes (fun)
    ;; port patterns
    (pattern (u:wire-port -> v:wire-port)
             #:with fun #'(connect 'u.name 'u.port 'v.name 'v.port))

    ;; const patterns
    (pattern (const str:id n:nat : w:nat -> u:wire-port)
             #:with fun #'(constant 'str n w 'u.name 'u.port))

    ;; create module pattern
    (pattern (name:id = new mod:id)
             #:with fun #'(create-module 'name (call mod)))

    ;; split port pattern
    (pattern (n1:id & n2:id = split pt:nat var:id)
             #:with fun #'(split 'var pt 'n1 'n2)))

  (define-splicing-syntax-class wire-port
    #:description "syntax for specifying the port of a submodule"
    #:datum-literals (@)
    (pattern (~seq name:id
                   (~optional (~seq @ port:id)
                              #:defaults ([port #'inf#])))))

  (define-syntax-class portdecl
    #:description "port declaration in module signature defintion"
    #:datum-literals (:)
    (pattern (name:id : width:nat)))

  (define-syntax-class constr-expr
    #:description "possible constraint expressions"
    #:literals (if)
    #:datum-literals (while ifen)

    (pattern (if (comp:id port) [tbranch:constraint ...] [fbranch:constraint ...])
             #:with val #'(if-stmt '(comp . port)
                                   (seq-comp (list tbranch.item ...))
                                   (seq-comp (list fbranch.item ...))))
    (pattern (ifen (comp:id port) [tbranch:constraint ...] [fbranch:constraint ...])
             #:with val #'(ifen-stmt '(comp . port)
                                     (seq-comp (list tbranch.item ...))
                                     (seq-comp (list fbranch.item ...))))
    (pattern (while (comp:id port) [body:constraint ...])
             #:with val #'(while-stmt '(comp . port)
                                      (seq-comp (list body.item ...))))
    (pattern (x ...)
             #:with val #'(deact-stmt (list 'x ...))))

  (define-syntax-class constraint
    #:description "the constraint language for futil"

    (pattern (x:constr-expr ...+)
             #:with item #'(par-comp (list x.val ...)))

    (pattern ()
             #:with item #'(deact-stmt (list))))

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
                      (gen-proc name (i1.name ...) (o1.name ...))
                      #:control (seq-comp (list constraint.item ...)))])
             (stmt.fun c) ...
             c))
         (name))]))

;; (require macro-debugger/stepper)
;; (expand/step
;;  #'(define/module add1 ((a : 32)) ((out : 32))
;;    ([add = new comp/add]
;;     [a -> add @ right]
;;     [const 1 : 32 -> add @ left]
;;     [add @ out -> out]))
;;  )
;; (syntax->datum
;;  (expand
  ;; '(define/module add1 ((a : 32)) ((out : 32))
  ;;    ([add = new comp/add]
  ;;     [a -> add @ right]
  ;;     [const 1 : 32 -> add @ left]
  ;;     [add @ out -> out]))
;;  ))
