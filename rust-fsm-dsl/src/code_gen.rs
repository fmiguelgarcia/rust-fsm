use crate::parser::InputVariant;

use proc_macro2::Span;
use syn::{Attribute, Ident};

use quote::ToTokens;

/// Generate parameter names: __arg0, __arg1, etc.
pub fn input_param_names(input: &InputVariant) -> Vec<Ident> {
    (0..input.fields.len())
        .map(|i| Ident::new(&format!("__arg{i}"), Span::call_site()))
        .collect()
}

/// Convert a vector of attributes into a token stream.
///
/// This transforms each attribute into its token representation and combines them
/// into a single token stream for code generation.
pub fn attrs_to_token_stream(attrs: Vec<Attribute>) -> proc_macro2::TokenStream {
    let attrs = attrs.into_iter().map(ToTokens::into_token_stream);
    proc_macro2::TokenStream::from_iter(attrs)
}
