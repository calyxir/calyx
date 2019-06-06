#lang racket
(require "component.rkt")
(provide comp/add
         comp/sub
         comp/mult
         comp/div
         comp/and
         comp/or
         comp/xor)

(define input-list
  (list (port 'left 32)
        (port 'right 32)))
(define output-list
  (list (port 'out 32)))

(define (comp/add)
  (default-component
    'add
    input-list
    output-list
    (keyword-lambda (left right) (+ left right))
    #t))
(define (comp/sub)
  (default-component 'sub input-list output-list))
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
