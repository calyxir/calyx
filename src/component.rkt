#lang racket/base

(require graph
         racket/hash
         racket/list
         racket/match
         "port.rkt")

(provide (struct-out component)
         transform-control
         input-component
         output-component
         default-component
         make-constant
         connect!
         add-submod!
         get-submod!
         split!
         convert-graph)

(struct component (;; name of the component
                   name
                   ;; list of input ports
                   [ins #:mutable]
                   ;; list of output ports
                   [outs #:mutable]
                   ;; hashtbl of sub components keyed on their name
                   submods
                   ;; hashtbl keeping track of split nodes
                   splits
                   ;; list of (inactive lst * constr list) tuples
                   control
                   ;; procedure representing this modules computation
                   proc
                   ;; graph representing internal connections
                   graph
                   ;; true when this component should always activate
                   activation-mode))

;; (define component/c
;;   (struct/dc
;;    [a (list/c any/c)]
;;    [b (list/c number?)]))

;; creates a default component given a name for the component,
;; a list of input port names, and a list of output port names
(define (empty-graph) (weighted-graph/directed '()))

;; creates a component with a single infinite output port of width w
;; and no input ports. Designed to be used as the input of a component.
(define (input-component w) (component
                             'input
                             '()
                             (list (port 'inf# w))
                             (make-hash) ; submods
                             (make-hash) ; splits
                             '() ;; (list (control-pair '() '()))
                             (keyword-lambda (inf#) () [inf# => inf#])
                             (empty-graph)
                             #t))

;; creates a component with a single infinite input port of width w
;; and no output ports. Designed to be used as the output of a component.
(define (output-component w) (component
                              'output
                              (list (port 'inf# w))
                              '()
                              (make-hash) ; submods
                              (make-hash) ; splits
                              '() ;; (list (control-pair '() '()))
                              (keyword-lambda (inf#) () [inf# => inf#])
                              (empty-graph)
                              #f))

(define (transform-control control) control)

;; TODO: maybe add vertices for ins and outs

;; given a name, list of input ports, and list of output ports, creates
;; a component an empty graph and the appropriate input and output ports
;; in the hashtable.
(define (default-component
          name
          ins
          outs
          proc
          #:control [control '() ;; (list (control-pair '() '()))
                             ]
          #:mode [mode #f])
  (let ([htbl (make-hash)]
        [g (empty-graph)])
    (for-each (lambda (p) ; p is a port
                (hash-set! htbl (port-name p) (input-component (port-width p))))
              ins)
    (for-each (lambda (p)
                (hash-set! htbl (port-name p) (output-component (port-width p))))
              outs)
    (component
     name
     ins
     outs
     htbl          ; sub-mods
     (make-hash)   ; splits
     control
     proc
     g
     mode)))

(define (make-constant n width)
  (default-component n '() (list (port 'inf# width)) (keyword-lambda () () [inf# => n])))

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
  (void))
(define (consume-out! comp port)
  (void))

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

  (define (mint-inactive-hash comp name)
  (make-immutable-hash (map
                        (lambda (x)
                          `((,name . ,(port-name x)) . #f))
                        (append
                         (component-outs (get-submod! comp name))
                         (filter-map
                          (lambda (x) (and (equal? name (port-name x))
                                           (port 'inf# (port-width x))))
                          (component-outs comp))))))

(define (mint-remembered-hash comp hsh name)
  (define base (mint-inactive-hash comp name))
  (make-immutable-hash
   (hash-map base (lambda (k v) `(,k . ,(hash-ref hsh k))))))

(define (submod-compute comp inputs name)
  (define ins (make-immutable-hash (hash-map inputs (lambda (k v) `(,(cdr k) . ,v)))))
  (if (andmap (lambda (x) x) (hash-values ins))
      (make-immutable-hash
       (hash-map ((component-proc (get-submod! comp name)) ins)
                 (lambda (k v) `((,name . ,k) . ,v))))
      (mint-inactive-hash comp name)))

(define (convert-graph comp [vals #f])
  (define g (component-graph comp))
  (define newg (empty-graph))
  (for-each (lambda (edge)
              (match edge
                [(cons (cons u _) (cons (cons v _) _))
                 (if vals
                     (begin
                       (add-directed-edge! newg u v (hash-ref vals (car edge))))
                     (add-directed-edge! newg u v)
                     )
                 ]))
            (get-edges g))
  newg)
