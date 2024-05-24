#[cfg(test)]
mod unit_tests {
    use crate::egg::{
        egg_optimize::EggOptimizeBackend,
        utils::{self, RewriteRule},
    };
    use egglog::EGraph;
    use itertools::Itertools;
    use std::{fs, path::Path};
    use strum::IntoEnumIterator;

    /// Retrieve the egglog rules for the unit test.
    pub fn egglog_rules(
        rules: &[RewriteRule],
    ) -> calyx_utils::CalyxResult<String> {
        let mut s = String::new();
        let path = Path::new("src/egg/ruleset/");
        for rule in rules {
            s.push_str(&fs::read_to_string(
                path.join(rule.to_string()).with_extension("egg"),
            )?);
        }
        Ok(s)
    }

    /// Tests egglog input with egglog checks, e.g.,
    ///
    /// test_egglog(
    ///     r#"
    ///     (let A 42)
    ///     (let B 42)
    ///     "#,
    ///     "(check (= A B))",
    ///     &utils::RewriteRule::iter().collect_vec()
    /// )
    fn test_egglog_internal(
        prologue: &str,
        check: &str,
        rules: &[utils::RewriteRule],
        display: bool,
    ) -> utils::Result {
        let mut s: String = String::new();

        s.push_str(egglog_rules(rules).unwrap().as_str());
        s.push_str(prologue);
        s.push_str(
            EggOptimizeBackend::egglog_schedule(&rules)
                .unwrap()
                .as_str(),
        );
        s.push_str(check);

        let mut egraph = EGraph::default();
        let result = egraph.parse_and_run_program(&s).map(|lines| {
            for line in lines {
                println!("{}", line);
            }
        });

        if display {
            let serialized = egraph.serialize_for_graphviz(true, 100, 100);
            let file = tempfile::NamedTempFile::new()?;
            let path = file.into_temp_path().with_extension("svg");
            serialized.to_svg_file(path.clone())?;
            std::process::Command::new("open")
                .arg(path.to_str().unwrap())
                .output()?;
        }

        if result.is_err() {
            println!("{:?}", result);
        }
        Ok(result?)
    }

    fn test_egglog(
        prologue: &str,
        check: &str,
        rules: &[utils::RewriteRule],
    ) -> utils::Result {
        test_egglog_internal(prologue, check, rules, false)
    }

    fn test_egglog_debug(
        prologue: &str,
        check: &str,
        rules: &[utils::RewriteRule],
    ) -> utils::Result {
        test_egglog_internal(prologue, check, rules, true)
    }

    #[test]
    fn test_identity() -> utils::Result {
        test_egglog(
            r#"
            (let c1 (CellSet (set-of (Cell "a"))))
            (let c2 (CellSet (set-of (Cell "a"))))
            "#,
            r#"(check (= c1 c2))"#,
            &utils::RewriteRule::iter().collect_vec(),
        )
    }

    #[test]
    fn test_exclusivity() -> utils::Result {
        test_egglog(
            r#"
            (let c1 (CellSet (set-of (Cell "a"))))
            (let c2 (CellSet (set-of (Cell "b"))))
            (let c3 (CellSet (set-of (Cell "a") (Cell "b"))))
            (let c4 (CellSet (set-of (Cell "b") (Cell "c"))))

            (let A (Enable (Group "A" c1) (Attributes (map-empty))))
            (let B (Enable (Group "B" c2) (Attributes (map-empty))))
            (let C (Enable (Group "C" c3) (Attributes (map-empty))))
            (let D (Enable (Group "D" c4) (Attributes (map-empty))))
            (let S (Seq (Attributes (map-empty)) (Cons A (Nil))))
            (Nil)
            (Cons S (Nil))
            (Cons D (Nil))
            (Cons B (Nil))
            (Cons B (Cons D (Nil)))
            (Cons C (Cons D (Nil)))
            (Cons B (Cons C (Cons D (Nil))))
            (Cons A (Cons B (Cons C (Cons D (Nil)))))
            "#,
            r#"
            (check (= (exclusive A B) true))
            (check (= (exclusive A A) false))
            (check (= (exclusive A C) false))
            (check (= (exclusive A D) true))

            (check (= (exclusive-with-all A (Nil)) true))
            (check (= (exclusive-with-all A (Cons A (Nil))) false))
            (check (= (exclusive-with-all A (Cons B (Cons C (Cons D (Nil))))) false))
            (check (= (exclusive-with-all A (Cons D (Nil))) true))
            (check (= (exclusive-with-all A (Cons B (Nil))) true))
            (check (= (exclusive-with-all A (Cons B (Cons D (Nil)))) true))
            (check (= (exclusive-with-all A (Cons S (Nil))) false))

            "#,
            &utils::RewriteRule::iter().collect_vec(),
        )
    }

