use std::path::PathBuf;

use proc_macro2::{Span, TokenTree};
use syn::parse::{Parse, ParseStream};

pub struct CalyxFFIMacroArgs {
    pub src_attr_span: Span,
    pub src: PathBuf,
    pub comp_attr_span: Span,
    pub comp: String,
    pub backend: syn::Path,
}

impl Parse for CalyxFFIMacroArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        syn::custom_keyword!(src);
        syn::custom_keyword!(comp);
        syn::custom_keyword!(backend);

        let src_ident = input.parse::<src>()?;
        input.parse::<syn::Token![=]>()?;
        let src_lit = input.parse::<syn::LitStr>()?.value();

        input.parse::<syn::Token![,]>()?;

        let comp_ident = input.parse::<comp>()?;
        input.parse::<syn::Token![=]>()?;
        let comp_lit = input.parse::<syn::LitStr>()?.value();

        input.parse::<syn::Token![,]>()?;
        input.parse::<backend>()?;
        input.parse::<syn::Token![=]>()?;
        let backend_path = input.parse::<syn::Path>()?;

        if !input.is_empty() {
            return Err(syn::Error::new_spanned(
                input.parse::<TokenTree>()?,
                "Invalid `calyx_ffi` argument syntax: expected 'src = \"...\", comp = \"...\", backend = ...",
            ));
        }

        Ok(Self {
            src_attr_span: src_ident.span,
            src: src_lit.into(),
            comp_attr_span: comp_ident.span,
            comp: comp_lit,
            backend: backend_path,
        })
    }
}
