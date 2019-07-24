#lang racket/base

(require racket/hash
         racket/bool
         racket/sequence
         racket/list
         racket/pretty
         racket/format
         racket/match
         graph
         "component.rkt"
         "port.rkt")

(provide (struct-out par-comp)
         (struct-out seq-comp)
         (struct-out deact-stmt)
         (struct-out if-stmt)
         (struct-out ifen-stmt)
         (struct-out while-stmt)
         (struct-out ast-tuple)
         compute)

;; type of statements
(struct par-comp (stmts) #:transparent)
(struct seq-comp (stmts) #:transparent)
(struct deact-stmt (mods) #:transparent)
(struct if-stmt (condition tbranch fbranch) #:transparent)
(struct ifen-stmt (condition tbranch fbranch) #:transparent)
(struct while-stmt (condition body) #:transparent)

;; a hash union that tries to make overlapping keys non-false
;;   if v1 or v2 is #f, choose non-false option
;;   otherwise, if both v1 and v2 have values, choose v2
(define (save-hash-union h1 h2)
  (hash-union
   h1
   h2
   #:combine (lambda (v1 v2) (if (and v1 v2) v2 (xor v1 v2)))))

;; a hash union function that always prefers h2 when keys overlap
(define (clob-hash-union h1 h2)
  (hash-union h1 h2 #:combine (lambda (v1 v2) v2)))

;; a hash union function that chooses non-false values
;; over false ones, keeps equal values the same,
;; and errors on non-equal values
(define (equal-hash-union h0 h1
                          #:error [error-msg "Expected same values or one false."])
  (hash-union
   h0
   h1
   #:combine
   (lambda (v0 v1)
     (cond
       [(xor v0 v1) (or v1 v0)] ; when only one is false, choose the true one.
       [(equal? v0 v1) v0]      ; v0 = v1, then v0
       [else
        (raise-result-error 'equal-hash-union error-msg `(,h0 ,h1))]))))

(define (input-hash comp lst)

  (define empty-hash
    (make-immutable-hash
     (map (lambda (x) `(,x . #f))
          (append
           (map car (get-edges (component-graph comp)))
           (map (lambda (p)
                  `(,(port-name p) . inf#))
                (component-outs comp))))))

  (clob-hash-union
   empty-hash
   (make-immutable-hash
    (map (lambda (x) `((,(car x) . inf#) . ,(cdr x))) lst))))

(define (transform comp inputs name)
  (if (findf (lambda (x) (equal? name (port-name x))) (component-ins comp))
      (make-immutable-hash `(((,name . inf#) . ,(hash-ref inputs `(,name . inf#)))))
      (begin
        (let* ([sub (get-submod! comp name)]
               [ins (map port-name (component-ins sub))])  ; XXX: deal with port widths
          (make-immutable-hash
           (map (lambda (in)
                  (define neighs
                    (sequence->list
                     (in-neighbors
                      (transpose (component-graph comp)) `(,name . ,in))))
                  (define filt-neighs-vals
                    (filter-map (lambda (x) (hash-ref inputs x)) neighs))
                  (define neighs-vals
                    (if (empty? filt-neighs-vals)
                        (map (lambda (x) (hash-ref inputs x)) neighs)
                        filt-neighs-vals))
                  `((,name . ,in) . ,(car neighs-vals)))
                ins))))))

(define (mint-inactive-hash comp name)
  (make-immutable-hash
   (map
    (lambda (x)
      `((,name . ,(port-name x)) . #f))
    (append
     (component-outs (get-submod! comp name))
     (filter-map
      (lambda (x) (and (equal? name (port-name x))
                       (port 'inf# (port-width x))))
      (component-outs comp))))))

;; creates a hash for the outputs of sub-component [name] in [comp]
;; that has values from [hsh]
(define (mint-remembered-hash comp hsh name)
  (define base (mint-inactive-hash comp name))
  (make-immutable-hash
   (hash-map base (lambda (k v) `(,k . ,(hash-ref hsh k))))))

(struct memory-tup (current sub-mem) #:transparent)
;; given a subcomponent (comp name) a state and memory,
;; run subcomponents proc with state and memory and
;; return updated state and memory
(define (submod-compute comp name state tot-mem)
  ;; state is of the form (((sub . port) . val) ...)
  ;; change to ((port . val) ...)
  (define ins
    (make-immutable-hash
     (hash-map state (lambda (k v) `(,(cdr k) . ,v)))))

  ;; get the current submemory out of mem, creating one if
  ;; it doesn't exist
  (define sub-mem
    (if (hash-has-key? tot-mem name)
        (hash-ref tot-mem name)
        (memory-tup (make-immutable-hash)
                    (make-immutable-hash))))

  ;; add memory to ins
  (define ins-p (hash-set ins 'mem# sub-mem))

  (let* ([proc (component-proc (get-submod! comp name))]
         [res (proc ins-p)]
         [sub-mem-p (if (hash-has-key? res 'mem#)
                        (hash-ref res 'mem#)
                        (memory-tup (make-immutable-hash)
                                    (make-immutable-hash)))]
         [res-wo-mem (hash-remove res 'mem#)])
    (values
     (make-immutable-hash
      (hash-map res-wo-mem
                (lambda (k v) `((,name . ,k) . ,v))))
     (hash-set tot-mem name sub-mem-p))))

(define (compute-step comp memory state inactive)
  ;; sort the components so that we evaluate things in the right order
  ;; (define order (top-order comp))
  (define order (tsort (convert-graph comp)))

  ;; function that goes through a given hashmap and sets all disabled wires to false
  (define (filt hsh)
    (make-immutable-hash
     (hash-map hsh (lambda (k v) (if (member (car k) inactive)
                                     `(,k . #f)
                                     `(,k . ,v))))))
  ;; for every node in the graph, call submod-compute;
  ;; making sure to thread the state through properly
  (struct accum (state memory))
  (define filled
    (foldl (lambda (sub acc)
             (define res
               (if (member sub inactive)
                   ; inactive (set sub to false in acc)
                   (struct-copy accum acc
                                [state
                                 ; use save-union because other mods might
                                 ; need values on the wires. we'll set them
                                 ; to #f later
                                 (save-hash-union (accum-state acc)
                                                  (mint-inactive-hash comp sub))])
                   ; active
                   (let*-values
                       (; remove disabled wires from memory, then union with state
                        [(vals) (filt (save-hash-union
                                       (memory-tup-current (accum-memory acc))
                                       (accum-state acc)))]
                        [(trans) (transform comp vals sub)]
                        [(state-p sub-mem-p) ; pass in state and memory to submodule
                         (submod-compute comp sub trans
                                         (memory-tup-sub-mem (accum-memory acc)))]
                        [(curr-mem-p) ; update this modules curr memory
                         (if (component-activation-mode (get-submod! comp sub))
                             ; is a register, update memory
                             ; prefering first non-false then new vals
                             (save-hash-union (memory-tup-current (accum-memory acc))
                                              (mint-remembered-hash comp state-p sub))
                             ; is not a register
                             (memory-tup-current (accum-memory acc)))])
                     (accum
                      (save-hash-union (accum-state acc) state-p)
                      (memory-tup curr-mem-p sub-mem-p)))))
             res)
           (accum state memory)
           order))

  ;; after we have used all the values, set the wires coming from inactive modules to #f
  (define filled-mod
    (foldl (lambda (x acc)
             (hash-set acc x #f))
           (accum-state filled)
           (filter (lambda (x) (member (car x) inactive))
                   (hash-keys (accum-state filled)))))

  ;; (define filled-mod filled)
  (values
   filled-mod                ; state
   (accum-memory filled)     ; memory
   (map (lambda (x)          ; output
          `(,(car x) . ,(hash-ref (accum-state filled) x)))
        (map (lambda (x) `(,(port-name x) . inf#)) (component-outs comp)))))

(define-syntax-rule (if-valued condition tbranch fbranch disbranch)
  (if condition
      (if (not (equal? condition 0))
          tbranch
          fbranch)
      disbranch))

(struct ast-tuple (inactive state memory history) #:transparent)

(define (merge-state st0 st1)
  (equal-hash-union st0 st1))

(define (merge-mem mem0 mem1)
  (match-define (memory-tup curr0 subm0) mem0)
  (match-define (memory-tup curr1 subm1) mem1)
  (memory-tup (equal-hash-union curr0 curr1 #:error "Invalid current mem merge!")
              (equal-hash-union subm0 subm1)))

(define (update-history ast-tup)
  (struct-copy ast-tuple ast-tup
               [history (cons ast-tup (ast-tuple-history ast-tup))]))

(define (ast-step comp tup ast)
  (match-define (ast-tuple inactive state memory history) tup)
  (log-debug "(open ast-step ~a" ast)
  (define result
    (match ast
      [(par-comp stmts)
       (define (merge-tup tup1 tup2)
         (match-let ([(ast-tuple inact-1 st-1 mem-1 hist-1)
                      tup1]
                     [(ast-tuple inact-2 st-2 mem-2 hist-2)
                      tup2])
           (ast-tuple
            (remove-duplicates (append inact-1 inact-2))
            (merge-state st-1 st-2)
            mem-1 ;; XXX fix this
            history)))
       ;; handle the case when we don't have any parallel stmts
       ;; (would be nice to do this in syntax)
       (if (empty? stmts)
           (ast-step comp tup (deact-stmt '()))
           (foldl merge-tup
                  (struct-copy ast-tuple tup
                               [state (make-immutable-hash)]
                               [memory (memory-tup (make-immutable-hash)
                                                   (make-immutable-hash))])
                  (map (lambda (s) (ast-step comp tup s))
                       stmts)))]
      [(seq-comp stmts)
       (foldl (lambda (s acc)
                (define acc-p (struct-copy ast-tuple acc
                                           [inactive (ast-tuple-inactive tup)]))
                (define res (ast-step comp acc-p s))
                (update-history res))
              tup
              stmts)]
      [(deact-stmt mods) ; compute step with this list of inactive modules
       (let*-values ([(st mem out)
                      (compute-step comp
                                    memory
                                    state
                                    mods)])
         (log-debug "state: ~a\n memory: ~a\n" st mem)
         (struct-copy ast-tuple tup
                      [state st]
                      [memory mem]
                      [inactive mods]))]
      [(if-stmt condition tbranch fbranch)
       (log-debug "if: ~a" state)
       (if-valued (hash-ref state condition)
                  (ast-step comp tup tbranch)
                  (ast-step comp tup fbranch)
                  tup)]
      [(ifen-stmt condition tbranch fbranch)
       (if (hash-ref state condition)
           (ast-step comp tup tbranch)
           (ast-step comp tup fbranch))]
      [(while-stmt condition body)
       (if-valued (hash-ref state condition)
                  (ast-step comp
                            (ast-step comp tup body)
                            ast)
                  tup
                  tup)]
      [_ (error "Malformed ast!" ast)]))
  (log-debug "close)")
  result)

(define (compute comp inputs #:memory [mem (memory-tup
                                            (make-immutable-hash)
                                            (make-immutable-hash))])
  (define ast (component-control comp))
  (define state (input-hash comp inputs))
  (log-debug "================")
  (log-debug "(start compute for ~a" (component-name comp))
  (log-debug "memory: ~a" mem)
  (define st-mem
    (struct-copy memory-tup mem
                 [current
                  (clob-hash-union
                   (make-immutable-hash
                    (map (lambda (x) `((,(car x) . inf#) . ,(cdr x)))
                         inputs))
                   (memory-tup-current mem))]))
  (log-debug (~a st-mem))
  (define result (ast-step comp (ast-tuple '() state st-mem '()) ast))

  (log-debug "~a" (ast-tuple-state result))
  (log-debug "~a" (ast-tuple-memory result))
  (log-debug "end compute)")
  (log-debug "================")
  result)
