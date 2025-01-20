#[macro_export]
/// Internal macro used to homogenize representation for raw identifiers in
/// port names.
macro_rules! in_fix {
    ( r#in ) => {
        stringify!(in)
    };
    ( $name:ident ) => {
        stringify!($name)
    };
}

/// Helper macro designed to work with port_bindings!. Exists only to break if something
/// other than a literal or identifier is used and does not actually do anything other
/// than return what it matches.
#[macro_export]
macro_rules! lit_or_id {
    ($lit:literal) => {
        $lit
    };
    ($name:ident) => {
        $name
    };
    ($name:ident . $field:ident) => {
        $name.$field
    };
}

#[macro_export]
/// Helper macro to generate port bindings.
/// ```
/// # use cider::port_bindings;
/// port_bindings![ binds;
///   r#in -> (16, 32),
///   write_en -> (1, 1)
/// ];
/// assert!(binds[1].0 == "write_en");
/// assert!(binds[0].0 == "in");
/// ```
macro_rules! port_bindings {
    ( $binds: ident; $( $port: ident -> ($val: tt, $width: tt) ),+ ) => {
        $( let $port = baa::BitVecValue::from_u64($crate::lit_or_id!($val), $crate::lit_or_id!($width)); )+
        let $binds = vec![ $( (calyx_ir::Id::from($crate::in_fix!($port)), &$port) ),+ ];
    }
}

/// Helper macro to generate validation checks for the input passed to primitives
/// ```
///  # use cider::validate;
///  # use baa::BitVecValue;
///  # let input = [("left", [4,4,4,4])];
///  # let inputs = &input;
///  # let width = 4;
///  validate![inputs;
///       left: width,
///       right: width,
///       go: 1
///  ];
/// ```
#[macro_export]
macro_rules! validate {
    ( $inputs:ident; $( $port:ident : $width:expr ),+ ) => {
        for (id, v) in $inputs {
            match id.as_ref() {
                $( $crate::in_fix!($port) => assert_eq!(v.len() as u64, $width) ),+,
                p => unreachable!("Unknown port: {}", p),
            }
        }
    }
}

/// Helper macro to generate validation checks for the input passed to
/// primitives, does not error on unknown ports
/// ```
///  # use cider::validate_friendly;
///  # use baa::BitVecValue;
///  # let input = [("left", [4,4,4,4])];
///  # let inputs = &input;
///  # let width = 4;
///  validate_friendly![inputs;
///       left: width,
///       right: width,
///       go: 1
///  ];
/// ```
#[macro_export]
macro_rules! validate_friendly {
    ( $inputs:ident; $( $port:ident : $width:expr ),+ ) => {
        for (id, v) in $inputs {
            match id.as_ref() {
                $( $crate::in_fix!($port) => assert_eq!(v.len() as u64, $width) ),+,
                _ => {},
            }
        }
    }
}
