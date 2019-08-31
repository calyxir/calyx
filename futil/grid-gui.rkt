#lang racket/gui

(require racket/gui/base
         graph
         mrlib/graph
         racket/set
         racket/hash
         racket/match
         "component.rkt"
         "port.rkt"
         "futil-prims.rkt")

(define (round-up-to n to)
  (+ n
     (-
      to
      (modulo n to))))

(define (round-to-closest n to)
  (let ([n (round n)]
        [to (round to)])
    (if (< (modulo n to) (/ to 2))
        (- n
           (modulo n to))
        (+ n
           (- to
              (modulo n to))))))

(define node-snip-class
  (make-object
   (class snip-class%
     (super-new)
     (send this set-classname "node-snip"))))

(send (get-the-snip-class-list) add node-snip-class)

(define node-snip%
  (class snip%
    (init-field value
                active
                grid-width
                inputs
                outputs)
    (super-new)
    (send this set-snipclass node-snip-class)

    (define num-inputs (length inputs))
    (define num-outputs (length outputs))

    (define char-width 12)
    (define value-width
      (round-up-to
       (+ 4
          (* char-width (string-length (~a value))))
       grid-width))

    (define value-height
      (round-up-to
       (* grid-width (+ (max num-inputs num-outputs) 0))
       grid-width))

    (define/public (get-width) value-width)
    (define/public (get-height) value-height)

    (define/override (get-extent dc x y width height descent space lspace rspace)
      (when width (set-box! width value-width))
      (when height (set-box! height value-height))
      (when descent (set-box! descent 0.0))
      (when space (set-box! space 0.0))
      (when lspace (set-box! lspace 0.0))
      (when rspace (set-box! rspace 0.0)))

    (define/override (draw dc x y . other)
      (define font
        (send the-font-list find-or-create-font 10 'modern 'normal 'normal))
      (send dc set-font font)
      (define label (~a value))
      (cond
        [(not active)
         (send* dc
           (set-pen "blue" 1 'solid)
           (set-brush "blue" 'bdiagonal-hatch)
           (draw-rectangle x y value-width 20)
           (set-text-foreground "red")
           (draw-text label
                      (+ x (/ value-width 4))
                      (+ y (/ value-height 4))))]
        [active
         (send* dc
           (set-text-background "black")
           (draw-text label
                      (+ x (/ value-width 4))
                      (+ y (/ value-height 4)))
           (set-pen "black" 1 'solid)
           (set-brush "white" 'solid)
           (draw-rectangle (+ 2 x) (+ 2 y) (- value-width 4) (- value-height 4))
           (set-text-foreground "black")
           (draw-text label
                      (+ x (/ value-width 4))
                      (+ y (/ value-height 4))))])
      (for ([in (in-range num-inputs)])
        (define in-y (+ y grid-width (* in grid-width) -2))
        (send* dc
          (set-pen "blue" 1 'solid)
          (set-brush "blue" 'solid)
          (draw-ellipse (- x 0) (- in-y 2) 4 4)))
      (for ([out (in-range num-outputs)])
        (define out-y (+ y grid-width (* out grid-width) -2))
        (send* dc
          (set-pen "blue" 1 'solid)
          (set-brush "blue" 'solid)
          (draw-ellipse (+ x value-width -4) out-y 4 4))))

    (define/public (get-in-port-pos name x y)
      (values
       (round (/ x grid-width))
       (+ (round (/ y grid-width))
          (index-of (map port-name inputs) name)
          1)))

    (define/public (get-out-port-pos name x y)
      (values
       (round (/ (+ x value-width) grid-width))
       (+ (round (/ y grid-width))
          (index-of (map port-name outputs) name)
          1)))
    ))

(define node%
  (graph-snip-mixin node-snip%))

(define graph-board%
  (class pasteboard%
    (inherit get-admin)
    (init-field [(comp component)] grid-width)
    (super-new)

    ;; structure representing a rectangle and associated methods
    (struct rectangle (x1 y1 x2 y2)
      #:transparent)

    ;; redraw the given rectangle
    (define/private (refresh-rectangle rect)
      (match-define (rectangle x1 y1 x2 y2) rect)
      (send this refresh x1 y1 x2 y2
            'no-caret #f))

    ;; (in-rectangle? x y rect) is true when [x] [y] is contained with in [rect]
    (define/private (in-rectangle? x y rect)
      (and (<= (min (rectangle-x1 rect)
                    (rectangle-x2 rect))
               x
               (max (rectangle-x1 rect)
                    (rectangle-x2 rect)))
           (<= (min (rectangle-y1 rect)
                    (rectangle-y2 rect))
               y
               (max (rectangle-y1 rect)
                    (rectangle-y2 rect)))))


    ;; (rectangle-mult rect v) applies f to each coordinate in rect
    (define/private (rectangle-apply rect f)
      (match-define (rectangle x1 y1 x2 y2) rect)
      (rectangle (f x1) (f y1) (f x2) (f y2)))

    (struct path-render-data (route
                              bbox
                              cache
                              cache-offx
                              cache-offy))

    ;; structure representing a path and associated methods
    (struct path (;; name of starting port in the form: (mod . port)
                  start
                  ;; name of ending port in the form: (mod . port)
                  end
                  ;; label on the wire, #f signifies no value
                  [label #:mutable]
                  ;; how to draw this path. options are 'inactive, 'active
                  [style #:mutable]
                  ;; flag signifying that this path should be highlighted
                  [hover #:mutable #:auto]
                  ;; render data
                  [data #:mutable #:auto])
      #:transparent)



    (define grid-cache #f)

    ;; initialize nodes
    (send this begin-edit-sequence)
    (define nodes
      ;; make a hash from node names to snip objects
      (make-hash
       (map (lambda (vert)
              (let* ([sub (get-submod! comp vert)]
                     [;; construct new node
                      node (new node%
                                [inputs (component-ins sub)]
                                [outputs (component-outs sub)]
                                [value vert]
                                [active #t]
                                [grid-width grid-width])])
                ;; insert the new node into the board
                (send this insert node 0 0)
                ;; return a pair mapping the vertex name to the new node
                `(,vert . ,node)))
            ;; map over all submods in the component
            (get-vertices (convert-graph comp)))))

    ;; construct a hash mapping edges to paths
    (define edge-path-hash
      (make-hash
       (map (lambda (edge)
              (let-values ([(start end) (values (car edge) (cadr edge))])
                `(,edge . ,(path start end "" 'inactive))))
            ;; map over edges in the graph
            (get-edges (component-graph comp)))))

    ;; construct a hash mapping node names to paths connected to it
    (define node-path-hash
      (let ([;; first construct a hash mapping node names to the empty list
             empty-hash
             (make-hash
              (map (lambda (node) `(,node . ()))
                   (get-vertices (convert-graph comp))))])
        ;; then map over edges adding each path to the corresponding node list
        (hash-for-each
         edge-path-hash
         (lambda (edge path)
           (define-values (start-mod end-mod)
             (values (caar edge) (caadr edge)))
           (hash-update! empty-hash
                         start-mod
                         (lambda (old)
                           (cons path old)))
           (hash-update! empty-hash
                         end-mod
                         (lambda (old)
                           (cons path old)))))
        empty-hash))
    (layout)
    (send this end-edit-sequence)

    ;; overriding the on-paint method to add our own drawing code
    (define/override (on-paint before? dc topx topy width height . other)
      ;; when we are drawing before the snips
      (when before?
        (draw-grid dc width height)
        (draw-wires dc))

      ;; call the super method
      (super on-paint before? dc topx topy width height . other))

    ;; called when the size of the window changes
    (define/augment (on-display-size)
      (set! grid-cache #f))

    ;; creates an empty bitmap with size given by rect
    ;; or if rect is [#f] then sized by the window
    (define/private (empty-cache [rect #f])
      (define admin (get-admin))
      (define (fullscreen-cache)
        (if admin
            (let ([xb (box 0)]
                  [yb (box 0)]
                  [wb (box 0)]
                  [hb (box 0)])
              (send admin get-max-view xb yb wb hb)
              (make-bitmap (unbox wb) (unbox hb)))
            #f))
      (if rect
          (let* ([rect
                 (rectangle-apply rect (compose inexact->exact))]
                 [width (- (rectangle-x2 rect) (rectangle-x1 rect))]
                 [height (- (rectangle-y2 rect) (rectangle-y1 rect))])
            (if (or (zero? width) (zero? height))
                (fullscreen-cache)
                (make-bitmap width height)))
          (fullscreen-cache)))

    ;; renders the grid of dots to the screen
    (define/private (draw-grid dc width height)
      (unless grid-cache
        (set! grid-cache (empty-cache))
        (define grid-dc (make-object bitmap-dc% grid-cache))
        (for ([x (in-range grid-width width grid-width)])
          (for ([y (in-range grid-width height grid-width)])
            (send* grid-dc
              (set-pen "gray" 1 'solid)
              (set-brush "gray" 'solid)
              (draw-ellipse (- x 1) (- y 1) 2 2))))
        (send grid-dc set-bitmap #f))

      (send dc draw-bitmap grid-cache 0 0))

    (define/private (draw-wires dc)
      (hash-for-each
       edge-path-hash
       (lambda (name p)
         (unless (path-data p)
           (recreate-path-render-data p))

         (match-define (path start end lbl style hover data) p)
         (match-let
             ([(path-render-data route bbox
                                 cache cache-offx cache-offy)
               data])
           (send dc draw-bitmap cache cache-offx cache-offy)))))

    (define/private (recreate-path-render-data p)
      (match-define (path start end label style hover data)
        p)
      (define-values (route bbox) (make-route start end))
      (define cache (empty-cache
                     ;; (rectangle-apply bbox
                     ;;                  (lambda (x) (* x grid-width)))
                     ))
      (define pdc (make-object bitmap-dc% cache))
      (define (set-style dc)
        (if hover
            (send dc set-pen "red" 2 'solid)
            (match style
              ['inactive
               (send dc set-pen "black" 2 'solid)]
              ['active
               (send dc set-pen "green" 2 'solid)])))
      (draw-route pdc route set-style)
      (define new-data
        (path-render-data
         route
         (rectangle-apply bbox (lambda (x) (* x grid-width)))
         cache
         0
         0))
      (set-path-data! p new-data)
      )

    ;; (define/augment (on-move-to snip x y dragging?)
    ;;   (hash-for-each
    ;;    wires
    ;;    (lambda (name pth)
    ;;      (invalidate-path name pth)
    ;;      )
    ;;    )
    ;;   )

    (define/augment (after-move-to snip x y dragging?)
      (when (not dragging?)
        (send this begin-edit-sequence)
        (send this move-to snip
              (round-to-closest x grid-width)
              (round-to-closest y grid-width))
        (for-each
         (lambda (p)
           (when (path-data p)
             (refresh-rectangle (path-render-data-bbox (path-data p)))
             (set-path-data! p #f)))
         (hash-ref node-path-hash (get-field value snip)))
        (send this end-edit-sequence)))

    ;; listener for mouse events (and probably other events)
    (define/override (on-event evt)
      (cond
        ;; [(send evt leaving?)
        ;;  ]
        [(or (send evt entering?)
             (send evt moving?))
         (let* ([ex (send evt get-x)]
                [ey (send evt get-y)]
                [hover (get-nodes-at ex ey)])
           (void)
           ;; (hash-for-each
           ;;  node-path-hash
           ;;  (lambda (name plst)
           ;;    (for-each
           ;;     (lambda (p)
           ;;       ;; (set-path-hover! p (member name hover))
           ;;       ;; (define data (path-data p))
           ;;       ;; (when data
           ;;       ;;   (set-path-data! p #f)
           ;;       ;;   (refresh-rectangle (path-render-data-bbox data))
           ;;       ;;   )
           ;;       (define data (path-data p))
           ;;       (if (member name hover)
           ;;           (unless (path-hover p)
           ;;             (set-path-hover! p #t)
           ;;             (when data
           ;;               (set-path-data! p #f)
           ;;               (refresh-rectangle (path-render-data-bbox data))))
           ;;           (when (path-hover p)
           ;;             (set-path-hover! p #f)
           ;;             (when data
           ;;               (set-path-data! p #f)
           ;;               (refresh-rectangle (path-render-data-bbox data)))))
           ;;       )
           ;;     plst)))
           )]
        [else (void)])
      (super on-event evt))

    ;; function that returns the position rectangle of a node
    (define/private (get-location node)
      (let ([x (box 0)]
            [y (box 0)]
            [w (send node get-width)]
            [h (send node get-height)])
        (send this get-snip-location node x y)
        (rectangle (unbox x) (unbox y)
                   (+ (unbox x) w)
                   (+ (unbox y) h))))

    ;; get nodes at a given position
    (define/private (get-nodes-at ex ey)
      (filter-map
       (lambda (pair)
         (define-values (key node) (values (car pair) (cdr pair)))
         (if (in-rectangle? ex ey (get-location node))
             key
             #f))
       (hash->list nodes)))

    ;; function responsible for positioning all the snips
    (define/private (layout)
      (dot-positioning this "dot"))

    ;; render a given route to the provided drawing context
    (define/private (draw-route dc route set-style)
      ;; convert virtual coords of route to real coords
      (define realpoints
        (map (lambda (x)
               (cons (* grid-width (car x))
                     (* grid-width (cdr x))))
             route))

      ;; get all the x coords
      (define real-xs
        (map (lambda (p)
               (car p))
             realpoints))

      ;; get all the y coords
      (define real-ys
        (map (lambda (p)
               (cdr p))
             realpoints))

      (set-style dc)
      (send dc draw-lines realpoints))


    ;; given a start port and end port, create a route and a bounding box
    (define/private (make-route start end)
      (define start-node (hash-ref nodes (car start)))
      (define-values (start-vx start-vy)
        (let ([rect (get-location start-node)])
          (send start-node get-out-port-pos
                (cdr start)
                (rectangle-x1 rect)
                (rectangle-y1 rect))))

      (define end-node (hash-ref nodes (car end)))
      (define-values (end-vx end-vy)
        (let ([rect (get-location end-node)])
          (send end-node get-in-port-pos
                (cdr end)
                (rectangle-x1 rect)
                (rectangle-y1 rect))))

      (define path
        `((,start-vx . ,start-vy)
          (,(add1 start-vx) . ,start-vy)
          (,(sub1 end-vx) . ,end-vy)
          (,end-vx . ,end-vy)))

      (define bbox
        (let* ([xs (map car path)]
               [ys (map cdr path)])
          (rectangle (apply min xs) (apply min ys)
                     (apply max xs) (apply max ys))))

      (values path bbox))))

(define (show comp #:grid-width [grid-width 20])
  (define toplevel
    (new frame%
         [label "Grid"]
         [width (* 50 11)]
         [height (* 50 10)]))

  (define board (new graph-board%
                     [component comp]
                     [grid-width grid-width]))

  (define canvas
    (new editor-canvas%
         [parent toplevel]
         [style '(auto-vscroll auto-hscroll)]
         [horizontal-inset 0]
         [vertical-inset 0]
         [editor board]))

  (send toplevel show #t))

;; (define g
;;   (matrix-graph [[0 3 8 #f -4]
;;                  [#f 0 #f 1 7]
;;                  [#f 4 0 #f #f]
;;                  [2 #f -5 0 #f]
;;                  [#f #f #f 6 0]]))
;; (get-neighbors g 0)
(require futil)

(define/module test ((a : 32) (b : 32)) ((out : 32))
  ([add = new comp/add]
   [a -> add @ left]
   [b -> add @ right]
   [add @ out -> out])
  [])
(show (test))












