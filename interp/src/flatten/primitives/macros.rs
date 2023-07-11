macro_rules! ports {
    ($base:expr; $( $port:ident : $offset:expr ),+ ) => {
        $(let $port: $crate::flatten::flat_ir::prelude::GlobalPortId = ($crate::flatten::structures::index_trait::IndexRef::index($base) + $offset).into();)+
    }
}

macro_rules! declare_ports {
    ($( $port:ident : $offset:expr ),+  $(,)? ) => {
        $(const $port: usize = $offset;)+
    }
}

macro_rules! output {
    ( $( $port:ident : $value:expr ),+ $(,)? ) => {
        vec![$( ($port, $value).into(),)+]
    }
}

pub(crate) use declare_ports;
pub(crate) use output;
pub(crate) use ports;
