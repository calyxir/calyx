#lang racket/base
(require graph
         "port.rkt"
         "component.rkt"
         "interpret.rkt"
         "ast.rkt"
         "util.rkt"
         "state-dict.rkt"
         threading
         racket/list
         racket/match
         racket/dict
         racket/format
         racket/function)

(require (for-syntax racket/base
                     syntax/parse))

(provide define/component)
(provide define/namespace)

(define (make-component name
                        inputs
                        outputs
                        structure
                        control)
  (define subcomps
    (~>
     (filter-map (match-lambda
                   [(connection _ _) #f]
                   [(decl var comp) `(,var . ,comp)])
                 structure)
     (append _
             (map (match-lambda
                    [(port name width) `(,name . ,(input-component width))])
                  inputs)
             (map (match-lambda
                    [(port name width) `(,name . ,(output-component width))])
                  outputs))
     make-immutable-hash))

  (println subcomps)

    ;; construct the graph
  (define graph (empty-graph))
  ;; add vertices for all the inputs and outputs
  (for-each (match-lambda
              [(port name _)
               (add-vertex! graph `(,name . inf#))])
            (append inputs outputs))
  ;; add the edges
  (for-each (match-lambda
              [(connection src dest)
               (add-directed-edge! graph src dest)]
              [(decl _ _) (void)])
            structure)

  (define comp
    (component name
               inputs
               outputs
               subcomps
               (make-immutable-hash) ; splits
               control
               (void)
               (lambda (old st) #f)
               graph
               (transpose graph)))

  ;; define process in terms of comp
  (define (proc hsh)
    ;; lookup all the input values from the hash
    (define input-vals
      (map (lambda (x)
             `(,x . ,(dict-ref hsh x)))
           (map port-name inputs)))
    ;; get sub-memory from the hash
    (define sub-mem# (dict-ref hsh 'sub-mem#))
    ;; compute the result
    (match-define (ast-tuple _ _ state memory)
      (compute comp input-vals #:memory sub-mem#))
    ;; construct the output hash
    (state-dict
     (cons
      `(sub-mem# . ,memory)
      (map (lambda (x)
             `(,x . ,(dict-ref state `(,x . inf#))))
           (map port-name outputs)))))

  ;; backpatch the proc
  (set-component-proc! comp proc)
  comp)

(define-syntax (define/component stx)
  (define-syntax-class io
    #:description "input and output ports"
    #:attributes (expr)
    (pattern (name:id width:exact-positive-integer)
             #:with expr #'(port 'name width)))

  (define-syntax-class port
    #:description "sub component port"
    #:datum-literals (@ this)
    #:attributes (expr)
    (pattern (@ this port)
             #:with expr #'(port . inf#))
    (pattern (@ sub-comp port)
             #:with expr #'(sub-comp . port)))

  ;; ~seq and splicing syntax lets us get away with fewer paranthesis in the syntax
  (define-splicing-syntax-class decl
    #:description "declare a sub component"
    #:datum-literals (new)
    #:attributes (expr)
    (pattern (~seq new var comp)
             #:with expr #'(decl 'var comp)))

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
    #:datum-literals (enable disable seq par ifen while empty)
    #:attributes (expr)

    ;; composition
    (pattern (seq con:control ...+)
             #:with expr #'(seq-comp (list con.expr ...)))
    (pattern (par con:control ...+)
             #:with expr #'(par-comp (list con.expr ...)))

    ;; control flow
    (pattern (if con:port tbranch:control fbranch:control)
             #:with expr #'(if-stmt 'con.expr tbranch.expr fbranch.expr))
    (pattern (ifen con:port tbranch:control fbranch:control)
             #:with expr #'(ifen-stmt 'con.expr tbranch.expr fbranch.expr))
    (pattern (while con:port body:control)
             #:with expr #'(while-stmt 'con.expr body.expr))

    ;; empty control
    (pattern (empty)
             #:with expr #'(seq-comp (list)))

    ;; activation statements
    (pattern (enable x ...+)
             #:with expr #'(act-stmt (list 'x ...)))
    (pattern (disable x ...+)
             #:with expr #'(deact-stmt (list 'x ...))))

  ;; (define-splicing-syntax-class component
  ;;   #:description "component structure and control definitions"
  ;;   #:datum-literals (define/component)
  ;;   (pattern (define/component name:id (inputs:io ...) (outputs:io ...)
  ;;              (structure:structure ...)
  ;;              (~optional (~seq control:control)
  ;;                         #:defaults ([control.expr #'(seq-comp (list))])))
  ;;            #:with expr ))

  (syntax-parse stx
    [(_ name:id (inputs:io ...) (outputs:io ...)
        (structure:structure ...)
        (~optional (~seq control:control)
                   #:defaults ([control.expr #'(seq-comp (list))])))
     #'(define name
         (make-component 'name
                         (list inputs.expr ...)
                         (list outputs.expr ...)
                         (list structure.expr ...)
                         control.expr))]))


(define-syntax (define/namespace stx)
  (define-splicing-syntax-class namespace-stmt
    #:description "namepsace statements"
    #:datum-literals (import)
    #:attributes (expr)
    (pattern (import name:id)
             #:with expr #'(require 'name))
    (pattern (~seq stmt)
             #:with expr #'stmt))

  (syntax-parse stx
    [(_ name:id ns:namespace-stmt ...)
     #'(begin
         ns.expr ...)]))




