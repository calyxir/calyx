use fud_core::{flang::session::ParseSession, visitors::ast_to_string};

macro_rules! test_parse {
    ($e:expr) => {
        let src = $e;
        let sess = ParseSession::with_str_buf(src);
        let ast = sess.parse();
        match ast {
            Ok(ast) => insta::assert_debug_snapshot!(ast),
            Err(e) => insta::assert_snapshot!(e),
        }
    };
}

#[test]
fn basic_parse() {
    test_parse!(r#""x" = "input"("a", "b"); "z", "y" = "f"("x");"#);
}

#[test]
fn no_args() {
    test_parse!(r#""x" = "input"();"#);
}

#[test]
fn single_assignment() {
    test_parse!(r#""x" = "input"("y");"#);
}

#[test]
fn unicode_ids() {
    test_parse!(r#""Æ" = "Ë"("Ð");"#);
}

#[test]
fn underscores() {
    test_parse!(r#""__" = "_hello__"("_Ð");"#);
}

#[test]
fn many_io_lists() {
    test_parse!(
        r#"a, b, c, d, e, f = "input"(g, h, i, j, k); l, m, n, o = go(p, q, r, s, t); u, v = stop(w, x, y, z);"#
    );
}

#[test]
fn weird_whitespace() {
    test_parse!(
        "\u{2029}a,b,      c = \t\twah\u{2009}(c\n,d\r\n);\n\r\n\t    \te,f=g(h,i);"
    );
}

#[test]
fn ignore_input() {
    test_parse!("_ = input();");
}

#[test]
fn missing_ident() {
    test_parse!(" = input();");
}

#[test]
fn empty_file() {
    test_parse!("");
}

#[test]
fn only_whitespace() {
    test_parse!("  \n \t ");
}

#[test]
fn missing_value() {
    test_parse!("x = ;");
}

#[test]
fn missing_assign_operator() {
    test_parse!("x input();");
}

#[test]
fn missing_arguments() {
    test_parse!("x = input;");
}

#[test]
fn missing_semicolon() {
    test_parse!("x = input()");
}

#[test]
fn missing_semicolon_with_more_after() {
    test_parse!("x = input() y = more();");
}

#[test]
fn trailing_comma_in_vars() {
    test_parse!("x, = input();");
}

#[test]
fn trailing_comma_in_args() {
    test_parse!("x, y = input(a, );");
}

#[test]
fn total_gibberish() {
    test_parse!("x, y, != i39(nput/(a, ),;:");
}

#[test]
fn double_semicolon() {
    test_parse!("x, y = _XxDarkNightxX_(oops, two, semicolons);;");
}

#[test]
fn simple_serialization() {
    let src = "a, b, c, d, e, f = input(g, h, i, j, k); l, m, n, o = go(p, q, r, s, t); u, v = stop(w, x, y, z);";
    let sess = ParseSession::with_str_buf(src);
    let ast = sess.parse();
    match ast {
        Ok(ast) => {
            let s = ast_to_string(&ast);
            insta::assert_snapshot!(s)
        }
        Err(e) => insta::assert_snapshot!(e),
    }
}

#[test]
fn empty_serialization() {
    let src = "";
    let sess = ParseSession::with_str_buf(src);
    let ast = sess.parse();
    match ast {
        Ok(ast) => {
            let s = ast_to_string(&ast);
            insta::assert_snapshot!(s)
        }
        Err(e) => insta::assert_snapshot!(e),
    }
}

#[test]
fn no_args_serialization() {
    let src = "x = input();";
    let sess = ParseSession::with_str_buf(src);
    let ast = sess.parse();
    match ast {
        Ok(ast) => {
            let s = ast_to_string(&ast);
            insta::assert_snapshot!(s)
        }
        Err(e) => insta::assert_snapshot!(e),
    }
}

#[test]
fn trailing_quotes() {
    test_parse!(r#"x, "y = input(a);"#);
}

#[test]
fn spaces_in_id() {
    test_parse!(r#""x lkjsdf - \\\" \"" = "cool ids"();"#);
}
