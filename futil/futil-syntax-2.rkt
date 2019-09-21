#lang racket/base
(require graph
         "port.rkt"
         "component.rkt"
         "ast.rkt"
         "util.rkt"
         racket/dict
         racket/format)

(require (for-syntax racket/base
                     syntax/parse))

(provide design)

(define-syntax (design stx)
  (define-syntax-class io
    #:description "input and output ports"
    #:attributes (expr)
    (pattern (name:id width:exact-positive-integer)
             #:with expr #'(port 'name width)))

  ;; ~seq and splicing syntax lets us get away with fewer paranthesis in the syntax
  (define-splicing-syntax-class port
    #:description "sub component port"
    #:datum-literals (@)
    #:attributes (expr)
    (pattern (@ sub-comp port)
             #:with expr #'(sub-comp . port))
    (pattern (~seq io-port)
             #:with expr #'(io-port . inf#)))

  (define-splicing-syntax-class decl
    #:description "declare a sub component"
    #:datum-literals (new)
    #:attributes (expr)
    (pattern (~seq new var comp)
             #:with expr #'(decl var comp)))

  (define-splicing-syntax-class connection
    #:description "connect two ports of sub components"
    #:datum-literals (->)
    #:attributes (expr)
    (pattern (~seq -> src:port dest:port)
             #:with expr #'(connection 'src.expr 'dest.expr)))

  (define-syntax-class structure
    #:description "component structure"
    #:attributes (expr)
    (pattern (c:connection)
             #:with expr #'c.expr)
    (pattern (d:decl)
             #:with expr #'d.expr))

  (define-syntax-class control
    #:description "control expressions"
    #:literals (if)
    #:datum-literals (!! seq par ifen while)
    #:attributes (expr)

    ;; composition
    (pattern (seq con:control ...)
             #:with expr #'(seq-comp (list con.expr ...)))
    (pattern (par con:control ...)
             #:with expr #'(par-comp (list con.expr ...)))

    ;; control flow
    (pattern (if con:port tbranch:control fbranch:control)
             #:with expr #'(if-stmt 'con.expr tbranch.expr fbranch.expr))
    (pattern (ifen con:port tbranch:control fbranch:control)
             #:with expr #'(ifen-stmt 'con.expr tbranch.expr fbranch.expr))
    (pattern (while con:port body:control)
             #:with expr #'(while-stmt 'con.expr body.expr))

    ;; activation statements
    (pattern (!! x ...)
             #:with expr #'(act-stmt (list 'x ...)))
    (pattern (x ...)
             #:with expr #'(deact-stmt (list 'x ...))))

  (define-syntax-class component
    #:description "component structure and control definitions"
    #:datum-literals (define/component)
    ;; #:attributes (expr)
    (pattern (define/component name:id (inputs:io ...) (outputs:io ...)
               (structure:structure ...)
               control:control ...)
             #:with expr #'(make-component 'name
                                           (list inputs.expr ...)
                                           (list outputs.expr ...)
                                           (list structure.expr ...)
                                           (list control.expr ...)
                                           )))

  (syntax-parse stx
    [(_ c:component ...)
     #'(begin
         (define c.name c.expr)
         ...)])
  )

;; (require macro-debugger/stepper)
;; (expand/step
;;  #'(design
;;     (define/component test ((a 32)) ((out 32))
;;       ([new test comp/add]
;;        [-> (@ test left) out])
;;       (seq [!! a b c d]
;;            [a b c d]
;;            [!! hi there]
;;            (if (@ a out)
;;                (seq [!! a]
;;                     [!! b])
;;                (seq [!! b]
;;                     [!! a])
;;                )
;;            )
;;       )
;;     )
;;  )
;; syntax that generates the correct computation function
;; that is used for a modules procedure
;; (define-syntax-rule (gen-proc name (in ...) (out ...))
;;   (keyword-lambda (sub-mem# in ...)
;;                   ([res = (let* ([inputs (list (cons 'in in) ...)]
;;                                  [tup (compute (name) inputs
;;                                                #:memory sub-mem#)])
;;                             (cons
;;                              (ast-tuple-state tup)
;;                              (ast-tuple-memory tup)))])
;;                   [sub-mem# => (cdr res)]
;;                   [out => (dict-ref (car res) '(out . inf#))] ...))

