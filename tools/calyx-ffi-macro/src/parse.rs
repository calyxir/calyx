use std::path::PathBuf;

use proc_macro2::{Span, TokenTree};
use syn::{
    bracketed, parenthesized,
    parse::{Parse, ParseStream},
};

pub struct CalyxPortDeclaration(pub syn::Ident, pub syn::LitInt);

impl Parse for CalyxPortDeclaration {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse::<syn::Ident>()?;
        input.parse::<syn::Token![:]>()?;
        let width = input.parse::<syn::LitInt>()?;
        Ok(Self(name, width))
    }
}

pub struct CalyxInterface {
    pub name: syn::Ident,
    pub inputs: Vec<CalyxPortDeclaration>,
    pub outputs: Vec<CalyxPortDeclaration>,
}

impl Parse for CalyxInterface {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse::<syn::Ident>()?;
        let inputs;
        let outputs;
        parenthesized!(inputs in input);
        let inputs = inputs
            .parse_terminated(CalyxPortDeclaration::parse, syn::Token![,])?
            .into_iter()
            .collect();
        input.parse::<syn::Token![->]>()?;
        parenthesized!(outputs in input);
        let outputs = outputs
            .parse_terminated(CalyxPortDeclaration::parse, syn::Token![,])?
            .into_iter()
            .collect();
        Ok(Self {
            name,
            inputs,
            outputs,
        })
    }
}

pub struct CalyxFFIMacroArgs {
    pub src_attr_span: Span,
    pub src: PathBuf,
    pub comp_attr_span: Span,
    pub comp: String,
    pub backend: syn::Path,
    pub derives: Vec<CalyxInterface>,
}

impl Parse for CalyxFFIMacroArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        syn::custom_keyword!(src);
        syn::custom_keyword!(comp);
        syn::custom_keyword!(backend);
        syn::custom_keyword!(derive);

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

        let _ = input.parse::<syn::Token![,]>();

        let derives = if input.parse::<derive>().is_ok() {
            input.parse::<syn::Token![=]>()?;
            let content;
            bracketed!(content in input);
            content
                .parse_terminated(CalyxInterface::parse, syn::Token![,])?
                .into_iter()
                .collect()
        } else {
            vec![]
        };

        if !input.is_empty() {
            return Err(syn::Error::new_spanned(
                input.parse::<TokenTree>()?,
                "Invalid `calyx_ffi` argument syntax: expected 'src = \"...\", comp = \"...\", extern = ...",
            ));
        }

        Ok(Self {
            src_attr_span: src_ident.span,
            src: src_lit.into(),
            comp_attr_span: comp_ident.span,
            comp: comp_lit,
            backend: backend_path,
            derives,
        })
    }
}
