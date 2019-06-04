#lang racket
(require graph
         (for-syntax racket/base
                     syntax/parse))

(struct component (name                ;; name of the component
                   [ins #:mutable]     ;; list of input ports
                   [outs #:mutable]    ;; list of output ports
                   submods             ;; hashtbl of sub components keyed on their name
                   graph)              ;; graph representing internal connections
  #:transparent)

(define (empty-graph) (unweighted-graph/directed '()))

(define (input-component) (component 'input '() #f (make-hash) (empty-graph)))

(define (output-component) (component 'output #f '() (make-hash) (empty-graph)))

(define (default-component name ins outs)
  (let ([htbl (make-hash)])
    (for-each (lambda (n)
                (hash-set! htbl n (input-component)))
              ins)
    (for-each (lambda (n)
                (hash-set! htbl n (output-component)))
              outs)
    (component name ins outs htbl (empty-graph))))

(define-syntax (define/module stx)

  (define-syntax-class stmt
    #:description "connecting components and instantiating modules"
    #:literals (-> = new)
    #:attributes (fun)
    (pattern (u:id -> v:id)
             #:with fun #'(println '(u v)))
    (pattern (name:id = new mod:id)
             #:with fun #'(println '(name mod))))

  (syntax-parse stx
    [(_ name:id (i1:id ...) (o1:id ...) (stmt:stmt ...))
     #'(define (name)
         (let ([c (empty-graph)])
           (println c)
           (stmt.fun c) ...
           ))]
    )
  )

;; (require macro-debugger/stepper)
;; (expand/step #'(define/module myadd (a b) (x)
;;                  ([adder = new add]
;;                   [a -> adder]
;;                   [b -> adder]
;;                   [adder -> x])))

