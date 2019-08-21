#lang racket/base

(require racket/match
         racket/contract)

(provide (struct-out port)
         infinite-port?
         find-port
         name->port
         split-port-ok?
         join-port)

(define-struct/contract port
  ([name symbol?]
   [width (and/c number? positive?)])
  #:transparent)

(define (infinite-port? p)
  (equal? (port-name p) 'inf#))

(define (find-port p lst)
  (findf (lambda (x) (equal? x p)) lst))

(define (name->port comp-name name lst)
  (define res (findf (lambda (x) (equal? (port-name x) name)) lst))
  (when (not res)
    (error 'name->port "Could not find port ~v in ~v ports: ~v" name comp-name (map port-name lst)))
  res)

(define (split-port-ok? p pt)
  (match p
    [(port name width)
     (if (and (< 0 pt) (< pt width))
         (void)
         (error "The split point:" pt "was invalid!"))]))

(define (join-port p1 p2 name)
  (port name (+ (port-width p1) (port-width p2))))
