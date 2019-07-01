#lang racket
(require "component.rkt"
         "port.rkt")
(provide comp/id
         comp/reg
         comp/add
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

(define-syntax-rule (filter-apply op item ...)
  (apply op (filter-map (lambda (x) x) (list item ...))))

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
                    [out => (filter-apply + left right)])))
(define (comp/sub)
  (default-component
    'sub
    input-list
    output-list
    (keyword-lambda (left right) ()
                    [out => (let ([x (filter-apply - left right)])
                              (if (< x 0)
                                  0
                                  x))])))
(define (comp/mult)
  (default-component 'mult input-list output-list))
(define (comp/div)
  (default-component 'div input-list output-list))
(define (comp/and)
  (default-component 'and input-list output-list))
(define (comp/or)
  (default-component 'or input-list output-list))
(define (comp/xor)
  (default-component 'xor input-list output-list))

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
