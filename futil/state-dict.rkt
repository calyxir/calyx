#lang racket/base

(require racket/dict
         racket/list
         racket/hash)

(provide (all-defined-out))

(struct state-dict (v)
  #:methods gen:dict
  [(define (dict-ref dict key
                     [default (lambda () (error "key not found" key))])
     (cond [(assoc key (state-dict-v dict)) => cdr]
           [else (if (procedure? default) (default) default)]))
   (define (dict-set dict key val)
     (state-dict
      (remove-duplicates (cons (cons key val) (state-dict-v dict))
                         #:key car)))
   (define (dict-remove dict key)
     (define al (state-dict-v dict))
     (state-dict (remove* (filter (lambda (p) (equal? (car p) key)) al) al)))
   (define (dict-count dict)
     (length (state-dict-v dict)))
   (define (dict-map dict proc)
     (map (lambda (x) (proc (car x) (cdr x)))
          (state-dict-v dict)))
   (define (dict->list dict)
     (state-dict-v dict))
   (define (dict-keys dict)
     (map car (state-dict-v dict)))]
  #:methods gen:custom-write
  [(define (write-proc state port mode)
     (fprintf port
              "state-dict#~v"
              (state-dict-v state)))])

(define (state-union s1 s2
                     #:combine [combine (lambda (a b)
                                          (error 'hash-union
                                                 "Clashing keys"))]
                     #:combine/key [combine/key (lambda (k a b) (combine a b))])
  (state-dict
   (append
    (foldl (lambda (x acc)
             (define key (car x))
             (cons (if (dict-has-key? s2 key)
                       `(,key . ,(combine/key
                                  key
                                  (dict-ref s1 key)
                                  (dict-ref s2 key)))
                       x)
                   acc))
           (list)
           (state-dict-v s1))
    (foldl (lambda (x acc)
             (if (dict-has-key? s1 (car x))
                 acc
                 (cons x acc)))
           (list)
           (state-dict-v s2)))))

;; (define (state-dict l) (make-immutable-hash l))

;; (define state-union hash-union)

(define (empty-state)
   (state-dict '()))

(define (state-map dict proc)
   (state-dict (dict-map dict proc)))
