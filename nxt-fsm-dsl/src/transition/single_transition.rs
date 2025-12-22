use crate::{binding_args, Output, UsedTypes};

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use std::{collections::BTreeSet, iter::once};
use syn::{
	parse::{Parse, ParseStream, Result},
	token::Bracket,
	ExprClosure, Ident, Token,
};

pub struct SingleTransition {
	pub guard: Option<ExprClosure>,
	pub next_state: Ident,
	pub output: Option<Output>,
}

impl Parse for SingleTransition {
	fn parse(input: ParseStream) -> Result<Self> {
		let guard = input
			.peek(Token![if])
			.then(|| {
				let _ = input.parse::<Token![if]>()?;
				input.parse::<ExprClosure>()
			})
			.transpose()?;

		let _ = input.parse::<Token![=>]>()?;
		let next_state = input.parse::<Ident>()?;
		let output = input.peek(Bracket).then(|| input.parse::<Output>()).transpose()?;

		Ok(Self { guard, next_state, output })
	}
}
impl ToTokens for SingleTransition {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		if let Some(guard) = &self.guard {
			let bindings = binding_args(guard.inputs.len());
			quote! { if ( #guard )( #(#bindings),* ) }.to_tokens(tokens);
		}

		let output = self.output.as_ref().map(|output| quote! { Some(#output) }).unwrap_or_else(|| quote! { None });

		let next_state = &self.next_state;
		quote! { => Some( (Self::State::#next_state, #output ) ) }.to_tokens(tokens);
	}
}

impl UsedTypes for SingleTransition {
	fn outputs(&self) -> BTreeSet<&Ident> {
		self.output.as_ref().map(&Output::outputs).unwrap_or_default()
	}

	fn states(&self) -> BTreeSet<&Ident> {
		once(&self.next_state).collect()
	}
}
