/// Parse guard expression into [`ir::Guard`](crate::Guard).
///
/// The identifier should either be a [`ir::Group`](crate::Group) or an
/// [`ir::Cell`](crate::Cell).
/// Example:
/// ```
/// let fsm_out = guard!(fsm["out"] == lb["out"] & g);
/// ```
///
/// The macro supports constructing guards using the following operators:
/// - Port access: `node[port]`
/// - Comparison operators: `==`, `>=`, `<=`, `>`, `<`
/// - Logical operators: `&`, `|`
/// - Parentheses: `()`
#[macro_export]
macro_rules! guard {
    // Base
    // Port access
    ($node:ident[$port:expr]) => {
        $crate::Guard::from($node.borrow().get($port))
    };
    // Parentheses
    ( ( $($head:tt)* ) ) => {
        guard!($($head)*)
    };
    // A bare name
    ($e:ident) => { $e };

    // Comparison operators
    ($node:ident[$port:expr] >= $($tail:tt)*) => {
        $crate::Guard::from($node.borrow().get($port)).ge(guard!($($tail)*))
    };
    ($node:ident[$port:expr] <= $($tail:tt)*) => {
        $crate::Guard::from($node.borrow().get($port)).le(guard!($($tail)*))
    };
    ($node:ident[$port:expr] < $($tail:tt)*) => {
        $crate::Guard::from($node.borrow().get($port)).lt(guard!($($tail)*))
    };
    ($node:ident[$port:expr] > $($tail:tt)*) => {
        $crate::Guard::from($node.borrow().get($port)).gt(guard!($($tail)*))
    };
    ($node:ident[$port:expr] == $($tail:tt)*) => {
        $crate::Guard::from($node.borrow().get($port)).eq(guard!($($tail)*))
    };
    ($node:ident[$port:expr] != $($tail:tt)*) => {
        $crate::Guard::from($node.borrow().get($port)).neq(guard!($($tail)*))
    };

    // Logical operators
    // AND
    ($node:ident[$port:expr] & $($tail:tt)*) => {
        $crate::Guard::from($node.borrow().get($port)) & guard!($($tail)*)
    };
    ( ( $($head:tt)* ) & $($tail:tt)*) => {
        guard!($($head)*) & guard!($($tail)*)
    };

    // OR
    ($node:ident[$port:expr] | $($tail:tt)*) => {
        $crate::Guard::from($node.borrow().get($port)) | guard!($($tail)*)
    };
    ( ( $($head:tt)* ) | $($tail:tt)*) => {
        guard!($($head)*) | guard!($($tail)*)
    };
}

/// Add primitives and constants to the component and `let`-bind the
/// references.
///
/// Example:
/// ```
/// let builder = ir::Builder::from(&mut component, &sigs, validate);
/// structure!(builder;
///     let signal_on = constant(1, 32); // Define 32-bit constant 1.
///     let fsm_reg = prim std_reg(32);  // Define 32-bit register.
/// )
/// ```
#[macro_export]
macro_rules! structure {
    ($builder:expr;) => { };

    ($builder:expr;
     let $var:ident = prim $comp:ident( $($n:expr),* ); $($tail:tt)*) => {
        let $var = $builder.add_primitive(
            stringify!($var),
            stringify!($comp),
            &[$($n),*]
        );
        structure!($builder; $($tail)*)
    };

    ($builder:expr;
     let $var:pat = constant($v:expr, $w:expr); $($tail:tt)*) => {
        let $var = $builder.add_constant($v, $w);
        structure!($builder; $($tail)*)
    }
}

/// Build guarded assignment statements and return a vector containing them.
///
/// The macro accepts two forms:
/// ```
/// build_assignments!(builder;
///     group["go"] = ? signal_on["out"]; // no guard
///     fsm["in"] = guard ? add["out"];
/// )
/// ```
/// **Note**: Guards used in the assignments are `cloned`.
#[macro_export]
macro_rules! build_assignments {
    // Unguarded assignment.
    (@base $builder:expr;
     $dst_node:ident[$dst_port:expr] = ? $src_node:ident[$src_port:expr]) => {
        $builder.build_assignment(
            $dst_node.borrow().get($dst_port),
            $src_node.borrow().get($src_port),
            $crate::Guard::True)
    };

    // Guarded assignment.
    (@base $builder:expr;
     $dst_node:ident[$dst_port:expr] =
        $guard:ident ?
        $src_node:ident[$src_port:expr]) => {
        $builder.build_assignment(
            $dst_node.borrow().get($dst_port),
            $src_node.borrow().get($src_port),
            $guard.clone())
    };

    ($builder:expr;
     $($dst_node:ident[$dst_port:expr] =
         $($guard:ident)? ?
         $src_node:ident[$src_port:expr];)*)  => {
        [$(
            build_assignments!(@base $builder;
                $dst_node[$dst_port] = $($guard)? ? $src_node[$src_port])
        ),*]

    };
}
