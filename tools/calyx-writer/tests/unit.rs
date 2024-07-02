#[cfg(test)]
mod tests {
    use calyx_writer::*;
    use std::fmt::{self, Display, Write};

    struct TestIndentFormatter;
    impl Display for TestIndentFormatter {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let mut f = IndentFormatter::new(f, 2);
            writeln!(f, "hello\ngoodbye")?;
            f.increase_indent();
            writeln!(f, "hello\ngoodbye")?;
            f.decrease_indent();
            writeln!(f, "hello\ngoodbye")
        }
    }

    #[test]
    fn test_indent_formatter() {
        assert_eq!(
            "hello\ngoodbye\n  hello\n  goodbye\nhello\ngoodbye\n",
            TestIndentFormatter.to_string()
        );
    }

    #[test]
    fn test_build_cell() {
        let mut prog = Program::new();
        let comp = prog.comp("test");
        build_cells!(comp;
            x = std_reg(32);
            ref y = std_reg(32);
            mem = comb_mem_d1(1, 2, 3);
            le = std_le();
        );
        assert!(!x.borrow().is_ref());
        assert!(y.borrow().is_ref());
        assert!(!mem.borrow().is_ref());
        assert!(!le.borrow().is_ref());
    }

    #[test]
    fn test_build_control() {
        let mut prog = Program::new();
        let comp = prog.comp("test");
        declare_group!(comp; group foo);
        declare_group!(comp; group bar);
        let control = build_control! {
            [seq {
                [foo],
                (Control::enable(bar.clone()))
            }]
        };
        assert_eq!(
            Control::seq(vec![
                Control::enable(foo.clone()),
                Control::enable(bar.clone())
            ]),
            control
        );
    }
}
