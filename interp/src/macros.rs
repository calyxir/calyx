/// Define a primitive
///
/// ```
///  # use interp::comb_primitive;
///  # use interp::values::Value;
///  # use interp::primitives::Primitive;
/// comb_primitive!(StdAdd[width](left: width, right: width) -> (out: width) {
///   let left_64 = left.as_u64();
///   let right_64 = right.as_u64();
///   let init_val = left_64 + right_64;
///   let bitwidth = left.width();
///   Ok(Value::from(init_val, bitwidth))
/// });
///
/// ```
/// The macro implements the [Primitive](crate::primitives::Primitive) trait
/// for the struct as well as `StdAdd::new(bindings: ir::Params)` and
/// `StdAdd::from_constants(ports)`
///
/// TODO(rachit): $out_width is never used. (TODO: Griffin) fix the copy-paste
#[macro_export]
macro_rules! comb_primitive {
    ($name:ident[
        $( $param:ident ),+
    ]( $( $port:ident : $width:ident ),+ ) ->
     ( $( $out:ident : $out_width:ident ),+ ) $execute:block
    ) => {
        comb_primitive!(LOG; NAME; $name[$( $param ),+]($( $port : $width ),+) -> ($($out : $out_width),+ ) $execute);
    };


    (FLAG : $flag:ident; LOG: $log:ident; $name:ident[
        $( $param:ident ),+
    ]( $( $port:ident : $width:ident ),+ ) ->
     ( $( $out:ident : $out_width:ident ),+ ) $execute:block
    ) => {
        comb_primitive!($flag; $log; NAME; $name[$( $param ),+]($( $port : $width ),+) -> ($($out : $out_width),+ ) $execute);
    };

    (LOG: $log:ident; FLAG : $flag:ident; $name:ident[
        $( $param:ident ),+
    ]( $( $port:ident : $width:ident ),+ ) ->
     ( $( $out:ident : $out_width:ident ),+ ) $execute:block
    ) => {
        comb_primitive!($flag; $log; NAME; $name[$( $param ),+]($( $port : $width ),+) -> ($($out : $out_width),+ ) $execute);
    };

    (FLAG : $flag:ident; $name:ident[
        $( $param:ident ),+
    ]( $( $port:ident : $width:ident ),+ ) ->
     ( $( $out:ident : $out_width:ident ),+ ) $execute:block
    ) => {
        comb_primitive!($flag; LOG; NAME; $name[$( $param ),+]($( $port : $width ),+) -> ($($out : $out_width),+ ) $execute);
    };

    (FLAG : $flag:ident; NAME : $full_name:ident; $name:ident[
        $( $param:ident ),+
    ]( $( $port:ident : $width:ident ),+ ) ->
     ( $( $out:ident : $out_width:ident ),+ ) $execute:block
    ) => {
        comb_primitive!($flag; LOG; $full_name; $name[$( $param ),+]($( $port : $width ),+) -> ($($out : $out_width),+ ) $execute);
    };


    (LOG : $log:ident; $name:ident[
        $( $param:ident ),+
    ]( $( $port:ident : $width:ident ),+ ) ->
     ( $( $out:ident : $out_width:ident ),+ ) $execute:block
    ) => {
        comb_primitive!($log; NAME; $name[$( $param ),+]($( $port : $width ),+) -> ($($out : $out_width),+ ) $execute);
    };

    (NAME : $full_name:ident; $name:ident[
        $( $param:ident ),+
    ]( $( $port:ident : $width:ident ),+ ) ->
     ( $( $out:ident : $out_width:ident ),+ ) $execute:block
    ) => {
        comb_primitive!(LOG; $full_name; $name[$( $param ),+]($( $port : $width ),+) -> ($($out : $out_width),+ ) $execute);
    };

    ($log:ident; $full_name:ident; $name:ident[
        $( $param:ident ),+
    ]( $( $port:ident : $width:ident ),+ ) ->
     ( $( $out:ident : $out_width:ident ),+ ) $execute:block
    ) => {
        #[derive(Clone, Debug)]
        #[allow(non_snake_case)]
        pub struct $name {
            $($param: u64),+,
            name: calyx_ir::Id,
            logger: $crate::logging::Logger,
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    name: "".into(),
                    logger: $crate::logging::new_sublogger(""),
                    $($param: 0),+,
                }
            }
        }


        impl $name {
            pub fn new(params: &calyx_ir::Binding, name: calyx_ir::Id) -> Self {
                let mut base = Self::default();
                for (param, value) in params {
                    match param.as_ref() {
                        $( $crate::in_fix!($param) => base.$param = *value ),+,
                        p => unreachable!("Unknown parameter: {}", p),
                    }
                }
                base.logger = $crate::logging::new_sublogger(&name);
                base.name = name;
                base
            }

            #[allow(non_snake_case)]
            pub fn from_constants($( $param: u64 ),+, name: calyx_ir::Id) -> Self {
                $name {
                    $($param),+,
                    logger: $crate::logging::new_sublogger(&name),
                    name
                }
            }
        }

        impl $crate::primitives::Named for $name {
            fn get_full_name(&self) -> &calyx_ir::Id {
                &self.name
            }
        }


        impl $crate::primitives::Primitive for $name {

            //null-op; comb don't use do_tick()
            fn do_tick(&mut self) -> $crate::errors::InterpreterResult<Vec<(calyx_ir::Id, $crate::values::Value)>>{
                Ok(vec![])
            }

            fn is_comb(&self) -> bool { true }

            fn validate(
                &self,
                inputs: &[(calyx_ir::Id, &$crate::values::Value)]
            ) {
                for (id, v) in inputs {
                    match id.as_ref() {
                        $( $crate::in_fix!($port) => assert_eq!(v.len() as u64, self.$width) ),+,
                        p => unreachable!("Unknown port: {}", p),
                    }
                }
            }

            #[allow(non_snake_case,unused)]
            fn execute(
                &mut self,
                inputs: &[(calyx_ir::Id, &$crate::values::Value)],
            ) -> $crate::errors::InterpreterResult<Vec<(calyx_ir::Id, $crate::values::Value)>> {

                #[derive(Default)]
                struct Ports<'a> {
                    $( $port: Option<&'a $crate::values::Value> ),+
                }

                let mut base = Ports::default();

                for (id, v) in inputs {
                    match id.as_ref() {
                        $( $crate::in_fix!($port) => base.$port = Some(v) ),+,
                        p => unreachable!("Unknown port: {}", p),
                    }
                }

                let exec_func = |$($param: u64),+, $( $port: &Value ),+, $log: &$crate::logging::Logger, $full_name:&calyx_ir::Id| -> $crate::errors::InterpreterResult<Value> {
                    $execute
                };

                #[allow(unused_parens)]
                let ($( $out ),+) = exec_func(
                    $(self.$param),+,
                    $( base
                        .$port
                        .expect(&format!("No value for port: {}", $crate::in_fix!($port)).to_string()) ),+,
                    &self.logger,
                    $crate::primitives::Named::get_full_name(self),
                )?;

                return Ok(vec![
                    $( ($crate::in_fix!($out).into(), $out) ),+
                ])

            }

            // Combinational components cannot be reset
            fn reset(
                &mut self,
                inputs: &[(calyx_ir::Id, &$crate::values::Value)],
            ) -> $crate::errors::InterpreterResult<Vec<(calyx_ir::Id, $crate::values::Value)>> {
                self.execute(inputs)
            }

        }
    };

    ($flag: ident; $log:ident; $full_name:ident; $name:ident[
        $( $param:ident ),+
    ]( $( $port:ident : $width:ident ),+ ) ->
     ( $( $out:ident : $out_width:ident ),+ ) $execute:block
    ) => {
        #[derive(Clone, Debug)]
        #[allow(non_snake_case)]
        pub struct $name {
            $($param: u64),+,
            name: calyx_ir::Id,
            logger: $crate::logging::Logger,
            $flag: bool
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    name: "".into(),
                    logger: $crate::logging::new_sublogger(""),
                    $flag: false,
                    $($param: 0),+,
                }
            }
        }


        impl $name {
            pub fn new(params: &calyx_ir::Binding, name: calyx_ir::Id, $flag: bool) -> Self {
                let mut base = Self::default();
                for (param, value) in params {
                    match param.as_ref() {
                        $( $crate::in_fix!($param) => base.$param = *value ),+,
                        p => unreachable!("Unknown parameter: {}", p),
                    }
                }
                base.logger = $crate::logging::new_sublogger(&name);
                base.name = name;
                base.$flag = $flag;
                base
            }

            #[allow(non_snake_case)]
            pub fn from_constants($( $param: u64 ),+, name: calyx_ir::Id, $flag: bool) -> Self {
                $name {
                    $($param),+,
                    logger: $crate::logging::new_sublogger(&name),
                    name,
                    $flag
                }
            }
        }

        impl $crate::primitives::Named for $name {
            fn get_full_name(&self) -> &calyx_ir::Id {
                &self.name
            }
        }


        impl $crate::primitives::Primitive for $name {

            //null-op; comb don't use do_tick()
            fn do_tick(&mut self) -> $crate::errors::InterpreterResult<Vec<(calyx_ir::Id, $crate::values::Value)>>{
                Ok(vec![])
            }

            fn is_comb(&self) -> bool { true }

            fn validate(
                &self,
                inputs: &[(calyx_ir::Id, &$crate::values::Value)]
            ) {
                for (id, v) in inputs {
                    match id.as_ref() {
                        $( $crate::in_fix!($port) => assert_eq!(v.len() as u64, self.$width) ),+,
                        p => unreachable!("Unknown port: {}", p),
                    }
                }
            }

            #[allow(non_snake_case,unused)]
            fn execute(
                &mut self,
                inputs: &[(calyx_ir::Id, &$crate::values::Value)],
            ) -> $crate::errors::InterpreterResult<Vec<(calyx_ir::Id, $crate::values::Value)>> {

                #[derive(Default)]
                struct Ports<'a> {
                    $( $port: Option<&'a $crate::values::Value> ),+
                }

                let mut base = Ports::default();

                for (id, v) in inputs {
                    match id.as_ref() {
                        $( $crate::in_fix!($port) => base.$port = Some(v) ),+,
                        p => unreachable!("Unknown port: {}", p),
                    }
                }

                let exec_func = |$($param: u64),+, $( $port: &Value ),+, $log: &$crate::logging::Logger, $full_name:&calyx_ir::Id, $flag: bool| -> $crate::errors::InterpreterResult<Value> {
                    $execute
                };

                #[allow(unused_parens)]
                let ($( $out ),+) = exec_func(
                    $(self.$param),+,
                    $( base
                        .$port
                        .expect(&format!("No value for port: {}", $crate::in_fix!($port)).to_string()) ),+,
                    &self.logger,
                    $crate::primitives::Named::get_full_name(self),
                    self.$flag
                )?;

                return Ok(vec![
                    $( ($crate::in_fix!($out).into(), $out) ),+
                ])

            }

            // Combinational components cannot be reset
            fn reset(
                &mut self,
                inputs: &[(calyx_ir::Id, &$crate::values::Value)],
            ) -> $crate::errors::InterpreterResult<Vec<(calyx_ir::Id, $crate::values::Value)>> {
                self.execute(inputs)
            }

        }
    };
}

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
/// # use interp::port_bindings;
/// port_bindings![ binds;
///   r#in -> (16, 32),
///   write_en -> (1, 1)
/// ];
/// assert!(binds[1].0 == "write_en");
/// assert!(binds[0].0 == "in");
/// ```
macro_rules! port_bindings {
    ( $binds: ident; $( $port: ident -> ($val: tt, $width: tt) ),+ ) => {
        $( let $port = $crate::values::Value::from($crate::lit_or_id!($val), $crate::lit_or_id!($width)); )+
        let $binds = vec![ $( (calyx_ir::Id::from($crate::in_fix!($port)), &$port) ),+ ];
    }
}

/// Helper macro to generate validation checks for the input passed to primitives
/// ```
///  # use interp::validate;
///  # use interp::values::Value;
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
///  # use interp::validate_friendly;
///  # use interp::values::Value;
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
