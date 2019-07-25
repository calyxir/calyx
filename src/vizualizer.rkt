#lang racket/gui

(require graph)

(require racket/gui/base
         mrlib/graph
         "component.rkt"
         "ast.rkt")
(provide plot
         plot-compute)

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

  (define g (convert-graph comp vals))

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

(define (plot comp vals animate)
  (define index 0)
  (define hist (list->vector (reverse vals)))

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

  (define control-panel
    (new horizontal-panel%
         [parent toplevel]
         [alignment '(center center)]
         [stretchable-height #f]))

  (define play
    (new button%
         [parent control-panel]
         [label "Play"]
         [callback (lambda (button event)
                     (if (equal? (send play get-label) "Play")
                         (begin
                           (send play set-label "Stop")
                           (send timer start animate))
                         (begin
                           (send play set-label "Play")
                           (send timer stop))))]))

  (define prev
    (new button%
         [parent control-panel]
         [label "<"]
         [callback (lambda (button event)
                     (update -1))]))

  (define next
    (new button%
         [parent control-panel]
         [label ">"]
         [callback (lambda (button event)
                     (update 1))]))

  (define index-label
    (new message%
         [parent control-panel]
         [label ""]))

  (define (update dir)
    (set! index (modulo (+ index dir) (vector-length hist)))
    (send index-label set-label (~a (add1 index) "/" (vector-length hist)))
    (send board begin-edit-sequence)
    (plot-comp board comp
               (ast-tuple-state (vector-ref hist index))
               (ast-tuple-inactive (vector-ref hist index)))
    (send board end-edit-sequence))

  (define timer
    (new timer%
         [notify-callback (lambda () (update 1))]))

  (send toplevel show #t)
  (update 0))

(define (plot-compute comp inputs
                      #:memory [memory (make-immutable-hash)]
                      #:animate [animate 1000])
  (plot comp (ast-tuple-history (compute comp inputs #:memory memory)) animate))
