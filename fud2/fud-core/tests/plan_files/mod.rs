use fud_core::plan_files::session::ParseSession;

#[test]
fn basic_parse() {
    let src = "x = input(a, b); z, y = f(x);";
    let sess = ParseSession::with_str_buf(src);
    let ast = sess.parse();
    match ast {
        Ok(ast) => insta::assert_debug_snapshot!(ast),
        Err(e) => panic!("{:?}", e.msg()),
    }
}

#[test]
fn no_args() {
    let src = "x = input();";
    let sess = ParseSession::with_str_buf(src);
    let ast = sess.parse();
    match ast {
        Ok(ast) => insta::assert_debug_snapshot!(ast),
        Err(e) => panic!("{:?}", e.msg()),
    }
}

#[test]
fn single_assignment() {
    let src = "x = input(y);";
    let sess = ParseSession::with_str_buf(src);
    let ast = sess.parse();
    match ast {
        Ok(ast) => insta::assert_debug_snapshot!(ast),
        Err(e) => panic!("{:?}", e.msg()),
    }
}

#[test]
fn unicode_ids() {
    let src = "Æ = Ë(Ð);";
    let sess = ParseSession::with_str_buf(src);
    let ast = sess.parse();
    match ast {
        Ok(ast) => insta::assert_debug_snapshot!(ast),
        Err(e) => panic!("{:?}", e.msg()),
    }
}

#[test]
fn underscores() {
    let src = "__ = _hello__(_Ð);";
    let sess = ParseSession::with_str_buf(src);
    let ast = sess.parse();
    match ast {
        Ok(ast) => insta::assert_debug_snapshot!(ast),
        Err(e) => panic!("{:?}", e.msg()),
    }
}

#[test]
fn many_io_lists() {
    let src = "a, b, c, d, e, f = input(g, h, i, j, k); l, m, n, o = go(p, q, r, s, t); u, v = stop(w, x, y, z);";
    let sess = ParseSession::with_str_buf(src);
    let ast = sess.parse();
    match ast {
        Ok(ast) => insta::assert_debug_snapshot!(ast),
        Err(e) => panic!("{:?}", e.msg()),
    }
}

#[test]
fn weird_whitespace() {
    let src = "\u{2029}a,b,      c = \t\twah\u{2009}(c\n,d\r\n);\n\r\n\t    \te,f=g(h,i);";
    let sess = ParseSession::with_str_buf(src);
    let ast = sess.parse();
    match ast {
        Ok(ast) => insta::assert_debug_snapshot!(ast),
        Err(e) => panic!("{:?}", e.msg()),
    }
}

#[test]
fn ignore_input() {
    let src = "_ = input();";
    let sess = ParseSession::with_str_buf(src);
    let ast = sess.parse();
    match ast {
        Ok(ast) => insta::assert_debug_snapshot!(ast),
        Err(e) => panic!("{:?}", e.msg()),
    }
}

#[test]
fn missing_ident() {
    let src = " = input();";
    let sess = ParseSession::with_str_buf(src);
    let ast = sess.parse();
    match ast {
        Ok(ast) => panic!("this was supposed to be an error, but got: {ast:?}"),
        Err(e) => insta::assert_snapshot!(e.msg()),
    }
}

#[test]
fn empty_file() {
    let src = "";
    let sess = ParseSession::with_str_buf(src);
    let ast = sess.parse();
    match ast {
        Ok(ast) => insta::assert_debug_snapshot!(ast),
        Err(e) => insta::assert_snapshot!(e.msg()),
    }
}

#[test]
fn only_whitespace() {
    let src = "  \n \t ";
    let sess = ParseSession::with_str_buf(src);
    let ast = sess.parse();
    match ast {
        Ok(ast) => insta::assert_debug_snapshot!(ast),
        Err(e) => insta::assert_snapshot!(e.msg()),
    }
}

#[test]
fn missing_value() {
    let src = "x = ;";
    let sess = ParseSession::with_str_buf(src);
    let ast = sess.parse();
    match ast {
        Ok(ast) => insta::assert_debug_snapshot!(ast),
        Err(e) => insta::assert_snapshot!(e.msg()),
    }
}

#[test]
fn missing_assign_operator() {
    let src = "x input();";
    let sess = ParseSession::with_str_buf(src);
    let ast = sess.parse();
    match ast {
        Ok(ast) => insta::assert_debug_snapshot!(ast),
        Err(e) => insta::assert_snapshot!(e.msg()),
    }
}

#[test]
fn missing_arguments() {
    let src = "x = input;";
    let sess = ParseSession::with_str_buf(src);
    let ast = sess.parse();
    match ast {
        Ok(ast) => insta::assert_debug_snapshot!(ast),
        Err(e) => insta::assert_snapshot!(e.msg()),
    }
}

#[test]
fn missing_semicolon() {
    let src = "x = input()";
    let sess = ParseSession::with_str_buf(src);
    let ast = sess.parse();
    match ast {
        Ok(ast) => insta::assert_debug_snapshot!(ast),
        Err(e) => insta::assert_snapshot!(e.msg()),
    }
}

#[test]
fn missing_semicolon_with_more_after() {
    let src = "x = input() y = more();";
    let sess = ParseSession::with_str_buf(src);
    let ast = sess.parse();
    match ast {
        Ok(ast) => insta::assert_debug_snapshot!(ast),
        Err(e) => insta::assert_snapshot!(e.msg()),
    }
}

#[test]
fn trailing_comma_in_vars() {
    let src = "x, = input();";
    let sess = ParseSession::with_str_buf(src);
    let ast = sess.parse();
    match ast {
        Ok(ast) => insta::assert_debug_snapshot!(ast),
        Err(e) => insta::assert_snapshot!(e.msg()),
    }
}

#[test]
fn trailing_comma_in_args() {
    let src = "x, y = input(a, );";
    let sess = ParseSession::with_str_buf(src);
    let ast = sess.parse();
    match ast {
        Ok(ast) => insta::assert_debug_snapshot!(ast),
        Err(e) => insta::assert_snapshot!(e.msg()),
    }
}

#[test]
fn total_gibberish() {
    let src = "x, y, != i39(nput/(a, ),;:";
    let sess = ParseSession::with_str_buf(src);
    let ast = sess.parse();
    match ast {
        Ok(ast) => insta::assert_debug_snapshot!(ast),
        Err(e) => insta::assert_snapshot!(e.msg()),
    }
}

#[test]
fn double_semicolon() {
    let src = "x, y = _XxDarkNightxX_(oops, two, semicolons);;";
    let sess = ParseSession::with_str_buf(src);
    let ast = sess.parse();
    match ast {
        Ok(ast) => insta::assert_debug_snapshot!(ast),
        Err(e) => insta::assert_snapshot!(e.msg()),
    }
}
