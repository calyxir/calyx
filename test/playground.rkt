#lang racket/base

(require futil)

(show-debug
 (compute
  (comp/iterator)
  '((start . 0) (end . 10) (incr . 1) (en . 1))
  ))