    #[ignore = "removed (temporarily)"]
    #[test]
    fn test_non_exclusive_set() -> utils::Result {
        test_egglog(
            r#"
            (let c1 (CellSet (set-of (Cell "a"))))
            (let c2 (CellSet (set-of (Cell "b"))))
            (let c3 (CellSet (set-of (Cell "a") (Cell "b"))))
            (let c4 (CellSet (set-of (Cell "b") (Cell "c"))))

            (let A (Enable (Group "A" c1) (Attributes (map-empty))))
            (let B (Enable (Group "B" c2) (Attributes (map-empty))))
            (let C (Enable (Group "C" c3) (Attributes (map-empty))))
            (let D (Enable (Group "D" c4) (Attributes (map-empty))))
            (let S (Seq (Attributes (map-empty)) (Cons A (Nil))))
            (let P (Par (Attributes (map-empty)) (Cons A (Nil))))
            (Nil)
            (Cons S (Nil))
            (Cons C (Cons P (Nil)))
            (Cons C (Cons S (Nil)))  
            (Cons C (Nil))
            (Cons B (Nil))
            (Cons A (Cons B (Cons C (Cons D (Nil)))))
            "#,
            r#"

            (check (= (nonexclusive-set A (Nil)) (ControlSet (set-empty))))
            (check (= (nonexclusive-set A (Cons B (Nil))) (ControlSet (set-empty))))
            (check (= (nonexclusive-set A (Cons C (Nil))) (ControlSet (set-of C))))
            (check (=
               (nonexclusive-set A (Cons A (Cons B (Cons C (Cons D (Nil))))))
               (ControlSet (set-of A C))
            ))

            (check (=
                (nonexclusive-set A (Cons C (Cons S (Nil))))
                (ControlSet (set-of C S))
             ))

            (check (=
                (nonexclusive-set A (Cons C (Cons P (Nil))))
                (ControlSet (set-of C P))
            ))
            "#,
            &utils::RewriteRule::iter().collect_vec(),
        )
    }

    #[test]
    fn test_list_length() -> utils::Result {
        test_egglog(
            r#"
            (let A (Enable (Group "A" (CellSet (set-empty))) (Attributes (map-empty))))
            (let B (Enable (Group "B" (CellSet (set-empty))) (Attributes (map-empty))))
            (let C (Enable (Group "C" (CellSet (set-empty))) (Attributes (map-empty))))
            (let D (Enable (Group "D" (CellSet (set-empty))) (Attributes (map-empty))))
            (let S (Seq (Attributes (map-empty)) (Cons A (Nil))))
            (Nil)
            (Cons D (Nil))
            (Cons C (Cons D (Nil)))
            (Cons B (Cons C (Cons D (Nil))))
            (Cons A (Cons B (Cons C (Cons D (Nil)))))

            (Cons A (Cons S (Nil)))
            "#,
            r#"
            (check (= (list-length (Nil)) 0))
            (check (= (list-length (Cons D (Nil))) 1))
            (check (= (list-length (Cons C (Cons D (Nil)))) 2))
            (check (= (list-length (Cons B (Cons C (Cons D (Nil))))) 3))
            (check (= (list-length (Cons A (Cons B (Cons C (Cons D (Nil)))))) 4))

            (check (= (list-length (Cons A (Cons S (Nil)))) 2))
            "#,
            &utils::RewriteRule::iter().collect_vec(),
        )
    }

    #[test]
    fn test_list_slice() -> utils::Result {
        test_egglog(
            r#"
            (let A (Enable (Group "A" (CellSet (set-empty))) (Attributes (map-empty))))
            (let B (Enable (Group "B" (CellSet (set-empty))) (Attributes (map-empty))))
            (let C (Enable (Group "C" (CellSet (set-empty))) (Attributes (map-empty))))
            (let D (Enable (Group "D" (CellSet (set-empty))) (Attributes (map-empty))))
            (let xs (Cons A (Cons B (Cons C (Cons D (Nil))))))
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
            &utils::RewriteRule::iter().collect_vec(),
        )
    }

