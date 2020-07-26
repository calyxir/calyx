/// Simple macro that provides convienent syntax for
/// constructing ports. The syntax is:
/// ```
/// port!(st; node["hole-string"])
/// port!(st; node[hole_var])
/// ```
/// The main point of this macro is to make port
/// construction easier to read, and thus easier to debug.
#[macro_export]
macro_rules! port {
    ($struct:expr; $node:ident[$port:expr]) => {
        (
            $node,
            $struct.port_ref($node, $port).expect("port!").clone(),
        )
    };
}

#[macro_export]
macro_rules! guard {
    ($struct:expr; $node:ident[$port:expr]) => {
        st.to_guard(port!($struct, $node[$port]))
    };
}

#[macro_export]
macro_rules! structure {
    ($struct:expr,
     $ctx:expr,
     ) => {
    };

    ($struct:expr,
     $ctx:expr,
     let $var:pat = prim $comp:ident( $($n:expr),* ); $($tail:tt)*) => {
        let $var = $struct.new_primitive(
            $ctx,
            stringify!($var),
            stringify!($comp),
            &[$($n),*]
        )?;
        structure!($struct, $ctx, $($tail)*)
    };

    ($struct:expr,
     $ctx:expr,
     let $var:pat = constant($v:expr, $w:expr); $($tail:tt)*) => {
        let $var = $struct.new_constant($v, $w)?;
        structure!($struct, $ctx, $($tail)*)
    }
}

#[macro_export]
macro_rules! add_wires {
    // Terminating pattern
    ($struct:expr,
     $group:expr,) => {};

    // Base pattern 1: Unguarded assignment
    ($struct:expr,
     $group:expr,
     $dst:ident = ? ($src:ident); $($tail:tt)*) => {
        $struct.insert_edge(
            $src,
            $dst,
            $group.clone(),
            None
        )?;
        add_wires!($struct, $group, $($tail)*)
    };

    // Base pattern 1: Guarded assignment
    ($struct:expr,
     $group:expr,
     $dst:ident = $guard:ident ? ($src:ident); $($tail:tt)*) => {
        $struct.insert_edge(
            $src,
            $dst,
            $group.clone(),
            Some($guard.clone())
        )?;
        add_wires!($struct, $group, $($tail)*);
    };

    // Recursive pattern where RHS is a port expression.
    ($struct:expr,
     $group:expr,
     $dst_node:ident[$dst_port:expr] = $($guard:ident ?)? ($src_node:ident[$src_port:expr]); $($tail:tt)*)  => {
        add_wires!($struct, $group,
            (port!($struct; $dst_node[$dst_port])) = $($guard ?)? (port!($struct; $src_node[$src_port]));
            $($tail)*
        );
    };

    // Recursive pattern where src and dst can both be exprs.
    ($struct:expr,
     $group:expr,
     ($dst:expr) = $($guard:ident ?)? ($src:expr); $($tail:tt)*)  => {
        let dst = $dst;
        let src = $src;
        add_wires!($struct, $group,
            dst = $($guard)? ? (src);
            $($tail)*
        );
    };

    // Recursive pattern where LHS is a port expression.
    ($struct:expr,
     $group:expr,
     $dst_node:ident[$dst_port:expr] = $($guard:ident ?)? ($src:expr); $($tail:tt)*)  => {
        add_wires!($struct, $group,
            (port!($struct; $dst_node[$dst_port])) = $($guard ?)? ($src);
            $($tail)*
        );
    };

}
