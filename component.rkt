#lang racket
(require graph
         "dis-graphs.rkt")
(provide keyword-lambda
         (struct-out component)
         (struct-out port)
         name->port
         input-component
         output-component
         default-component
         make-constant
         connect!
         add-submod!
         get-submod!
         split!
         compute
         stabilize
         plot)

(define-syntax-rule (keyword-lambda (arg ...) body ...)
  (lambda (h)
    (define arg (hash-ref h 'arg)) ...
    ((lambda (arg ...) body ...) arg ...)))

(struct port (name width) #:transparent)
(define (infinite-port? p)
  (equal? (port-name p) 'inf#))
(define (find-port p lst)
  (findf (lambda (x) (equal? x p)) lst))
(define (name->port name lst)
  (findf (lambda (x) (equal? (port-name x) name)) lst))
(define (split-port-ok? p pt)
  (match p
    [(port name width)
     (if (and (< 0 pt) (< pt width))
         (void)
         (error "The split point:" pt "was invalid!"))]
    [_ (error "Impossible")]))
(define (join-port p1 p2 name)
  (port name (+ (port-width p1) (port-width p2))))

(struct component (name               ;; name of the component
                   [ins #:mutable]    ;; list of input ports
                   [outs #:mutable]   ;; list of output ports
                   submods            ;; hashtbl of sub components keyed on their name
                   splits             ;; hashtbl keeping track of split nodes
                   proc               ;; procedure representing this modules computation
                   graph              ;; graph representing internal connections
                   primitive          ;; true when this component is primitive
                   ))

;; creates a default component given a name for the component,
;; a list of input port names, and a list of output port names
(define (empty-graph) (weighted-graph/directed '()))

;; creates a component with a single infinite output port of width w
;; and no input ports. Designed to be used as the input of a component.
(define (input-component w) (component
                             'input
                             '()
                             (list (port 'inf# w))
                             (make-hash)
                             (make-hash)
                             void
                             (empty-graph)
                             #f))

;; creates a component with a single infinite input port of width w
;; and no output ports. Designed to be used as the output of a component.
(define (output-component w) (component
                              'output
                              (list (port 'inf# w))
                              '()
                              (make-hash)
                              (make-hash)
                              (lambda (h) (hash-ref h 'inf#))
                              (empty-graph)
                              #f))

;; TODO: maybe add vertices for ins and outs

;; given a name, list of input ports, and list of output ports, creates
;; a component an empty graph and the appropriate input and output ports
;; in the hashtable.
(define (default-component name ins outs proc [prim #f])
  (let ([htbl (make-hash)]
        [g (empty-graph)])
    (for-each (lambda (p) ; p is a port
                (hash-set! htbl (port-name p) (input-component (port-width p))))
              ins)
    (for-each (lambda (p)
                (hash-set! htbl (port-name p) (output-component (port-width p))))
              outs)
    (component name ins outs htbl (make-hash) proc g prim)))

(define (make-constant n width)
  (default-component n '() (list (port 'inf# width)) (keyword-lambda () n) #t))

;; Looks for an input/output port matching [port] in [comp]. If the port is found
;; and is equal to the value [#f], then this function does nothing. Otherwise
;; it removes that port from [comp].
(define (consume! comp port set-prop! get-prop name)
  (let* ([lst (get-prop comp)]
         [p (find-port port lst)])
    (if p
        (if (infinite-port? p)
            (void)
            (set-prop! comp (remove p lst)))
        (error "Couldn't find" port "in" (component-name comp) name))))

(define (consume-in! comp port)
  (void)
  ;; (consume! comp port set-component-ins! component-ins 'input)
  )
(define (consume-out! comp port)
  (void)
  ;; (consume! comp port set-component-outs! component-outs 'outputs)
  )

(define (add-submod! comp name mod)
  (hash-set! (component-submods comp) name mod))
(define (get-submod! comp name)
  (hash-ref (component-submods comp) name))

(define (add-edge! comp src src-port tar tar-port width)
  (let ([src-name (hash-ref (component-splits comp) src src)]
        [tar-name (hash-ref (component-splits comp) tar tar)]
        [src-port-name (port-name src-port)]
        [tar-port-name (port-name tar-port)])
    (add-directed-edge!
     (component-graph comp)
     `(,src-name . ,src-port-name)
     `(,tar-name . ,tar-port-name)
     width)))

(define (connect! comp src src-portname tar tar-portname)
  (let* ([src-submod (get-submod! comp src)]
         [tar-submod (get-submod! comp tar)]
         [src-port (name->port src-portname (component-outs src-submod))]
         [tar-port (name->port tar-portname (component-ins tar-submod))])
    (if (= (port-width src-port) (port-width tar-port))
        (begin
          (consume-out! src-submod src-port)
          (consume-in! tar-submod tar-port)
          (add-edge! comp src src-port tar tar-port (port-width src-port)))
        (error "Port widths don't match!"
               src-port '!= tar-port))))

(define (split! comp name split-pt name1 name2)
  (define (port-eq x y) (equal? (port-name x) (port-name y)))
  (define (help port make-comp)
    (split-port-ok? port split-pt)
    (hash-set! (component-submods comp) name1 (make-comp split-pt))
    (hash-set! (component-submods comp)
               name2
               (make-comp (- (port-width port) split-pt))))
  (cond [(name->port name (component-ins comp))
         => (lambda (p)
              (help p input-component)
              (hash-set! (component-splits comp) name1 name)
              (hash-set! (component-splits comp) name2 name))]
        [(name->port name (component-outs comp))
         => (lambda (p)
              (help p output-component)
              (hash-set! (component-splits comp) name1 name)
              (hash-set! (component-splits comp) name2 name))]
        [else (error "Port not found in the inputs!")]))

(define (backtrack comp outlst)
  (apply append (map (lambda (vert)
                       (match vert
                         [(cons v _)
                          (map (lambda (kw) `(,v . ,kw))
                               (map port-name (component-ins (get-submod! comp v))))]))
                     outlst)))

(define (get-neighbors comp vertex)
  (define valid-ports (map port-name (component-ins (get-submod! comp vertex))))
  (define neighs (map (lambda (p)
                        (sequence->list (in-neighbors
                                         (transpose (component-graph comp))
                                         `(,vertex . ,p))))
                      valid-ports))
  (map (lambda (p n) `(,p . ,n)) valid-ports neighs))

(define (stabilize comp inputs vertex)
  (if (member vertex (map port-name (component-ins comp)))
      (hash-ref inputs vertex)
      (let* ([neighs (get-neighbors comp vertex)]
             [sub (get-submod! comp vertex)]
             [vals (map (lambda (pair)
                          (match pair
                            [(cons name (cons (cons v _) _))
                             `(,name . ,(stabilize comp inputs v))]))
                        neighs)]
             [proc (component-proc sub)])
        (proc (make-hash vals)))))

(define (compute comp inputs)
  (if (component-primitive comp)
      ((component-proc comp) inputs)
      (flatten (map (lambda (v) (stabilize comp inputs v))
                    (map port-name (component-outs comp))))))

(define (convert-graph g)
  (define newg (empty-graph))
  (for-each (lambda (edge)
              (match edge
                [(cons (cons u _) (cons (cons v _) _))
                 (add-directed-edge! newg u v)]))
            (get-edges g))
  newg)

(define (plot comp)
  (plot-graph (show-board (component-name comp)) (convert-graph (component-graph comp))))
