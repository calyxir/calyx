(define/namespace stdlib
  (define/component and3way
    ((port a 32) (port b 32) (port c 32))
    ((port out 32))
    ((new-std en (const 1))
     (-> (@ en out) (@ this out)))
    (ifen (@ this a)
          (ifen (@ this b)
                (ifen (@ this c)
                      (enable en out)
                      (empty))
                (empty))
          (empty)))

  (define/component comp/iterator
    ((port start 32) (port incr 32) (port end 32) (port en 32))
    ((port out 32) (port stop 32))
    ((new-std incr-reg (comp/reg))
     (new-std end-reg (comp/reg))
     (new-std add (comp/add))
     (new-std cmp (comp/trunc-sub))

     (new ins-and and3way)
     (-> (@ this start) (@ ins-and a))
     (-> (@ this incr) (@ ins-and b))
     (-> (@ this end) (@ ins-and c))

     (-> (@ this incr) (@ incr-reg in))
     (-> (@ this end) (@ end-reg in))

     (new-std val-reg (comp/res-reg))
     (new-std res-vel (const 1))
     (-> (@ res-val out) (@ val-reg res))

     (new-std add0 (const 0))
     (-> (@ add0 out) (@ add right))
     (-> (@ this start) (@ add left))
     (-> (@ incr-reg out) (@ add right))
     (-> (@ add out) (@ val-reg in))
     (-> (@ val-reg out) (@ add left))
     (-> (@ add out) (@ this out))
     (-> (@ end-reg out) (@ cmp left))
     (-> (@ add out) (@ cmp right))
     (-> (@ cmp out) (@ this stop)))
    (seq
     (enable start incr end ins-and)
     (ifen (@ this en)
           (ifen (@ ins-and out)
                 (seq
                  (enable res-val val-reg)
                  (enable start incr end incr-reg end-reg)
                  (disable incr incr-reg end res-val))
                 (disable add-zero start incr end res-val))
           (disable start incr incr-reg end res-val)))))
