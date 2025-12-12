use crate::parser::{InputFields, MatchArm};

use syn::{
    braced, parenthesized,
    parse::{Parse, ParseStream, Result},
    punctuated::Punctuated,
    token::Paren,
    Expr, Ident, Token,
};

use std::iter::once;

pub struct BindingGuard {
    pub bindings: Punctuated<Ident, Token![,]>,
    pub arms: Vec<MatchArm>,
}

impl BindingGuard {
    /// Generate guard expression for a specific arm
    pub fn guard_expr_for_arm(&self, fields: &InputFields, arm: &MatchArm) -> Expr {
        assert_eq!(
            fields.len(),
            self.bindings.len(),
            "Number of bindings ({}) must match number of fields ({})",
            self.bindings.len(),
            fields.len()
        );

        let pattern = &arm.pattern;
        
        // Pair each binding with its corresponding field type
        let params = self.bindings.iter().zip(fields.iter()).map(|(binding, field_type)| {
            quote::quote! { #binding: &#field_type }
        });
        
        let bindings = &self.bindings.iter().collect::<Vec<_>>();

        // Generate a closure that takes individual bindings (not as a tuple)
        // and checks if they match the pattern
        syn::parse_quote! {
            |#(#params),*| matches!((#(#bindings),*), #pattern)
        }
    }
}

impl Parse for BindingGuard {
    fn parse(input: ParseStream) -> Result<Self> {
        let lh = input.lookahead1();

        // Parse bindings: single or multiple
        let bindings = if lh.peek(Paren) {
            // Parse tuple: (binding1, binding2, ...)
            let content;
            parenthesized!(content in input);
            content.parse_terminated(Ident::parse, Token![,])?
        } else
        /*(if lh.peek(Ident)*/
        {
            // Parse single identifier
            let single_binding: Ident = input.parse()?;
            Punctuated::from_iter(once(single_binding))
        };

        // Parse match arms.
        let arms_content;
        braced!(arms_content in input);
        let arms = arms_content
            .parse_terminated(MatchArm::parse, Token![,])?
            .into_iter()
            .collect::<Vec<_>>();

        Ok(Self { bindings, arms })
    }
}

/// Represents a guard expression for a transition
/// Can represent both 'if' guards (single closure) and 'match' guards (multiple arms)
pub enum Guard {
    /// Match guard with bindings and multiple arms, each leading to different states
    Binding(BindingGuard),
    /// If guard with a single closure expression
    Closure(Expr),
}

impl Parse for Guard {
    fn parse(input: ParseStream) -> Result<Self> {
        let lh = input.lookahead1();
        if lh.peek(Token![if]) {
            input.parse::<Token![if]>()?;
        } else if lh.peek(Token![match]) {
            input.parse::<Token![match]>()?;
        } else {
            return Err(lh.error());
        }

        let lh = input.lookahead1();
        let this = if lh.peek(Token![|]) {
            Self::Closure(input.parse::<Expr>()?)
        } else {
            Self::Binding(input.parse::<BindingGuard>()?)
        };

        Ok(this)
    }
}
