use crate::{transition::SubTransition, UsedTypes};

use std::collections::BTreeSet;
use syn::{
	braced, parenthesized,
	parse::{Parse, ParseStream, Result},
	token::{Comma, Paren},
	Ident, Token,
};

pub struct MultipleTransition {
	pub bindings: Vec<Ident>,
	pub sub_transitions: Vec<SubTransition>,
}

impl Parse for MultipleTransition {
	fn parse(input: ParseStream) -> Result<Self> {
		let _ = input.parse::<Token![match]>()?;

		let bindings = if input.peek(Paren) {
			let content;
			parenthesized!(content in input);
			let bindings = content.parse_terminated(Ident::parse, Comma)?;
			bindings.into_iter().collect()
		} else {
			// Just one binding, so paren are optional
			vec![input.parse::<Ident>()?]
		};

		let match_content;
		braced!(match_content in input);
		let sub_transitions = match_content.parse_terminated(SubTransition::parse, Comma)?;
		let sub_transitions = sub_transitions.into_iter().collect();

		Ok(Self { bindings, sub_transitions })
	}
}

impl UsedTypes for MultipleTransition {
	fn outputs(&self) -> BTreeSet<&Ident> {
		self.sub_transitions.iter().flat_map(SubTransition::outputs).collect()
	}

	fn states(&self) -> BTreeSet<&Ident> {
		self.sub_transitions.iter().flat_map(SubTransition::states).collect()
	}
}
