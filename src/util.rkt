#lang racket/base
(require racket/logging)

(provide keyword-lambda
         debug
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

;; debugging stuffs
(define should-print-debug? (make-parameter #f))

(define-syntax-rule (debug fmt v ...)
  (begin
    (when (should-print-debug?)
      (displayln (format fmt v ...)))
    (values v ...)))

;; sugar for wrapping functions that I want to log
(define-syntax-rule (debug-wrap body ...)
  (parameterize ([should-print-debug? #t])
    body ...))
