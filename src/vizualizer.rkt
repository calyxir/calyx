#lang racket/gui

(require graph)

(require racket/gui/base
         mrlib/graph
         "component.rkt"
         "ast.rkt")
(provide plot-compute
         plot-component)

(define graph-board%
  (graph-pasteboard-mixin pasteboard%))

(define node-snip-class
  (make-object
   (class snip-class%
     (super-new)
     (send this set-classname "node-snip"))))

(send (get-the-snip-class-list) add node-snip-class)

(define node-snip%
  (class snip%
    (init-field value active)
    (super-new)
    (send this set-snipclass node-snip-class)

    (define char-width 12)
    (define value-width
      (+ 4
         (* char-width (string-length (~a value)))))

    (define/public (get-value) value)
    (define/public (get-width) value-width)

    (define/override (get-extent dc x y width height descent space lspace rspace)
      (when width (set-box! width value-width))
      (when height (set-box! height 20))
      (when descent (set-box! descent 0.0))
      (when space (set-box! space 0.0))
      (when lspace (set-box! lspace 0.0))
      (when rspace (set-box! rspace 0.0)))

    (define/override (draw dc x y . other)
      (define font (send the-font-list find-or-create-font 10 'modern 'normal 'normal))
      (send dc set-font font)
      (define label (~a value))
      (cond
        [(not active)
         (send dc set-pen "blue" 1 'solid)
         (send dc set-brush "blue" 'bdiagonal-hatch)
         (send dc draw-rectangle x y value-width 20)
         (send dc set-text-foreground "red")
         (send dc draw-text label (+ x (/ value-width 4)) (+ y 3))
         (send dc set-brush "white" 'transparent)]
        [active
         (send dc set-text-background "black")
         (send dc draw-text label (+ x (/ value-width 4)) (+ y 3))
         (send dc set-pen "black" 1 'solid)
         (send dc draw-rectangle x y value-width 20)
         (send dc set-text-foreground "black")
         (send dc draw-text label (+ x (/ value-width 4)) (+ y 3))]))))

(define node%
  (graph-snip-mixin node-snip%))

(define (plot-comp board comp vals inactive)
  ;; clear old graph
  (send board erase)

  (define spacing 75)
  (define center 250)

  (define nodes
    (map (lambda (vert)
           (let* ([active (not (member vert inactive))]
                  [node (new node% [value vert] [active active])])
             (send board insert node 0 0)
             node))
         (get-vertices (convert-graph comp))))

  (define g (convert-graph comp))

  ;; add all the edges
  (map (lambda (parent)
         (for-each (lambda (neigh-l)
                     (define obj-i
                       (index-where
                        nodes
                        (lambda (item) (equal? neigh-l (send item get-value)))))
                     (define child (list-ref nodes obj-i))
                     (add-links parent child)
                     (let* ([u (send parent get-value)]
                            [v (send child get-value)]
                            [label (~a (if (= +inf.0 (edge-weight g u v))
                                           ""
                                           (edge-weight g u v)))])
                       (cond
                         [(has-edge? g u v)
                          (set-link-label parent child label)])))
                   (get-neighbors g (send parent get-value))))
       nodes)

  (dot-positioning board "dot"))

;; ==========================

(define (plot-compute comp inputs
                      #:memory [memory (make-immutable-hash)]
                      #:animate [animate 100])

  (define board (new graph-board%))

  (define toplevel
    (new (class frame% (super-new)
           (define/augment (on-close)
             (send timer stop)
             (kill-thread compute-worker)
             (thread-wait compute-worker)))
         [label (~a (component-name comp))]
         [width (* 50 10)]
         [height (* 50 10)]))

  (define canvas
    (new (class editor-canvas% (super-new)
           (define/override (on-char evt)
             (match (send evt get-key-code)
               [#\space (start/stop-animation)]
               [#\n (next)]
               [_ (void)])))
         [parent toplevel]
         [style '(no-hscroll no-vscroll)]
         [horizontal-inset 0]
         [vertical-inset 0]
         [editor board]))

  (define control-panel
    (new horizontal-panel%
         [parent toplevel]
         [alignment '(center center)]
         [stretchable-height #f]))

  (define (start/stop-animation)
    (if (equal? (send play get-label) "Play")
        (begin
          (send play set-label "Reset")
          (send timer start animate))
        (begin
          (send play set-label "Play")
          (send timer stop)
          (thread-send compute-worker 'stop))))

  (define play
    (new button%
         [parent control-panel]
         [label "Play"]
         [callback (lambda (button event) (start/stop-animation))]))

  (define forward
    (new button%
         [parent control-panel]
         [label "Step"]
         [callback (lambda (button event) (next))]))

  (define frame-number 0)

  (define index-label
    (new message%
         [parent control-panel]
         [label (format "Frame ~v" frame-number)]))

  (define (update tup)
    (send index-label set-label (format "Frame ~v" frame-number))
    (send board begin-edit-sequence)
    (plot-comp board comp
               (ast-tuple-state tup)
               (ast-tuple-inactive tup))
    (send board end-edit-sequence))

  (define (next)
    (when (not (thread-running? compute-worker))
      (set! compute-worker (start-compute-worker)))
    (thread-send compute-worker 'next))

  (define (start-compute-worker)
    (set! frame-number 0)
    (thread
     (lambda ()
       (compute comp inputs
                #:memory memory
                #:hook (lambda (tup)
                         (match (thread-receive)
                           ['next
                            (update tup)
                            (set! frame-number (add1 frame-number))]
                           ['stop
                            (kill-thread (current-thread))]))))))
  (define compute-worker (start-compute-worker))

  (define timer
    (new timer%
         [notify-callback (lambda () (next))]))

  (send toplevel show #t)
  (next))

(define (plot-component comp)
  (define board (new graph-board%))

  (define toplevel
    (new frame%
         [label (~a (component-name comp))]
         [width (* 50 10)]
         [height (* 50 10)]))

  (define canvas
    (new editor-canvas%
         [parent toplevel]
         [style '(no-hscroll no-vscroll)]
         [horizontal-inset 0]
         [vertical-inset 0]
         [editor board]))

  (define (update)
    (send board begin-edit-sequence)
    (plot-comp board comp
               (input-hash '())
               '())
    (send board end-edit-sequence))

  (send toplevel show #t)
  (update))
