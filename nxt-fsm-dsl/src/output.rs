use crate::{binding_args, UsedTypes};

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use std::collections::BTreeSet;
use syn::{
	bracketed,
	parse::{Parse, ParseStream, Result},
	ExprClosure, Ident, Token,
};

/// The output of a state transition
pub enum Output {
	/// A constant output variant (e.g., [SetupTimer])
	Constant(Ident),
	/// A function call output (e.g., [|x| compute(x)])
	Call(ExprClosure),
}

impl Output {
	pub fn as_const(&self) -> Option<&Ident> {
		match &self {
			Output::Constant(id) => Some(id),
			Output::Call(..) => None,
		}
	}
}

impl ToTokens for Output {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		match self {
			Output::Constant(id) => {
				quote! { Self::Output::#id}.to_tokens(tokens);
			},
			Output::Call(closure) => {
				let bindings = binding_args(closure.inputs.len());
				quote! { (#closure)(#(#bindings),*) }.to_tokens(tokens);
			},
		}
	}
}

impl Parse for Output {
	fn parse(input: ParseStream) -> Result<Self> {
		let content;
		bracketed!(content in input);

		// Check if it starts with a closure (|)
		if content.peek(Token![|]) {
			let expr = content.parse::<ExprClosure>()?;
			return Ok(Self::Call(expr));
		}

		// Parse as constant identifier
		let ident: Ident = content.parse()?;
		Ok(Self::Constant(ident))
	}
}

impl UsedTypes for Output {
	fn outputs(&self) -> BTreeSet<&Ident> {
		self.as_const().into_iter().collect()
	}
}
