#lang racket/base

(require json
         racket/list
         racket/format
         "ast.rkt")

(provide json->memory)

(define (convert-1darray lst)
  (foldl (lambda (x i acc)
           (hash-set acc i x))
         (make-immutable-hash)
         lst
         (build-list (length lst) values)))

(define (convert-2darray lst)
  (foldl (lambda (l i acc)
           (foldl (lambda (x j acc)
                    (hash-set acc (~a i 'x j) x))
                  acc
                  l
                  (build-list (length l) values)))
         (make-immutable-hash)
         lst
         (build-list (length lst) values)))

(define (list-2d? lst)
  (if (list? lst)
      (list? (car lst))
      #f))

(define (json->memory filename)
  (define data
    (with-input-from-file filename
      (lambda () (read-json))))

  (make-immutable-hash
   (hash-map data
             (lambda (k v)
               (define v-p
                 (cond [(list-2d? v) (convert-2darray v)]
                       [(list? v) (convert-1darray v)]
                       [else v]))
               `(,k . ,(mem-tuple v-p (hash)))))))
