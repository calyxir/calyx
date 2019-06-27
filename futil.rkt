#lang racket
(require graph
         "port.rkt"
         "component.rkt"
         racket/format)
(require (for-syntax racket/base
                     syntax/parse))
(provide define/module
         make-constraint)

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

(define-syntax-rule (control-point name lst)
  (lambda (c)
    (add-control! c 'name lst)))

;; (define-syntax-rule (in-hole var-name u uport)
;;   (lambda(c)
;;     (add-in-hole! c var-name u uport)))
;; (define-syntax-rule (const-hole var-name n width)
;;   (lambda (c)
;;     (add-submod! c n (make-constant n width))
;;     (add-in-hole! c var-name n 'inf#)))
;; (define-syntax-rule (out-hole var-name u uport)
;;   (lambda(c)
;;     (add-out-hole! c var-name u uport)))

(define-syntax-rule (gen-proc name (in ...) (out ...))
  (keyword-lambda (in ...)
                  ([res = (let ([inputs (list (cons 'in in) ...)])
                            (match (compute (name) inputs)
                              [(cons state vals)
                               (make-hash (apply append vals))]))])
                  [out => (hash-ref res 'out)] ...))

(define-syntax-rule (make-constraint comp port tru fals)
  (constr '(comp . port) 'tru 'fals))

(define-syntax-rule (construct-control (var ...))
  (begin
    ;; (for-each (lambda (x) (println (~v 'cc (eval x) (constr? (eval x))))) (list var ...))
    (let ([lst (map (lambda (x)
                      (if (list? x)
                          (eval x)
                          x))
                    (list var ...))])
      (control-pair (filter (lambda (x) (not (constr? x))) lst)
                    (filter (lambda (x) (constr? x)) lst)))))

;; (expand/step #'(gen-proc test (a b c) (out1 out2)))

;; (define-syntax-rule (add-eq-constr left right)
;;   (lambda (c)
;;     (add-constraint! c (equal-constraint left right))))

;; (define-syntax-rule (add-when-constr left right con)
;;   (lambda (c)
;;     (add-constraint! c (cond-constraint left right con))))

;; (define-syntax-rule (add-unless-constr left right con)
;;   (lambda (c)
;;     (let ([ncon (if (= con 1) 0 1)])
;;       (add-constraint! c (cond-constraint left right ncon)))))

;; TODO: factor out the patterns properly
(define-syntax (define/module stx)
  (define-syntax-class stmt
    #:description "connecting components and instantiating modules"
    #:literals (-> = new const)
    #:datum-literals (@ split & control)
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
    ;; (pattern (const n:nat : w:nat -> hole var:id)
    ;;          #:with fun #'(const-hole var n w))

    ;; hole patterns
    ;; (pattern (u:id @ uport:id -> hole var:id)
    ;;          #:with fun #'(in-hole 'var 'u 'uport))
    ;; (pattern (u:id -> hole var:id)
    ;;          #:with fun #'(in-hole 'var 'u 'inf#))
    ;; (pattern (hole var:id -> u:id @ uport:id)
    ;;          #:with fun #'(out-hole 'var 'u 'uport))
    ;; (pattern (hole var:id -> u:id)
    ;;          #:with fun #'(out-hole 'var 'u 'inf#))

    ;; control point patterns
    (pattern (control name:id = x:id ...)
             #:with fun #'(control-point name '(x ...)))

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

  (define-syntax-class constr-expr
    #:description "possible constraint expressions"
    #:literals (if)

    (pattern (if (comp:id port) tru:id fals:id)
             #:with val #'(make-constraint comp port tru fals))
    (pattern (x:id)
             #:with val #'x))

  (define-syntax-class constraint
    #:description "the constraint language for futil"
    #:literals (when)

    (pattern (x:constr-expr ...)
             #:with item #'(construct-control ('x.val ...)))

    ;; (pattern (out:id = in:id (when con:id))
    ;;          #:with fun #'(void))
    ;; (pattern (out:id = in:id (unless con:id))
    ;;          #:with fun #'(add-unless-constr 'out 'in 'con))
    ;; (pattern (out:id = in:id)
    ;;          #:with fun #'(void))
    )

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
                      (transform-control (list constraint.item ...)))])
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