    #[test]
    fn test_list_slice2() -> utils::Result {
        test_egglog(
            r#"
            (let A (Enable (Group "A" (CellSet (set-empty))) (Attributes (map-empty))))
            (let B (Enable (Group "B" (CellSet (set-empty))) (Attributes (map-empty))))
            (let C (Enable (Group "C" (CellSet (set-empty))) (Attributes (map-empty))))
            (let D (Enable (Group "D" (CellSet (set-empty))) (Attributes (map-empty))))
            (let E (Enable (Group "E" (CellSet (set-empty))) (Attributes (map-empty))))
            (let F (Enable (Group "F" (CellSet (set-empty))) (Attributes (map-empty))))
            (let G (Enable (Group "G" (CellSet (set-empty))) (Attributes (map-empty))))
            (let H (Enable (Group "H" (CellSet (set-empty))) (Attributes (map-empty))))
            (let xs (Cons A (Cons B (Cons C (Cons D (Cons E (Cons F (Cons G (Cons H (Nil))))))))))
            (let s1 (list-slice xs 0 4))
            (let s2 (list-slice xs 4 8))
            "#,
            r#"
            (check (= s1 l1))
            (check (= s2 l2))
            "#,
            &utils::RewriteRule::iter().collect_vec(),
        )
    }

    #[test]
    fn test_seq_sum_latency() -> utils::Result {
        // If a "static" attribute is available, use it when performing latency analysis.
        // Otherwise, sum the latencies of the construct's groups.
        test_egglog(
            r#"
            (let m1 (map-insert (map-empty) "promotable" 1))
            (let m2 (map-insert (map-empty) "promotable" 2))
            (let g1 (Group "A" (CellSet (set-empty))))
            (let g2 (Group "B" (CellSet (set-empty))))

            (let ys (Cons (Enable g1 (Attributes (map-empty))) (Cons (Enable g2 (Attributes (map-empty))) (Nil))))
            ; @static(3) seq { ... }
            (let S1 (Seq (Attributes (map-insert (map-empty) "static" 3)) ys))
            (let X (Enable g1 (Attributes m1)))
            (let Y (Enable g2 (Attributes m2)))
            ; @promotable(1) A; @static(3) seq { ... }; @promotable(2) B;
            (let xs (Cons X (Cons S1 (Cons Y (Nil)))))
            
            ; @static(10) seq { ... }
            (let S2 (Seq (Attributes (map-insert (map-empty) "static" 10)) ys))
            (let xss (Cons S2 xs))

            ; seq { @promotable(1) A; @promotable(2) B; }
            (let zs (Cons (Enable g1 (Attributes m1)) (Cons (Enable g2 (Attributes m2)) (Nil))))
            (let S3 (Seq (Attributes (map-empty)) zs))
            (let xsss (Cons S3 xss))
            "#,
            r#"
            (check (= (sum-latency xs) 6)) ; 1 + 3 + 2
            (check (= (sum-latency xss) 16)) ; 1 + 3 + 2 + 10
            "#,
            &utils::RewriteRule::iter().collect_vec(),
        )
    }

    #[test]
    fn test_seq_max_latency() -> utils::Result {
        // If a "static" attribute is available, use it when performing latency analysis.
        // Otherwise, select a maximum from the latencies of the construct's groups.
        test_egglog(
            r#"
            (let m1 (map-insert (map-empty) "promotable" 1))
            (let m2 (map-insert (map-empty) "promotable" 2))
            (let g1 (Group "A" (CellSet (set-empty))))
            (let g2 (Group "B" (CellSet (set-empty))))

            (let ys (Cons (Enable g1 (Attributes (map-empty))) (Cons (Enable g2 (Attributes (map-empty))) (Nil))))
            ; @static(3) seq { ... }
            (let S1 (Seq (Attributes (map-insert (map-empty) "static" 3)) ys))
            (let X (Enable g1 (Attributes m1)))
            (let Y (Enable g2 (Attributes m2)))
            ; @promotable(1) A; @static(3) seq { ... }; @promotable(2) B;
            (let xs (Cons X (Cons S1 (Cons Y (Nil)))))
            
            ; @static(10) seq { ... }
            (let S2 (Seq (Attributes (map-insert (map-empty) "static" 10)) ys))
            (let xss (Cons S2 xs))

            ; seq { @promotable(1) A; @promotable(2) B; }
            (let zs (Cons (Enable g1 (Attributes m1)) (Cons (Enable g2 (Attributes m2)) (Nil))))
            (let S3 (Seq (Attributes (map-empty)) zs))
            (let xsss (Cons S3 xss))
            "#,
            r#"
            (check (= (max-latency xs) 3)) ; max(1, 3, 2)
            (check (= (max-latency xss) 10)) ; max(1, 3, 2, 10)
            (check (= (max-latency xsss) 10)) ; max(1, 3, 2, 10, 3)
            "#,
            &utils::RewriteRule::iter().collect_vec(),
        )
    }

