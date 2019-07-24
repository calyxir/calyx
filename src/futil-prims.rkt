#lang racket/base

(require "component.rkt"
         "port.rkt")

(provide comp/id
         comp/reg
         comp/add
         comp/trunc-sub
         comp/sub
         comp/mult
         comp/div
         comp/and
         comp/or
         comp/xor
         magic/mux)

(define input-list
  (list (port 'left 32)
        (port 'right 32)))
(define output-list
  (list (port 'out 32)))

(define-syntax-rule (falsify-apply op item ...)
  (if (andmap (lambda (x) x) (list item ...))
      (apply op (list item ...))
      #f))

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
    (keyword-lambda (in) ()
                    [out => in])
    #:mode #t))
(define (comp/add)
  (default-component
    'add
    input-list
    output-list
    (keyword-lambda (left right) ()
                    [out => (falsify-apply + left right)])))
(define (comp/trunc-sub)
  (default-component
    'sub
    input-list
    output-list
    (keyword-lambda (left right) ()
                    [out => (let ([x (falsify-apply - left right)])
                              (cond [(not x) #f]
                                    [(< x 0) 0]
                                    [else x]))])))

(define (comp/sub)
  (default-component
    'sub
    input-list
    output-list
    (keyword-lambda (left right) ()
                    [out => (falsify-apply - left right)])))
(define (comp/mult)
  (default-component
    'mult
    input-list
    output-list
    (keyword-lambda (left right) ()
                    [out => (falsify-apply * left right)])))
(define (comp/div)
  (default-component
    'div
    input-list
    output-list
    (keyword-lambda (left right) ()
                    [out => (falsify-apply / left right)])))
(define (comp/and)
  (default-component
    'and
    input-list
    output-list
    (keyword-lambda (left right) ()
                    [out => (falsify-apply bitwise-and left right)])))
(define (comp/or)
  (default-component
    'or
    input-list
    output-list
    (keyword-lambda (left right) ()
                    [out => (falsify-apply bitwise-ior left right)])))
(define (comp/xor)
  (default-component
    'xor
    input-list
    output-list
    (keyword-lambda (left right) ()
                    [out => (falsify-apply bitwise-xor left right)])))

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
