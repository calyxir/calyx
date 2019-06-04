#lang racket
(require graph
         "dis-graphs.rkt")
(provide (struct-out component)
         (struct-out port)
         name->port
         input-component
         output-component
         default-component
         connect!
         add-submod!
         get-submod!
         split!
         plot)

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
                   [submods]          ;; hashtbl of sub components keyed on their name
                   splits             ;; hashtbl keeping track of split nodes
                   graph))            ;; graph representing internal connections

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
                             (empty-graph)))

;; creates a component with a single infinite input port of width w
;; and no output ports. Designed to be used as the output of a component.
(define (output-component w) (component
                              'output
                              (list (port 'inf# w))
                              '()
                              (make-hash)
                              (make-hash)
                              (empty-graph)))

;; TODO: maybe add vertices for ins and outs

;; given a name, list of input ports, and list of output ports, creates
;; a component an empty graph and the appropriate input and output ports
;; in the hashtable.
(define (default-component name ins outs)
  (let ([htbl (make-hash)])
    (for-each (lambda (p) ; n is a port
                (hash-set! htbl (port-name p) (input-component (port-width p))))
              ins)
    (for-each (lambda (p)
                (hash-set! htbl (port-name p) (output-component (port-width p))))
              outs)
    (component name ins outs htbl (make-hash) (empty-graph))))

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
  (consume! comp port set-component-ins! component-ins 'input))
(define (consume-out! comp port)
  (consume! comp port set-component-outs! component-outs 'outputs))

(define (add-submod! comp name mod)
  (hash-set! (component-submods comp) name mod))
(define (get-submod! comp name)
  (hash-ref (component-submods comp) name))

(define (add-edge! comp src tar width)
  (let ([src-name (hash-ref (component-splits comp) src src)]
        [tar-name (hash-ref (component-splits comp) tar tar)])
    (add-directed-edge! (component-graph comp) src-name tar-name width)))

(define (connect! comp src src-portname tar tar-portname)
  (let* ([src-submod (get-submod! comp src)]
         [tar-submod (get-submod! comp tar)]
         [src-port (name->port src-portname (component-outs src-submod))]
         [tar-port (name->port tar-portname (component-ins tar-submod))])
    (if (= (port-width src-port) (port-width tar-port))
        (begin
          (consume-out! src-submod src-port)
          (consume-in! tar-submod tar-port)
          (add-edge! comp src tar (port-width src-port)))
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

(define (plot comp)
  (plot-graph (show-board (component-name comp)) (component-graph comp)))