    #[test]
    fn test_par_max_latency() -> utils::Result {
        // If a "static" attribute is available, use it when performing latency analysis.
        // Otherwise, select a maximum from the latencies of the construct's groups.
        test_egglog(
            r#"
            (let m1 (map-insert (map-empty) "promotable" 1))
            (let m2 (map-insert (map-empty) "promotable" 2))
            (let g1 (Group "A" (CellSet (set-empty))))
            (let g2 (Group "B" (CellSet (set-empty))))

            (let ys (Cons (Enable g1 (Attributes (map-empty))) (Cons (Enable g2 (Attributes (map-empty))) (Nil))))
            ; @static(3) par { ... }
            (let S1 (Par (Attributes (map-insert (map-empty) "static" 3)) ys))
            (let X (Enable g1 (Attributes m1)))
            (let Y (Enable g2 (Attributes m2)))
            ; @promotable(1) A; @static(3) par { ... }; @promotable(2) B;
            (let xs (Cons X (Cons S1 (Cons Y (Nil)))))
            
            ; @static(10) par { ... }
            (let S2 (Par (Attributes (map-insert (map-empty) "static" 10)) ys))
            (let xss (Cons S2 xs))

            ; par { @promotable(1) A; @promotable(2) B; }
            (let zs (Cons (Enable g1 (Attributes m1)) (Cons (Enable g2 (Attributes m2)) (Nil))))
            (let S3 (Par (Attributes (map-empty)) zs))
            (let xsss (Cons S3 xss))
            "#,
            r#"
            (check (= (max-latency xs) 3)) ; max(1, 3, 2)
            (check (= (max-latency xss) 10)) ; max(1, 3, 2, 10)
            (check (= (max-latency xsss) 10)) ; max(1, 3, 2, 10, 3)
            "#,
            &utils::RewriteRule::iter().collect_vec(),
        )
    }

    #[test]
    fn test_par_sum_latency() -> utils::Result {
        // If a "static" attribute is available, use it when performing latency analysis.
        // Otherwise, sum the latencies of the construct's groups.
        test_egglog(
            r#"
            (let m1 (map-insert (map-empty) "promotable" 1))
            (let m2 (map-insert (map-empty) "promotable" 2))
            (let g1 (Group "A" (CellSet (set-empty))))
            (let g2 (Group "B" (CellSet (set-empty))))

            (let ys (Cons (Enable g1 (Attributes (map-empty))) (Cons (Enable g2 (Attributes (map-empty))) (Nil))))
            ; @static(3) par { ... }
            (let P1 (Par (Attributes (map-insert (map-empty) "static" 3)) ys))
            (let X (Enable g1 (Attributes m1)))
            (let Y (Enable g2 (Attributes m2)))
            ; @promotable(1) A; @static(3) par { ... }; @promotable(2) B;
            (let xs (Cons X (Cons P1 (Cons Y (Nil)))))
            
            ; @static(10) par { ... }
            (let P2 (Par (Attributes (map-insert (map-empty) "static" 10)) ys))
            (let xss (Cons P2 xs))

            ; par { @promotable(1) A; @promotable(2) B; }
            (let zs (Cons (Enable g1 (Attributes m1)) (Cons (Enable g2 (Attributes m2)) (Nil))))
            (let S3 (Par (Attributes (map-empty)) zs))
            (let xsss (Cons S3 xss))
            "#,
            r#"
            (check (= (sum-latency xs) 6)) ; 1 + 3 + 2
            (check (= (sum-latency xss) 16)) ; 1 + 3 + 2 + 10
            (check (= (sum-latency xsss) 18)) ; 1 + 3 + 2 + 10 + max(1, 2)
            "#,
            &utils::RewriteRule::iter().collect_vec(),
        )
    }

