#[cfg(test)]
mod tests {
    use crate::utils;
    use egglog::EGraph;
    use main_error::MainError;
    // Thanks to www.github.com/egraphs-good/eggcc for inspiring this test suite.
    pub type Result = std::result::Result<(), MainError>;

    // fn test_calyx(actual: &str, expected: &str) {}

    /// Tests egglog input with egglog checks, e.g.,
    ///
    /// test_egglog(
    ///     r#"
    ///     (let A 42)
    ///     (let B 42)
    ///     "#,
    ///     "(check (= A B))",
    ///     &[utils::RewriteRule::CalyxControl],
    /// )
    fn test_egglog(
        prologue: &str,
        check: &str,
        rules: &[utils::RewriteRule],
    ) -> Result {
        let mut s: String = String::new();
        for rule in rules {
            s.push_str(utils::read_from(*rule)?.as_str());
        }
        s.push_str(prologue);
        s.push_str(utils::run_schedule(&rules)?.as_str());
        s.push_str(check);

        let result = EGraph::default().parse_and_run_program(&s).map(|lines| {
            for line in lines {
                println!("{}", line);
            }
        });
        if result.is_err() {
            println!("{:?}", result);
        }
        Ok(result?)
    }

    #[test]
    fn test_identity() -> Result {
        test_egglog(
            r#"
            (let c1 (CellSet (set-of (Cell "a"))))
            (let c2 (CellSet (set-of (Cell "a"))))
            "#,
            r#"(check (= c1 c2))"#,
            &[utils::RewriteRule::CalyxControl],
        )
    }

    #[test]
    fn test_list_length() -> Result {
        test_egglog(
            r#"
            (let A (Enable (Group "A" (CellSet (set-empty))) (Attributes (map-empty))))
            (let B (Enable (Group "B" (CellSet (set-empty))) (Attributes (map-empty))))
            (let C (Enable (Group "C" (CellSet (set-empty))) (Attributes (map-empty))))
            (let D (Enable (Group "D" (CellSet (set-empty))) (Attributes (map-empty))))
            (list-length-demand (Nil))
            (list-length-demand (Cons D (Nil)))
            (list-length-demand (Cons C (Cons D (Nil))))
            (list-length-demand (Cons B (Cons C (Cons D (Nil)))))
            (list-length-demand (Cons A (Cons B (Cons C (Cons D (Nil))))))
            "#,
            r#"
            (check (= (list-length (Nil)) 0))
            (check (= (list-length (Cons D (Nil))) 1))
            (check (= (list-length (Cons C (Cons D (Nil)))) 2))
            (check (= (list-length (Cons B (Cons C (Cons D (Nil))))) 3))
            (check (= (list-length (Cons A (Cons B (Cons C (Cons D (Nil)))))) 4))
            "#,
            &[utils::RewriteRule::CalyxControl],
        )
    }

    #[test]
    fn test_list_slice() -> Result {
        test_egglog(
            r#"
            (let A (Enable (Group "A" (CellSet (set-empty))) (Attributes (map-empty))))
            (let B (Enable (Group "B" (CellSet (set-empty))) (Attributes (map-empty))))
            (let C (Enable (Group "C" (CellSet (set-empty))) (Attributes (map-empty))))
            (let D (Enable (Group "D" (CellSet (set-empty))) (Attributes (map-empty))))
            (let xs (Cons A (Cons B (Cons C (Cons D (Nil))))))
            (list-length-demand xs)
            (_sliceB xs 1) (_sliceE xs 2)
            (list-slice xs 1 2) (list-slice xs 1 3) (list-slice xs 0 1)
            "#,
            r#"
            (check (= (_sliceB xs 1) (Cons B (Cons C (Cons D (Nil))))))
            (check (= (_sliceE xs 2) (Cons A (Cons B (Nil)))))
            (check (= (list-slice xs 1 2) (Cons B (Nil))))
            (check (= (list-slice xs 1 3) (Cons B (Cons C (Nil)))))
            (check (= (list-slice xs 0 1) (Cons A (Nil))))
            "#,
            &[utils::RewriteRule::CalyxControl],
        )
    }

    #[test]
    fn test_list_latency() -> Result {
        test_egglog(
            r#"
            (let m1 (map-insert (map-empty) "promotable" 1))
            (let m2 (map-insert (map-empty) "promotable" 2))
            (let g1 (Group "A" (CellSet (set-empty))))
            (let g2 (Group "B" (CellSet (set-empty))))

            (let ys (Cons (Enable g1 (Attributes (map-empty))) (Cons (Enable g2 (Attributes (map-empty))) (Nil))))
            (let S (Seq (Attributes (map-insert (map-empty) "static" 3)) ys))
            
            (let X (Enable g1 (Attributes m1)))
            (let Y (Enable g2 (Attributes m2)))
            (let xs (Cons X (Cons S (Cons Y (Nil)))))

            (list-length-demand xs) (list-length-demand ys)
            (sum-latency-demand xs) (sum-latency-demand ys)
            (max-latency-demand xs) (max-latency-demand ys)
            "#,
            r#"
            (check (= (max-latency xs) 3))
            (check (= (sum-latency xs) 6)) ; 1 + 3 + 2
            "#,
            &[utils::RewriteRule::CalyxControl],
        )
    }

