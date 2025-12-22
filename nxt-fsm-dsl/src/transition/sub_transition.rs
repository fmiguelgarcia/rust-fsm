use crate::{Output, UsedTypes};

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use std::{collections::BTreeSet, iter::once};
use syn::{
	parse::{Parse, ParseStream, Result},
	token::Bracket,
	Expr, Ident, Token,
};

/// Represents a single arm in a guard expression
#[allow(dead_code)]
pub struct SubTransition {
	pub pattern: Expr,
	pub next_state: Ident,
	pub output: Option<Output>,
}

impl Parse for SubTransition {
	fn parse(input: ParseStream) -> Result<Self> {
		let pattern = input.parse()?;
		input.parse::<Token![=>]>()?;
		let next_state = input.parse()?;
		let output = input.peek(Bracket).then(|| input.parse::<Output>()).transpose()?;

		Ok(Self { pattern, next_state, output })
	}
}

impl ToTokens for SubTransition {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		let pattern = &self.pattern;
		let next_state = &self.next_state;
		let output = self.output.as_ref().map(|output| quote! { Some(#output) }).unwrap_or_else(|| quote! { None });

		quote! {
			#pattern => Some( (Self::State::#next_state, #output) )
		}
		.to_tokens(tokens);
	}
}

impl UsedTypes for SubTransition {
	fn outputs(&self) -> BTreeSet<&Ident> {
		self.output.iter().flat_map(&Output::outputs).collect()
	}

	fn states(&self) -> BTreeSet<&Ident> {
		once(&self.next_state).collect()
	}
}
