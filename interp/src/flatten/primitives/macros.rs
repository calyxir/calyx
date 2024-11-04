// taken from https://veykril.github.io/tlborm/decl-macros/building-blocks/counting.html
macro_rules! replace_expr {
    ($_t:tt $sub:expr) => {
        $sub
    };
}

macro_rules! count_tts {
    ($($tts:tt)*) => {0usize $(+ $crate::flatten::primitives::macros::replace_expr!($tts 1usize))*};
}
// -- https://veykril.github.io/tlborm/decl-macros/building-blocks/counting.html

pub(crate) use count_tts;
pub(crate) use replace_expr;

macro_rules! ports {
    ($base:expr; $( $port:ident : $offset:expr ),+ ) => {
        $(let $port: $crate::flatten::flat_ir::prelude::GlobalPortIdx = ($crate::flatten::structures::index_trait::IndexRef::index($base) + $offset).into();)+
    }
}

/// Declare a list of ports for the primitive with their offsets given relative
/// to the cell's base port.A vertical bar is used to separate the input and
/// output ports
///
/// ## NOTE: These must be given in ascending order. And the base port of the struct must be named `base_port`
///
/// ```
/// // Example
/// // declare COND, TRU, FAL as input ports
/// // declare OUT as output port
/// declare_ports![ COND: 0, TRU: 1, FAL:2 | OUT: 3];
/// ```
macro_rules! declare_ports {
    ($( $input_port:ident : $input_offset:literal ),+ $(,)? |  $( $output_port:ident : $output_offset:literal ),+ $(,)? ) => {

        $(
            #[allow(non_upper_case_globals)]
            const $input_port: usize = $input_offset; // this is a usize because it encodes the position of the port!
        )+

        $(
            #[allow(non_upper_case_globals)]
            const $output_port: usize = $output_offset; // this is a usize because it encodes the position of the port!
        )+

        // determine the offset of the first and last output ports
        const SPLIT_IDX: usize = [$($output_offset),+][0];
        const END_IDX: usize = ([$($output_offset),+][$crate::flatten::primitives::macros::count_tts!($($output_offset)+) - 1]) + 1;

        #[inline]
        pub fn get_signature(&self) -> $crate::flatten::structures::index_trait::SplitIndexRange<$crate::flatten::flat_ir::prelude::GlobalPortIdx> {
            use $crate::flatten::structures::index_trait::IndexRef;

            $crate::flatten::structures::index_trait::SplitIndexRange::new(self.base_port,
                $crate::flatten::flat_ir::prelude::GlobalPortIdx::new(self.base_port.index() + Self::SPLIT_IDX),
                $crate::flatten::flat_ir::prelude::GlobalPortIdx::new(self.base_port.index() + Self::END_IDX),
            )
        }
    }
}

/// Declare a list of ports for the primitive with their offsets given relative
/// to the cell's base port. Unlike `declare_ports` this does not generate a
/// `get_signature` function or distinguish between input and output ports.
macro_rules! declare_ports_no_signature {
    ($( $port:ident : $offset:literal ),+  $(,)? ) => {
        $(
            #[allow(non_upper_case_globals)]
            const $port: usize = $offset; // this is a usize because it encodes the position of the port!
        )+
    }
}

macro_rules! make_getters {
    ($base:ident; $( $port:ident : $offset:expr ),+ ) => {
        $(
            #[inline]
            fn $port(&self) -> $crate::flatten::flat_ir::prelude::GlobalPortIdx {
                ($crate::flatten::structures::index_trait::IndexRef::index(&self.$base) + &self.addresser.non_address_base() + $offset).into()
            }
        )+

    }
}

pub(crate) use declare_ports;
pub(crate) use declare_ports_no_signature;
pub(crate) use make_getters;

pub(crate) use ports;

macro_rules! comb_primitive {
    ($name:ident$([$($param:ident),+])?
        ( $($port:ident [$port_idx:expr]),+ )
        ->
        ($out_port:ident [$out_port_idx:expr])
        $execute:block) => {
        #[derive(Clone, Debug)]
        #[allow(non_snake_case)]
        pub struct $name {
            $($($param: u32,)+)?
            base_port: $crate::flatten::flat_ir::prelude::GlobalPortIdx
        }

        impl $name {

            $crate::flatten::primitives::macros::declare_ports![$($port: $port_idx),+ | $out_port: $out_port_idx];

            #[allow(non_snake_case)]
            pub fn new(
                base_port: $crate::flatten::flat_ir::prelude::GlobalPortIdx,
                $($($param: u32,)+)?
            ) -> Self {
                Self {
                    base_port,
                    $($($param,)+)?
                }
            }
        }

        impl $crate::flatten::primitives::Primitive for $name {
            fn exec_comb(
                &self,
                port_map: &mut $crate::flatten::structures::environment::PortMap,
            ) -> $crate::flatten::primitives::prim_trait::UpdateResult {

                $crate::flatten::primitives::macros::ports![&self.base_port;
                    $($port: Self::$port,)+
                    $out_port: Self::$out_port
                ];


                #[allow(non_snake_case)]
                let exec_func = |$($($param: u32,)+)? $($port: &$crate::flatten::flat_ir::prelude::PortValue),+| ->$crate::errors::RuntimeResult<Option<baa::BitVecValue>>  {
                    $execute
                };


                let output = exec_func(
                    $($(self.$param,)*)?
                    $(&port_map[$port],)+

                )?;

                if let Some(val) = output {
                    if port_map[$out_port].val().is_some() && *port_map[$out_port].val().unwrap() == val {
                        Ok($crate::flatten::primitives::prim_trait::UpdateStatus::Unchanged)
                    } else {
                        port_map[$out_port] = $crate::flatten::flat_ir::prelude::PortValue::new_cell(val);
                        Ok($crate::flatten::primitives::prim_trait::UpdateStatus::Changed)
                    }
                } else {
                    port_map.write_undef($out_port)?;
                    Ok($crate::flatten::primitives::prim_trait::UpdateStatus::Unchanged)
                }
            }

            fn has_stateful(&self) -> bool {
                false
            }

            fn get_ports(&self) -> $crate::flatten::structures::index_trait::SplitIndexRange<$crate::flatten::flat_ir::prelude::GlobalPortIdx> {
                self.get_signature()
            }

        }
    };

}

macro_rules! all_defined {
    ($($port_name:ident),+) => {
        #[allow(unused_parens)]
        let ($($port_name),+) = if [$($port_name),+].iter().all(|x|x.is_def()) {
            ($($port_name.val().unwrap()),+)
        } else {
            return Ok(None);
        };
    };
}

pub(crate) use all_defined;
pub(crate) use comb_primitive;
