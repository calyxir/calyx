#lang racket/base

(require racket/contract
         racket/list)

(provide (struct-out par-comp)
         (struct-out seq-comp)
         (struct-out deact-stmt)
         (struct-out act-stmt)
         (struct-out if-stmt)
         (struct-out ifen-stmt)
         (struct-out while-stmt)
         (struct-out mem-print))

;; type of statements
(define-struct/contract par-comp
  ([stmts (and/c list? (not/c empty?))])
  #:transparent)

(define-struct/contract seq-comp
  ([stmts list?])
  #:transparent)

(define-struct/contract deact-stmt
  ([mods (listof symbol?)])
  #:transparent)

(define-struct/contract act-stmt
  ([mods (listof symbol?)])
  #:transparent)

(define-struct/contract if-stmt
  ([condition pair?]
   [tbranch any/c]
   [fbranch any/c])
  #:transparent)

(define-struct/contract ifen-stmt
  ([condition pair?]
   [tbranch any/c]
   [fbranch any/c])
  #:transparent)

(define-struct/contract while-stmt
  ([condition pair?]
   [body any/c])
  #:transparent)

(define-struct/contract mem-print
  ([var any/c])
  #:transparent)
