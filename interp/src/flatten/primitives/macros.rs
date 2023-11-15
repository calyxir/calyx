macro_rules! ports {
    ($base:expr; $( $port:ident : $offset:expr ),+ ) => {
        $(let $port: $crate::flatten::flat_ir::prelude::GlobalPortId = ($crate::flatten::structures::index_trait::IndexRef::index($base) + $offset).into();)+
    }
}

macro_rules! declare_ports {
    ($( $port:ident : $offset:expr ),+  $(,)? ) => {

        $(
            #[allow(non_upper_case_globals)]
            const $port: usize = $offset;
        )+
    }
}

macro_rules! output {
    ( $( $port:ident : $value:expr ),+ $(,)? ) => {
        vec![$( ($port, $value).into(),)+]
    }
}

macro_rules! make_getters {
    ($base:ident; $( $port:ident : $offset:expr ),+ ) => {
        $(
            #[inline]
            fn $port(&self) -> $crate::flatten::flat_ir::prelude::GlobalPortId {
                ($crate::flatten::structures::index_trait::IndexRef::index(&self.$base) + $offset).into()
            }
        )+

    }
}

pub(crate) use declare_ports;
pub(crate) use make_getters;
pub(crate) use output;
pub(crate) use ports;

macro_rules! comb_primitive {
    ($name:ident$([$($param:ident),+])?
        ( $($port:ident [$port_idx:expr]),+ )
        ->
        ($($out_port:ident [$out_port_idx:expr]),+)
        $execute:block) => {
        #[derive(Clone, Debug)]
        #[allow(non_snake_case)]
        pub struct $name {
            $($($param: u32,)+)?
            base_port: $crate::flatten::flat_ir::prelude::GlobalPortId
        }

        impl $name {

            $crate::flatten::primitives::macros::declare_ports![$($port: $port_idx),+];
            $crate::flatten::primitives::macros::declare_ports![$($out_port: $out_port_idx),+];

            #[allow(non_snake_case)]
            pub fn new(
                base_port: $crate::flatten::flat_ir::prelude::GlobalPortId,
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
                port_map: &$crate::flatten::structures::environment::PortMap,
            ) -> $crate::flatten::primitives::prim_trait::Results {

                $crate::flatten::primitives::macros::ports![&self.base_port;
                    $($port: Self::$port,)+
                    $($out_port: Self::$out_port),+
                ];

                #[allow(non_snake_case)]
                let exec_func = |$($($param: u32,)+)? $($port: &$crate::values::Value),+, $($out_port:$crate::flatten::flat_ir::prelude::GlobalPortId,)+ | -> $crate::flatten::primitives::prim_trait::Results {
                    $execute
                };


                let out = exec_func(
                    $($(self.$param,)*)?
                    $(&port_map[$port],)+
                    $($out_port,)+
                );

                out
            }

            fn has_stateful(&self) -> bool {
                false
            }

            fn reset(&mut self, map:&$crate::flatten::structures::environment::PortMap) -> $crate::flatten::primitives::prim_trait::Results {
                self.exec_comb(map)
            }
        }
    };

}

pub(crate) use comb_primitive;
