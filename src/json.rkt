#lang racket/base

(require json
         racket/list
         racket/format
         racket/hash
         racket/port
         racket/string
         "ast.rkt"
         (for-syntax racket/base
                     syntax/parse))

(provide json->memory
         generate-json)

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

;; crude json formatting
(define (format-list l)
  (define (format-1d l)
    (string-join (map (lambda (x) (~a x)) l)
                 ","
                 #:before-first "["
                 #:after-last "]"))
  (if (list-2d? l)
      (string-join (map (lambda (x)
                          (format-1d x))
                        l)
                   ",\n"
                   #:before-first "["
                   #:after-last "]"
                   )
      (format-1d l)))

(define (display-json json)
  (display
   (string-join
    (hash-map json
              (lambda (k v)
                (format "\"~a\": ~a" k (format-list v))))
    ",\n"
    #:before-first "{\n"
    #:after-last "\n}")))


;; syntax for json creation
(define-syntax (generate-json stx)
  (define-syntax-class gen-type
    #:attributes (fun)
    #:datum-literals (zero random)
    (pattern (random low high)
             #:with fun #'(lambda (v) (random low high)))
    (pattern (zero)
             #:with fun #'(lambda (v) 0)))

  (define-syntax-class phrase
    (pattern (x:id dim)
             #:with obj #'(lambda (proc)
                            (hash 'x (build-list dim proc))))
    (pattern (x:id i-dim j-dim)
             #:with obj #'(lambda (proc)
                            (hash 'x
                                  (build-list
                                   i-dim
                                   (lambda (i)
                                     (build-list
                                      j-dim
                                      (lambda (j) (proc `(,i . ,j))))))))))
  (syntax-parse stx
    [(_ fn type:gen-type phrase:phrase ...)
     #'(when (= 0 (vector-length (current-command-line-arguments)))
         (with-output-to-file fn
           #:mode 'text
           #:exists 'replace
           (lambda ()
             (display-json
              (hash-union (phrase.obj type.fun) ...
                          #:combine/key (lambda (k v0 v1)
                                          (error (format "~v was in multple phrases!" k))))
              ))))
     ]))
