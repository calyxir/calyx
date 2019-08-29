#lang racket/base

(require json
         racket/list
         racket/format
         racket/dict
         racket/port
         racket/string
         racket/hash
         threading
         "ast.rkt"
         "state-dict.rkt"
         (for-syntax racket/base
                     syntax/parse))

(provide json->memory
         generate-json)

(define (convert-1darray lst)
  (foldl (lambda (x i acc)
           (dict-set acc i x))
         (empty-state)
         lst
         (build-list (length lst) values)))

(define (convert-2darray lst)
  (foldl (lambda (l i acc)
           (foldl (lambda (x j acc)
                    (dict-set acc (cons i j) x))
                  acc
                  l
                  (build-list (length l) values)))
         (empty-state)
         lst
         (build-list (length lst) values)))

(define (list-2d? lst)
  (if (list? lst)
      (list? (car lst))
      #f))

(define (zip-with-idx lst)
  (if (list? lst)
      (map (lambda (x i) (cons `(,i) x))
           lst
           (build-list (length lst) values))
      lst))

(define (build-mem mem)
  (cond [(and (pair? mem) (list? (car mem)))  ; if the head is a list
         (~>
          ; recursive call on every item in the list
          (map (lambda (x) (build-mem x))
               mem)
          ; add the idx for the last dim
          zip-with-idx
          ; map over each row
          (map (lambda (row)
                 ; map over content in each row to merge idxes
                 (map (lambda (item)
                        (cons (append (car row) (car item)) ; merge idx
                              (cdr item)))                  ; pass along content
                      (cdr row)))
               _)
          ; flatten list one level
          (apply append _))]
        [else (zip-with-idx mem)]))

(define (json->memory filename)
  (~> (with-input-from-file filename
        (lambda () (read-json)))
      (dict-map _
                (lambda (k v)
                  `(,k . ,(mem-tuple
                           (build-mem v)
                           (empty-state)))))
      state-dict))

(define (format-list l)
  (if (not (list? (car l)))
      (string-join (map ~a l)
                   ","
                   #:before-first "["
                   #:after-last "]")
      (string-join (map format-list l)
                   ",\n"
                   #:before-first "["
                   #:after-last "]")))

;; crude json formatting
(define (format-data d)
  (if (number? d)
      d
      (format-list d)))

(define (display-json json)
  (display
   (string-join
    (dict-map json
              (lambda (k v)
                (format "\"~a\": ~a" k (format-data v))))
    ",\n"
    #:before-first "{\n"
    #:after-last "\n}")))

(define (create-list dim-lst proc)
  (cond [(empty? dim-lst) (error "Can't create a zero-dimensional list")]
        [(= (length dim-lst) 1)
         (build-list (car dim-lst) proc)]
        [else
         (build-list (car dim-lst)
                     (lambda (v) (create-list (cdr dim-lst) proc)))]))

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
    (pattern (x:id)
             #:with obj #'(lambda (proc)
                            (hash 'x (proc 0))))
    (pattern (x:id dim ...+)
             #:with obj #'(lambda (proc)
                            (hash 'x (create-list (list dim ...) proc)))))
  (syntax-parse stx
    [(_ fn type:gen-type phrase:phrase ...)
     #'(with-output-to-file fn
         #:mode 'text
         #:exists 'replace
         (lambda ()
           (display-json
            (hash-union (phrase.obj type.fun) ...
                        #:combine/key
                        (lambda (k v0 v1)
                          (error 'generate-json
                                 "~v was in multple phrases!"
                                 k))))))
     ]))
