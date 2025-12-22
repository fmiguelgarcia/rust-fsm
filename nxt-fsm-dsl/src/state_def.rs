use crate::{Event, Transition, UsedTypes};

use proc_macro2::TokenStream;
use quote::ToTokens;
use std::collections::BTreeSet;
use syn::{
	braced,
	parse::{Parse, ParseStream, Result},
	punctuated::Punctuated,
	token::Comma,
	Ident, Token,
};

/// Parses the transition in any of the possible formats.
pub struct StateDef {
	pub state: Ident,
	pub transitions: Vec<Transition>,
}

impl StateDef {
	pub fn new(state: Ident, mut transitions: Vec<Transition>) -> Self {
		transitions.iter_mut().for_each(|tn| tn.parent_state = Some(state.clone()));

		Self { state, transitions }
	}

	#[cfg(feature = "diagram")]
	pub fn diagram(&self) -> Vec<String> {
		self.transitions.iter().flat_map(Transition::diagram).collect()
	}
}
impl ToTokens for StateDef {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		self.transitions.iter().for_each(|tn| tn.to_tokens(tokens));
	}
}

impl Parse for StateDef {
	/// Parses a complete transition definition.
	///
	/// # Examples:
	///  ```ignore
	///  State_1 (Event_1) => State_2 [Output],
	///  State_1 (Event_3(u32) if |age| age >= 18) => State_4,
	///  State_2 => {
	///       Event_2 => State_1,
	///       Event_3 match age {
	///           0..18 => State_4,
	///           18.. => State_5,
	///       }
	///   }
	///  }
	///  ```
	fn parse(ps: ParseStream) -> Result<Self> {
		let state = ps.parse()?;
		let transitions = if ps.peek(Token![=>]) {
			let _ = ps.parse::<Token![=>]>()?;
			let content;
			braced!(content in ps);
			let tns = Punctuated::<Transition, Comma>::parse_terminated(&content)?;

			tns.into_iter().collect()
		} else {
			vec![Transition::parse(ps)?]
		};

		Ok(Self::new(state, transitions))
	}
}

impl UsedTypes for StateDef {
	fn outputs(&self) -> BTreeSet<&Ident> {
		self.transitions.iter().flat_map(&Transition::outputs).collect()
	}

	fn states(&self) -> BTreeSet<&Ident> {
		let mut states = self.transitions.iter().flat_map(&Transition::states).collect::<BTreeSet<_>>();
		states.insert(&self.state);
		states
	}

	fn inputs(&self) -> BTreeSet<&Event> {
		self.transitions.iter().flat_map(&Transition::inputs).collect()
	}
}
