#lang racket/base

(require "component.rkt"
         "port.rkt"
         "util.rkt")

(provide (all-defined-out))

(define input-list
  (list (port 'left 32)
        (port 'right 32)))
(define output-list
  (list (port 'out 32)))

(define-syntax-rule (falsify-apply op item ...)
  (if (andmap (lambda (x) x) (list item ...))
      (apply op (list item ...))
      #f))

(define (simple-binop name op)
  (default-component
    name
    input-list
    output-list
    (keyword-lambda (left right) ()
                    [out => (falsify-apply op left right)])))

(define (comp/id)
  (default-component
    'id
    (list (port 'in 32))
    (list (port 'out 32))
    (keyword-lambda (in) ()
                    [out => in])))

(define (comp/reg)
  (default-component
    'reg
    (list (port 'in 32))
    (list (port 'out 32))
    (keyword-lambda (mem-val# in) ()
                    [out => (if in in mem-val#)])
    #:memory-proc (lambda (old st)
                    (define new-v (hash-ref st 'in))
                    (if new-v new-v old))))

(define (comp/memory-8bit)
  (default-component
    'mem-8bit
    (list (port 'addr 32)     ; XXX should be 8 bits
          (port 'data-in 32))
    (list (port 'out 32))
    (keyword-lambda (mem-val# addr data-in)
                    ([mem = (if (hash? mem-val#) mem-val# (make-immutable-hash))])
                    [out => (if data-in
                                data-in
                                (hash-ref mem addr
                                          (lambda () 0)))])
    #:memory-proc (lambda (old st)
                    (if (hash? old)
                        (if (hash-ref st 'data-in)
                            (hash-set
                             old
                             (hash-ref st 'addr) (hash-ref st 'data-in))
                            old)
                        (make-immutable-hash)))))

(define (comp/trunc-sub)
  (default-component
    'trunc-sub
    input-list
    output-list
    (keyword-lambda (left right) ()
                    [out => (let ([x (falsify-apply - left right)])
                              (cond [(not x) #f]
                                    [(< x 0) 0]
                                    [else x]))])))

(define (comp/add) (simple-binop 'add +))
(define (comp/sub) (simple-binop 'sub -))
(define (comp/mult) (simple-binop 'mult *))
(define (comp/div) (simple-binop 'div /))
(define (comp/and) (simple-binop 'and bitwise-and))
(define (comp/or) (simple-binop 'or bitwise-ior))
(define (comp/xor) (simple-binop 'xor bitwise-xor))

(define (magic/mux)
  (default-component
    'mux
    (list (port 'left 32)
          (port 'right 32)
          (port 'control 1))
    (list (port 'out 32))
    (keyword-lambda (left right control) ()
                    [out => (if (= 1 control)
                                left
                                right)])))
