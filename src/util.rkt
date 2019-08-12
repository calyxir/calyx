#lang racket/base
(require racket/logging
         racket/path
         racket/pretty)

(provide (all-defined-out))

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
      (pretty-display (format fmt v ...)))
    (values v ...)))

(define-syntax-rule (show-debug body ...)
  (parameterize ([should-print-debug? #t])
    body ...))

;; (define-logger base)
;; (define-logger worklist #:parent base-logger)
;; (define-logger compute #:parent worklist-logger)
;; (define-logger ast #:parent compute-logger)

;; (define-syntax-rule (show-debug body ...)
;;   (with-logging-to-port (current-output-port)
;;     (lambda ()
;;       body ...)
;;     #:logger base-logger
;;     'debug))

;; (define-syntax-rule (show-debug-worklist body ...)
;;   (with-logging-to-port (current-output-port)
;;     (lambda ()
;;       body ...)
;;     #:logger worklist-logger
;;     'debug))

;; (define-syntax-rule (show-debug-compute body ...)
;;   (with-logging-to-port (current-output-port)
;;     (lambda ()
;;       body ...)
;;     #:logger compute-logger
;;     'debug))

;; (define-syntax-rule (show-debug-ast body ...)
;;   (with-logging-to-port (current-output-port)
;;     (lambda ()
;;       body ...)
;;     #:logger ast-logger
;;     'debug))

(define (in-repl?)
  (= 0 (vector-length (current-command-line-arguments))))

;; file utilities
(define (benchmark-data-path default)
  (if (in-repl?)
      (simplify-path
       (build-path (current-directory) ".." "benchmarks" default))
      (build-path (current-directory) (vector-ref (current-command-line-arguments) 0))))
