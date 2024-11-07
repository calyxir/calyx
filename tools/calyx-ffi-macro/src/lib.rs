use std::{env, path::PathBuf};

use parse::CalyxFFIMacroArgs;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned};

mod calyx;
mod parse;
mod util;

fn get_first_token_span(ts: &TokenStream) -> Option<proc_macro::Span> {
    let mut iter = ts.clone().into_iter();
    iter.next().map(|token| token.span())
}

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

    // <sorry>
    let comp = calyx::parse_calyx_file(&args, path.clone());
    if let Err(error) = comp {
        return error;
    }
    let comp = comp.unwrap();
    let comp = comp.get();
    // </sorry>

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
    let mut field_names = vec![];
    let mut fields = vec![];
    let mut getters = vec![];

    for port in comp.signature.borrow().ports() {
        let port_name_str = port.borrow().name.to_string();
        let port_name = syn::parse_str::<syn::Ident>(&port_name_str)
            .expect("failed to turn port name into identifier");
        field_names.push(port_name.clone());
        // let port_width = port.borrow().width;

        // idk why input output ports are being flipped??
        match port.borrow().direction.reverse() {
            calyx_ir::Direction::Input => {
                fields.push(quote! {
                    pub #port_name: u64
                });
                input_names.push(port_name);
            }
            calyx_ir::Direction::Output => {
                fields.push(quote! {
                    #port_name: u64
                });
                getters.push(quote! {
                    pub fn #port_name(&self) -> u64 {
                        self.#port_name
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
            #(#getters)*
        }

        impl std::default::Default for #name {
            fn default() -> Self {
                Self {
                    #(#field_names: std::default::Default::default(),)*
                    user_data: unsafe { std::mem::MaybeUninit::zeroed() }
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
        for output in derive.outputs {
            getters.push(quote! {
                fn #output(&self) -> u64 {
                    self.#output
                }
            })
        }

        let mut setters = Vec::new();
        for input in derive.inputs {
            setters.push(quote! {
                fn #input(&mut self) -> &mut u64 {
                    &mut self.#input
                }
            })
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

#[derive(Default)]
struct CalyxFFITestModuleVisitor {
    pub wrappers: Vec<syn::ItemFn>,
    pub tests: Vec<syn::ItemFn>,
}

impl syn::visit::Visit<'_> for CalyxFFITestModuleVisitor {
    fn visit_item_fn(&mut self, i: &syn::ItemFn) {
        let has_calyx_ffi_test = i
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("calyx_ffi_test"));
        if has_calyx_ffi_test {
            let fn_name = &i.sig.ident;
            let dut_type = get_ffi_test_dut_type(i)
                .expect("calyx_ffi_test should enforce this invariant");

            self.wrappers.push(syn::parse_quote! {
                pub(crate) unsafe fn #fn_name(ffi: &mut CalyxFFI) {
                    let dut = ffi.new_comp::<#dut_type>();
                    let dut_ref = &mut *dut.borrow_mut();
                    let dut_pointer = dut_ref as *mut dyn CalyxFFIComponent as *mut _ as *mut #dut_type;
                    let dut_concrete: &mut #dut_type = &mut *dut_pointer;
                    super::#fn_name(dut_concrete);
                }
            });
            self.tests.push(syn::parse_quote! {
                #[test]
                pub(crate) fn #fn_name() {
                    let mut ffi = CalyxFFI::new();
                    unsafe {
                        super::calyx_ffi_generated_wrappers::#fn_name(&mut ffi);
                    }
                }
            });
        }
    }
}

#[proc_macro_attribute]
pub fn calyx_ffi_tests(args: TokenStream, item: TokenStream) -> TokenStream {
    if !args.is_empty() {
        return util::compile_error(
            &args.into_iter().next().unwrap().span().into(),
            "#[calyx_ffi_tests] takes no arguments".into(),
        );
    }

    let mut module = parse_macro_input!(item as syn::ItemMod);
    let module_name = &module.ident;

    let mut visitor = CalyxFFITestModuleVisitor::default();
    syn::visit::visit_item_mod(&mut visitor, &module);
    let wrappers = visitor.wrappers;
    let tests = visitor.tests;

    let test_names = wrappers.iter().map(|test| test.sig.ident.clone());
    let generated_wrappers = quote! {
        pub(crate) mod calyx_ffi_generated_wrappers {
            use super::*;

            pub(crate) const CALYX_FFI_TESTS: &'static [unsafe fn(&mut CalyxFFI) -> ()] = &[
                #(#test_names),*
            ];

            #(#wrappers)*
        }
    };
    let generated_wrappers_item: syn::Item =
        syn::parse2(generated_wrappers).unwrap();

    let generated_tests = quote! {
        pub(crate) mod calyx_ffi_generated_tests {
            use super::*;

            #(#tests)*
        }
    };
    let generated_tests_item: syn::Item = syn::parse2(generated_tests).unwrap();

    let items_to_add = vec![generated_wrappers_item, generated_tests_item];
    if let Some((_, ref mut items)) = module.content {
        items.extend(items_to_add);
    } else {
        module.content = Some((syn::token::Brace::default(), items_to_add));
    }

    quote! {
        #module

        pub mod calyx_ffi_generated_top {
            use super::*;

            pub unsafe fn run_tests(ffi: &mut CalyxFFI) {
                for test in #module_name::calyx_ffi_generated_wrappers::CALYX_FFI_TESTS {
                    test(ffi);
                }
            }
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn calyx_ffi_test(args: TokenStream, item: TokenStream) -> TokenStream {
    if !args.is_empty() {
        return util::compile_error(
            &args.into_iter().next().unwrap().span().into(),
            "#[calyx_ffi_test] takes no arguments".into(),
        );
    }

    let mut func = parse_macro_input!(item as syn::ItemFn);
    let dut_type = get_ffi_test_dut_type(&func);
    let Ok(dut_type) = dut_type else {
        return dut_type.err().unwrap();
    };

    let check_trait_impl = quote! {
        {
            fn assert_is_calyx_ffi_component<T: CalyxFFIComponent>() {}
            assert_is_calyx_ffi_component::<#dut_type>();
        }
    };

    let check_trait_impl_stmts: syn::Block = syn::parse2(check_trait_impl)
        .expect("Failed to parse check_trait_impl as a block");

    let new_stmts: Vec<syn::Stmt> = check_trait_impl_stmts
        .stmts
        .iter()
        .chain(func.block.stmts.iter())
        .cloned()
        .collect();

    let new_block = syn::Block {
        brace_token: func.block.brace_token,
        stmts: new_stmts,
    };
    func.block = Box::new(new_block);

    quote! {
        #func
    }
    .into()
}

fn get_ffi_test_dut_type(
    func: &syn::ItemFn,
) -> Result<&syn::Type, TokenStream> {
    let inputs: Vec<&syn::FnArg> = func.sig.inputs.iter().collect();

    let bad_sig_msg = "#[calyx_ffi_test] tests must take exactly one argument, namely, a mutable reference to the DUT".into();

    if inputs.len() != 1 {
        return Err(util::compile_error(&func.span(), bad_sig_msg));
    }
    let input = inputs.first().unwrap();

    let syn::FnArg::Typed(pat_ty) = input else {
        return Err(util::compile_error(&func.span(), bad_sig_msg));
    };

    let syn::Type::Reference(syn::TypeReference {
        mutability: Some(syn::token::Mut { span: _ }),
        ref elem,
        ..
    }) = *pat_ty.ty
    else {
        return Err(util::compile_error(&func.span(), bad_sig_msg));
    };

    Ok(elem)
}
