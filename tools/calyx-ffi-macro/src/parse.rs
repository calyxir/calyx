use std::path::PathBuf;

use proc_macro2::{Span, TokenTree};
use syn::parse::{Parse, ParseStream};

pub struct CalyxFFIMacroArgs {
    pub src_attr_span: Span,
    pub src: PathBuf,
    pub comp_attr_span: Span,
    pub comp: String,
}

impl Parse for CalyxFFIMacroArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        syn::custom_keyword!(src);
        syn::custom_keyword!(comp);

        let src_ident = input.parse::<src>()?;
        input.parse::<syn::Token![=]>()?;
        let src_lit = input.parse::<syn::LitStr>()?.value();

        input.parse::<syn::Token![,]>()?;

        let comp_ident = input.parse::<comp>()?;
        input.parse::<syn::Token![=]>()?;
        let comp_lit = input.parse::<syn::LitStr>()?.value();

        if !input.is_empty() {
            return Err(syn::Error::new_spanned(
                input.parse::<TokenTree>()?,
                "Invalid `calyx_ffi` attribute syntax: expected 'src = \"...\", comp = \"...\"",
            ));
        }

        Ok(Self {
            src_attr_span: src_ident.span,
            src: src_lit.into(),
            comp_attr_span: comp_ident.span,
            comp: comp_lit,
        })
    }
}
