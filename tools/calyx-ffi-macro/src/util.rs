use proc_macro::TokenStream;
use proc_macro2::Span;

pub fn compile_error(span: &Span, msg: String) -> TokenStream {
    syn::Error::new(*span, msg).to_compile_error().into()
}
