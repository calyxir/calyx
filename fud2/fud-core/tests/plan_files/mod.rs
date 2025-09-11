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
