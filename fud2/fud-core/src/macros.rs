#[macro_export]
macro_rules! defop {
    { $name:expr,[$bld:expr][$($s:expr),*]($($i:ident: $t:expr),*) -> $($o:ident: $t2:expr),*; $e:ident => $b:block} => {
        #[repr(usize)]
        #[allow(non_camel_case_types)]
        enum I {
            $($i),*
        }

        #[repr(usize)]
        #[allow(non_camel_case_types)]
        enum O {
            $($o),*
        }

        $bld.op(stringify!($name), &[$($s),*], &[$($t),*], &[$($t2),*], |e , input, output| {
            $(let $i = input[I::$i as usize];)*
            $(let $o = output[O::$o as usize];)*
            let $e = e;

            $b
            Ok(())
        });
    }
}
