/// Convinience macro to get a port and turn it into an ir::Guard.
/// Example:
/// ```
/// let fsm_out = guard!(fsm["out"]);
/// ```
#[macro_export]
macro_rules! guard {
    ($node:ident[$port:expr]) => {
        crate::ir::Guard::from(
            $node
                .borrow()
                .get(&crate::ir::Id::from($port)),
        )
    };
}

/// Add primitives and constants to the component and `let`-bind the
/// references.
/// Example:
/// ```
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
            crate::ir::Id::from(stringify!($var)),
            crate::ir::Id::from(stringify!($comp)),
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
/// **Note**: Guards used in the assignments are `cloned`.
/// The macro accepts two forms:
/// ```
/// build_assignments!(builder;
///     group["go"] = signal_on["out"]; // no guard
///     fsm["in"] = guard ? add["out"];
/// )
/// ```
#[macro_export]
macro_rules! build_assignments {
    // Unguarded assignment.
    (@base $builder:expr;
     $dst_node:ident[$dst_port:expr] = ? $src_node:ident[$src_port:expr]) => {
        $builder.build_assignment(
            $dst_node.borrow().get(
                &crate::ir::Id::from($dst_port)),
            $src_node.borrow().get(
                &crate::ir::Id::from($src_port)),
            None)
    };

    // Guarded assignment.
    (@base $builder:expr;
     $dst_node:ident[$dst_port:expr] =
        $guard:ident ?
        $src_node:ident[$src_port:expr]) => {
        $builder.build_assignment(
            $dst_node.borrow().get(
                &crate::ir::Id::from($dst_port)),
            $src_node.borrow().get(
                &crate::ir::Id::from($src_port)),
            Some($guard.clone()))
    };

    ($builder:expr;
     $($dst_node:ident[$dst_port:expr] =
         $($guard:ident)? ?
         $src_node:ident[$src_port:expr];)*)  => {
        vec![$(
            build_assignments!(@base $builder;
                $dst_node[$dst_port] = $($guard)? ? $src_node[$src_port])
        ),*]

    };
}
