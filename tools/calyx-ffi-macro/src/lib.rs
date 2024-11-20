#![forbid(unsafe_code)]

use std::{env, path::PathBuf};

use parse::{CalyxFFIMacroArgs, CalyxPortDeclaration};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, spanned::Spanned};

mod calyx;
mod parse;
mod util;

// this is super bad, might go out of sync with interp::WidthInt
type WidthInt = u32;

/// Connects this `struct` to a calyx component in the given file.
#[proc_macro_attribute]
pub fn calyx_ffi(attrs: TokenStream, item: TokenStream) -> TokenStream {
    let source_manifest_dir = PathBuf::from(
        env::vars()
            .find(|(name, _)| name == "CARGO_MANIFEST_DIR")
            .expect("caller of calyx_ffi did not use cargo to build project")
            .1,
    );

    let args = parse_macro_input!(attrs as CalyxFFIMacroArgs);
    let item_struct = parse_macro_input!(item as syn::ItemStruct);
    let name = item_struct.ident;
    let given_path = args.src.to_string_lossy().to_string();

    let mut path = source_manifest_dir;
    path.push(given_path);
    let path = path;

    let comp = calyx::parse_calyx_file(&args, path.clone());
    if let Err(error) = comp {
        return error;
    }
    let comp = comp.unwrap();
    let comp = comp.get();

    let comp_name =
        syn::parse_str::<syn::LitStr>(&format!("\"{}\"", comp.name))
            .expect("failed to turn quoted name into string");
    let comp_path = syn::parse_str::<syn::LitStr>(&format!(
        "\"{}\"",
        path.to_string_lossy()
    ))
    .expect("failed to turn quoted path into string");

    let backend_macro = args.backend;
    let mut input_names = Vec::new();
    let mut output_names = Vec::new();
    let mut fields = vec![];
    let mut default_field_inits = vec![];
    let mut getters = vec![];
    let mut setters = vec![];
    let mut width_getters = vec![];
    let mut port_names = vec![];

    for port in comp.signature.borrow().ports() {
        let port_name_str = port.borrow().name.to_string();
        let port_name = syn::parse_str::<syn::Ident>(&port_name_str)
            .expect("failed to turn port name into identifier");

        port_names.push(port_name.clone());

        let port_width = port.borrow().width as WidthInt;
        let width_getter = format_ident!("{}_width", port_name);
        width_getters.push(quote! {
            pub const fn #width_getter() -> calyx_ffi::value::WidthInt {
                #port_width as calyx_ffi::value::WidthInt
            }
        });

        default_field_inits.push(quote! {
            #port_name: calyx_ffi::value::Value::from(0)
        });

        // we need to reverse the port direction
        match port.borrow().direction.reverse() {
            calyx_ir::Direction::Input => {
                let setter = format_ident!("set_{}", port_name);

                fields.push(quote! {
                    pub #port_name: calyx_ffi::value::Value<#port_width>
                });

                setters.push(quote! {
                    pub fn #setter(&mut self, value: u64) {
                        self.#port_name = calyx_ffi::value::Value::from(value);
                    }
                });

                input_names.push(port_name);
            }
            calyx_ir::Direction::Output => {
                fields.push(quote! {
                    #port_name: calyx_ffi::value::Value<#port_width>

                });

                let bitvec_getter = format_ident!("{}_bits", port_name);

                if port_width <= 64 {
                    getters.push(quote! {
                        pub fn #port_name(&self) -> u64 {
                            (&self.#port_name).try_into().expect("port value wider than 64 bits")
                        }
                    })
                }

                getters.push(quote! {
                    pub const fn #bitvec_getter(&self) -> &calyx_ffi::value::Value<#port_width> {
                        &self.#port_name
                    }
                });

                output_names.push(port_name);
            }
            calyx_ir::Direction::Inout => {
                todo!("inout ports not supported yet")
            }
        }
    }

    let struct_def = quote! {
        struct #name {
            #(#fields,)*
            user_data: std::mem::MaybeUninit<#backend_macro!(@user_data_type)>
        }
    };

    let impl_block = quote! {
        impl #name {
            #(#width_getters)*
            #(#getters)*
            #(#setters)*
        }

        impl std::default::Default for #name {
            fn default() -> Self {
                Self {
                    #(#default_field_inits),*,
                    user_data: unsafe { std::mem::MaybeUninit::zeroed() }
                }
            }
        }

        impl std::clone::Clone for #name {
            fn clone(&self) -> Self {
                Self {
                    #(#port_names: self.#port_names.clone()),*,
                    user_data: unsafe { std::mem::MaybeUninit::new(self.user_data.assume_init_ref().clone()) }
                }
            }
        }

        impl CalyxFFIComponent for #name {
            fn path(&self) -> &'static str {
                #comp_path
            }

            fn name(&self) -> &'static str {
                #comp_name
            }

            fn init(&mut self, context: &calyx_ir::Context) {
                #backend_macro!(@init self, context; #(#input_names),*; #(#output_names),*);
            }

            fn reset(&mut self) {
                #backend_macro!(@reset self;  #(#input_names),*; #(#output_names),*);
            }

            fn can_tick(&self) -> bool {
                #backend_macro!(@can_tick self;  #(#input_names),*; #(#output_names),*)
            }

            fn tick(&mut self) {
                #backend_macro!(@tick self; #(#input_names),*; #(#output_names),*);
            }

            fn go(&mut self) {
                #backend_macro!(@go self; #(#input_names),*; #(#output_names),*);
            }
        }
    };

    let mut derive_impls = Vec::new();

    for derive in args.derives {
        let trait_name = derive.name;

        let mut getters = Vec::new();
        for CalyxPortDeclaration(name, width) in derive.outputs {
            let name_bits = format_ident!("{}_bits", &name);

            getters.push(quote! {
                fn #name_bits(&self) -> &calyx_ffi::value::Value<#width> {
                    &self.#name
                }

                fn #name(&self) -> u64 {
                    Self::#name(self)
                }
            });
        }

        let mut setters = Vec::new();
        for CalyxPortDeclaration(name, width) in derive.inputs {
            let name_bits = format_ident!("{}_bits", name);
            let setter = format_ident!("set_{}", name);

            setters.push(quote! {
                fn #name_bits(&mut self) -> &mut calyx_ffi::value::Value<#width> {
                    &mut self.#name
                }

                fn #setter(&mut self, value: u64) {
                    Self::#setter(self, value);
                }
            });
        }

        derive_impls.push(quote! {
            impl #trait_name for #name {
                #(#getters)*
                #(#setters)*
            }
        });
    }

    quote! {
        #struct_def
        #impl_block
        #(#derive_impls)*
    }
    .into()
}