    #[test]
    fn test_control_before() -> Result {
        test_egglog(
            r#"
            (let A (Enable (Group "A" (CellSet (set-empty))) (Attributes (map-empty))))
            (let B (Enable (Group "B" (CellSet (set-empty))) (Attributes (map-empty))))
            (let C (Enable (Group "C" (CellSet (set-empty))) (Attributes (map-empty))))
            (let D (Enable (Group "D" (CellSet (set-empty))) (Attributes (map-empty))))
            (let xs (control-before D (Seq (Attributes (map-empty)) (Cons A (Cons B (Cons C (Cons D (Nil))))))))
            (let ys (control-before C (Seq (Attributes (map-empty)) (Cons A (Cons B (Cons C (Cons D (Nil))))))))
            (let zs (control-before B (Seq (Attributes (map-empty)) (Cons A (Cons B (Cons C (Cons D (Nil))))))))
            "#,
            r#"
            (check (= xs (Cons A (Cons B (Cons C (Nil))))))
            (check (= ys (Cons A (Cons B (Nil)))))
            (check (= zs (Cons A (Nil))))
            "#,
            &[utils::RewriteRule::CalyxControl],
        )
    }

    #[test]
    fn test_exclusive() -> Result {
        test_egglog(
            r#"
            (let CS1 (CellSet (set-of (Cell "a"))))
            (let CS2 (CellSet (set-of (Cell "b"))))
            (let A0 (Enable (Group "A" CS1) (Attributes (map-empty))))
            (let B0 (Enable (Group "B" CS2) (Attributes (map-empty))))
        "#,
            r#"
            (check (= (exclusive A0 B0) true))
            (check (= (exclusive A0 A0) false))
        "#,
            &[utils::RewriteRule::CalyxControl],
        )
    }

    #[test]
    fn test_fan_out() -> Result {
        test_egglog(
            r#"
            (let A (Enable (Group "A" (CellSet (set-empty))) (Attributes (map-empty))))
            (let B (Enable (Group "B" (CellSet (set-empty))) (Attributes (map-empty))))
            (let C (Enable (Group "C" (CellSet (set-empty))) (Attributes (map-empty))))
            (let D (Enable (Group "D" (CellSet (set-empty))) (Attributes (map-empty))))
            (let xs (Cons A (Cons B (Cons C (Cons D (Nil))))))
            (list-length-demand xs)
            (let P (Par (Attributes (map-empty)) xs))
        "#,
            r#"
            (check (= FAN-OUT 2)) ; ...this test will fail otherwise.
            (check (= P 
                (Par (Attributes (map-empty))
                (Cons (Par (Attributes (map-empty)) (Cons A (Cons B (Nil))))
                (Cons (Par (Attributes (map-empty)) (Cons C (Cons D (Nil))))
                    (Nil)))
            )))
        "#,
            &[utils::RewriteRule::CalyxControl],
        )
    }

    #[test]
    fn test_par_to_seq() -> Result {
        test_egglog(
            r#"
            (let g1 (Group "A" (CellSet (set-empty))))
            (let g2 (Group "B" (CellSet (set-empty))))
            (let P (Par (Attributes (map-empty)) 
                (Cons (Enable g1 (Attributes (map-insert (map-empty) "promotable" 1011)))
                (Cons (Enable g2 (Attributes (map-insert (map-empty) "promotable" 5)))
                    (Nil)))))
            (let S (Seq (Attributes (map-empty)) 
                (Cons (Enable g1 (Attributes (map-insert (map-empty) "promotable" 1011)))
                (Cons (Enable g2 (Attributes (map-insert (map-empty) "promotable" 5)))
                    (Nil)))))
        "#,
            r#"
            (check (= PAR-TO-SEQ 1000)) ; ... this test will fail otherwise.
            (check (= P S))
        "#,
            &[utils::RewriteRule::CalyxControl],
        )
    }

    // TODO(cgyurgyik): Figure out why this is no longer working. Getting an illegal merge failure...
    // maybe it needs to be rewritten as a rule?
    #[ignore]
    #[test]
    fn test_split_seq() -> Result {
        test_egglog(
            r#"
            (let A (Enable (Group "A" (CellSet (set-empty))) (Attributes (map-empty))))
            (let B (Enable (Group "B" (CellSet (set-empty))) (Attributes (map-empty))))
            (let C (Enable (Group "C" (CellSet (set-empty))) (Attributes (map-empty))))
            (let D (Enable (Group "D" (CellSet (set-empty))) (Attributes (map-empty))))
            (let xss (Cons A (Cons B (Cons C (Cons D (Cons A (Cons B (Cons C (Cons D (Nil))))))))))
            (let xs (Cons A (Cons B (Cons C (Cons D (Nil))))))
            (let S-before (Seq (Attributes (map-empty)) xss))
            (let S-after (Seq (Attributes (map-empty))
                (Cons (Seq (Attributes (map-insert (map-empty) "new_fsm" 1)) xs)
                (Cons (Seq (Attributes (map-insert (map-empty) "new_fsm" 1)) xs)
                    (Nil)))))
            (list-length-demand xss)
            (list-length-demand xs)
            (list-slice xss 0 4)
            (list-slice xss 4 8)
        "#,
            r#"
            (check (= SPLIT-SEQ 8)) ; ... this test will fail otherwise.
            (check (= S-before S-after))
        "#,
            &[utils::RewriteRule::CalyxControl],
        )
    }
}
