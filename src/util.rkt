#lang racket/base
(require racket/logging)

(provide keyword-lambda
         debug-wrap)

;; syntatic sugar that makes it nice to define procedures in the way
;; that component expects them
(define-syntax-rule (keyword-lambda (arg ...)
                                    ([var = body1 ...] ...)
                                    [kw => body2 ...] ...)
  (lambda (h)
    (define arg (hash-ref h 'arg)) ...
    (define var (begin body1 ...)) ...
    (make-immutable-hash `((kw . ,(begin body2 ...)) ...))))

(define-syntax-rule (debug-wrap body ...)
  (with-logging-to-port (current-output-port)
    (lambda ()
      body ...)
    'debug))
