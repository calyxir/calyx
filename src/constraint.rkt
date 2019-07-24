#lang racket/base

(provide (struct-out equal-constraint)
         (struct-out cond-constraint)
         (struct-out equal-computation)
         (struct-out cond-computation)
         get-left
         get-dependencies)

;; equality constraint. expresses that anywhere that the variable
;; left appears, you can replace that with right.
(struct equal-constraint (left right)
  #:transparent)

;; conditional constraint. expresses that if the variable condition
;; is equal to 1, then you can replace instances of left with right.
(struct cond-constraint (left right condition)
  #:transparent)

(struct equal-computation (val)
  #:transparent)

(struct cond-computation (val condition)
  #:transparent)

(define (get-left con)
  (match con
    [(equal-constraint left _) left]
    [(cond-constraint left _ _) left]
    [_ (error "Not a constraint")]))

(define (get-dependencies con)
  (match con
    [(equal-constraint _ right) (equal-computation right)]
    [(cond-constraint _ right condition) (cond-computation right condition)]
    [_ (error "Not a constraint")]))