    #[ignore = "TODO(cgyurgyik): causing merge failures (necessary?)"]
    #[test]
    fn test_control_before() -> utils::Result {
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
            &utils::RewriteRule::iter().collect_vec(),
        )
    }

    #[test]
    fn test_exclusive() -> utils::Result {
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
            &utils::RewriteRule::iter().collect_vec(),
        )
    }

    #[test]
    fn test_fan_out() -> utils::Result {
        test_egglog(
            r#"
            (let A (Enable (Group "A" (CellSet (set-empty))) (Attributes (map-empty))))
            (let B (Enable (Group "B" (CellSet (set-empty))) (Attributes (map-empty))))
            (let C (Enable (Group "C" (CellSet (set-empty))) (Attributes (map-empty))))
            (let D (Enable (Group "D" (CellSet (set-empty))) (Attributes (map-empty))))
            (let xs (Cons A (Cons B (Cons C (Cons D (Nil))))))
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
            &utils::RewriteRule::iter().collect_vec(),
        )
    }

    #[test]
    fn test_par_to_seq() -> utils::Result {
        test_egglog(
            r#"
            (let g1 (Group "A" (CellSet (set-empty))))
            (let g2 (Group "B" (CellSet (set-empty))))
            (let P (Par (Attributes (map-insert (map-empty) "static" 1011)) 
                (Cons (Enable g1 (Attributes (map-insert (map-empty) "promotable" 1011)))
                (Cons (Enable g2 (Attributes (map-insert (map-empty) "promotable" 5)))
                    (Nil)))))
            (let S (Seq (Attributes (map-insert (map-empty) "static" 1016)) 
                (Cons (Enable g1 (Attributes (map-insert (map-empty) "promotable" 1011)))
                (Cons (Enable g2 (Attributes (map-insert (map-empty) "promotable" 5)))
                    (Nil)))))
        "#,
            r#"
            (check (= PAR-TO-SEQ 1000)) ; ... this test will fail otherwise.
            (check (= P S))
        "#,
            &utils::RewriteRule::iter().collect_vec(),
        )
    }

    #[test]
    fn test_collapse_seq() -> utils::Result {
        test_egglog(
            r#"
            (let A (Enable (Group "A" (CellSet (set-empty))) (Attributes (map-empty))))
            (let S (Seq (Attributes (map-empty)) (Cons A (Nil))))
            (let SS (Seq (Attributes (map-empty)) (Cons S (Nil))))
        "#,
            r#"(check (= S SS))"#,
            &utils::RewriteRule::iter().collect_vec(),
        )
    }

    #[test]
    fn test_collapse_par() -> utils::Result {
        test_egglog(
            r#"
            (let A (Enable (Group "A" (CellSet (set-empty))) (Attributes (map-empty))))
            (let P (Par (Attributes (map-empty)) (Cons A (Nil))))
            (let PP (Par (Attributes (map-empty)) (Cons P (Nil))))
        "#,
            r#"(check (= P PP))"#,
            &utils::RewriteRule::iter().collect_vec(),
        )
    }

    #[test]
    fn test_split_seq() -> utils::Result {
        test_egglog(
            r#"
            (let A (Enable (Group "A" (CellSet (set-empty))) (Attributes (map-empty))))
            (let B (Enable (Group "B" (CellSet (set-empty))) (Attributes (map-empty))))
            (let C (Enable (Group "C" (CellSet (set-empty))) (Attributes (map-empty))))
            (let D (Enable (Group "D" (CellSet (set-empty))) (Attributes (map-empty))))
            (let E (Enable (Group "E" (CellSet (set-empty))) (Attributes (map-empty))))
            (let F (Enable (Group "F" (CellSet (set-empty))) (Attributes (map-empty))))
            (let G (Enable (Group "G" (CellSet (set-empty))) (Attributes (map-empty))))
            (let H (Enable (Group "H" (CellSet (set-empty))) (Attributes (map-empty))))
            (let xs (Cons A (Cons B (Cons C (Cons D (Cons E (Cons F (Cons G (Cons H (Nil))))))))))
            (let l1 (Cons A (Cons B (Cons C (Cons D (Nil))))))
            (let l2 (Cons E (Cons F (Cons G (Cons H (Nil))))))
            (let s1 (list-slice xs 0 4))
            (let s2 (list-slice xs 4 8))
            (let S-before (Seq (Attributes (map-empty)) xs))
            (let S-after (Seq (Attributes (map-empty))
                (Cons (Seq (Attributes (map-insert (map-empty) "new_fsm" 1)) l1)
                (Cons (Seq (Attributes (map-insert (map-empty) "new_fsm" 1)) l2)
                    (Nil)))))
        "#,
            r#"
            (check (= SPLIT-SEQ 8)) ; ... this test will fail otherwise.
            (check (= s1 l1))
            (check (= s2 l2))
            ; (check (= S-before S-after))
        "#,
            &utils::RewriteRule::iter().collect_vec(),
        )
    }
}
