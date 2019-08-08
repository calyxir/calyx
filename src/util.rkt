#lang racket/base
(require racket/logging)

(provide keyword-lambda
         listen-debug
         unlisten-debug)

;; syntatic sugar that makes it nice to define procedures in the way
;; that component expects them
(define-syntax-rule (keyword-lambda (arg ...)
                                    ([var = body1 ...] ...)
                                    [kw => body2 ...] ...)
  (lambda (h)
    (define arg (hash-ref h 'arg)) ...
    (define var (begin body1 ...)) ...
    (make-immutable-hash `((kw . ,(begin body2 ...)) ...))))

(define futil-logger (make-logger 'futil-logger))
(define debug-rc (make-log-receiver futil-logger 'debug))
(define logger-thrd #f)
(current-logger futil-logger)

(define (disconnect-thread)
  (cond [(thread? logger-thrd) (kill-thread logger-thrd)]))

(define (listen-debug)
  (disconnect-thread)
  (set! logger-thrd
        (thread (lambda ()
                  (let loop ()
                    (define msg (sync debug-rc))
                    (printf "[~a] ~a\n" (vector-ref msg 0) (vector-ref msg 1))
                    (loop))))))
(define (unlisten-debug)
  (disconnect-thread)
  (set! logger-thrd
        (thread (lambda ()
                  (let loop ()
                    (define msg (sync debug-rc))
                    (loop))))))
(unlisten-debug)


