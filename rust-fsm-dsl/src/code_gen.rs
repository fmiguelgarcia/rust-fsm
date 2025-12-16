use crate::parser::InputVariant;

use proc_macro2::Span;
use syn::{Attribute, Ident, Type};

use quote::ToTokens;

/// Generate parameter names: __arg0, __arg1, etc.
pub fn input_param_names(input: &InputVariant) -> Vec<Ident> {
    (0..input.fields.len())
        .map(|i| Ident::new(&format!("__arg{i}"), Span::call_site()))
        .collect()
}

/// Check if a type is a reference type (e.g., `&str`, `&[u8]`, `&'a T`).
///
/// This is used to avoid adding an extra `&` when generating guard parameters
/// for types that are already references.
pub fn is_reference_type(ty: &Type) -> bool {
    matches!(ty, Type::Reference(_))
}

/// Convert a vector of attributes into a token stream.
///
/// This transforms each attribute into its token representation and combines them
/// into a single token stream for code generation.
pub fn attrs_to_token_stream(attrs: Vec<Attribute>) -> proc_macro2::TokenStream {
    let attrs = attrs.into_iter().map(ToTokens::into_token_stream);
    proc_macro2::TokenStream::from_iter(attrs)
}
