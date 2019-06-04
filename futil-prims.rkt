#lang racket
(require "component.rkt")
(provide comp/add
         comp/sub
         comp/mult
         comp/div
         comp/and
         comp/or
         comp/xor)

(define (comp/add)
  (default-component 'add (left right) '(out)))
(define (comp/sub)
  (default-component 'sub '(left right) '(out)))
(define (comp/mult)
  (default-component 'mult '(left right) '(out)))
(define (comp/div)
  (default-component 'div '(left right) '(out)))
(define (comp/and)
  (default-component 'and '(left right) '(out)))
(define (comp/or)
  (default-component 'or '(left right) '(out)))
(define (comp/xor)
  (default-component 'xor '(left right) '(out)))
