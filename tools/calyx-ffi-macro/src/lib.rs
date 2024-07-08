use parse::CalyxFFIMacroArgs;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, spanned::Spanned};

mod calyx;
mod parse;
mod util;

#[proc_macro_attribute]
pub fn calyx_ffi(attrs: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attrs as CalyxFFIMacroArgs);
    let item_struct = parse_macro_input!(item as syn::ItemStruct);
    let name = item_struct.ident;

    // <sorry>
    let comp = calyx::parse_calyx_file(&args);
    if let Err(error) = comp {
        return error;
    }
    let comp = comp.unwrap();
    let comp = comp.get();
    // </sorry>

    let comp_name = syn::parse_str::<syn::LitStr>(&format!(
        "\"{}\"",
        comp.name.to_string()
    ))
    .expect("failed to turn quoted name into string");

    let backend_macro = args.backend;
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
            }
            calyx_ir::Direction::Inout => {
                todo!("inout ports not supported yet")
            }
        }
    }

    let struct_def = quote! {
        struct #name {
            #(#fields),*
        }
    };

    let impl_block = quote! {
        impl #name {
            #(#getters)*
        }

        impl CalyxFFIComponent for #name {
            fn name(&self) -> &'static str {
                #comp_name
            }

            fn init(&mut self) {
                #backend_macro!(init self; #(#field_names),*);
            }

            fn deinit(&mut self) {
                #backend_macro!(deinit self; #(#field_names),*);
            }

            fn reset(&mut self) {
                #backend_macro!(reset self; #(#field_names),*);
            }

            fn tick(&mut self) {
                #backend_macro!(tick self; #(#field_names),*);
            }

            fn go(&mut self) {
                #backend_macro!(go self; #(#field_names),*);
            }
        }
    };

    quote! {
        #[derive(Default)]
        #struct_def
        #impl_block
    }
    .into()
}

#[derive(Default)]
struct CalyxFFITestModuleVisitor {
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
            let gen_fn_name =
                format_ident!("calyx_ffi_generated_wrapper_for_{}", fn_name);
            let dut_type = get_ffi_test_dut_type(i)
                .expect("calyx_ffi_test should enforce this invariant");

            self.tests.push(syn::parse_quote! {
                unsafe fn #gen_fn_name(ffi: &mut CalyxFFI) {
                    let dut = ffi.comp::<#dut_type>();
                    let dut_ref = &mut *dut.borrow_mut();
                    let dut_pointer = dut_ref as *mut dyn CalyxFFIComponent as *mut _ as *mut #dut_type;
                    let dut_concrete: &mut #dut_type = &mut *dut_pointer;
                    #fn_name(dut_concrete);
                }
            })
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

    let test_names = visitor.tests.iter().map(|test| test.sig.ident.clone());
    let test_array = quote! {
        pub const CALYX_FFI_TESTS: &'static [unsafe fn(&mut CalyxFFI) -> ()] = &[
            #(#test_names),*
        ];
    };
    let test_array_item: syn::Item = syn::parse2(test_array).unwrap();

    let mut items_to_add = vec![test_array_item];
    items_to_add.extend(visitor.tests.iter().cloned().map(syn::Item::Fn));

    if let Some((_, ref mut items)) = module.content {
        items.extend(items_to_add);
    } else {
        module.content = Some((syn::token::Brace::default(), items_to_add));
    }

    quote! {
        #module

        pub unsafe fn calyx_ffi_test(ffi: &mut CalyxFFI) {
            for test in #module_name::CALYX_FFI_TESTS {
                test(ffi);
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
