#lang racket
(require racket/hash
         "component.rkt")
(provide (struct-out deact-stmt)
         (struct-out if-stmt)
         (struct-out while-stmt)
         (struct-out par-comp)
         (struct-out seq-comp))

;; type of statements
(struct deact-stmt (mod) #:transparent)
(struct if-stmt (condition tbranch fbranch) #:transparent)
(struct while-stmt (condition body) #:transparent)
(struct par-comp (stmts) #:transparent)
(struct seq-comp (stmts) #:transparent)
