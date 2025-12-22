use crate::{SMDefAttr, StateDef, UsedTypes};

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use std::collections::BTreeSet;
use syn::{
	parenthesized,
	parse::{Parse, ParseStream, Result},
	punctuated::Punctuated,
	token::Comma,
	Attribute, Ident, ItemUse, Token, Visibility,
};

/// Parses the whole state machine definition in the following form (example):
///
/// ```rust,ignore
/// state_machine! {
///     CircuitBreaker(Closed)
///
///     Closed(Unsuccessful) => Open [SetupTimer],
///     Open(TimerTriggered) => HalfOpen,
///     HalfOpen => {
///         Successful => Closed,
///         Unsuccessful => Open [SetupTimer]
///     },
/// }
/// ```
pub struct StateMachineDef {
	pub doc_attrs: Vec<Attribute>,
	pub type_attrs: Vec<Attribute>,
	pub sm_attrs: SMDefAttr,
	/// The visibility modifier (applies to all generated items)
	pub visibility: Visibility,
	pub name: Ident,
	pub initial_state: Ident,
	pub use_statements: Vec<ItemUse>,
	pub states: Vec<StateDef>,
}

impl StateMachineDef {
	fn input_to_tokens(&self) -> (TokenStream, TokenStream) {
		match &self.sm_attrs.input_type {
			Some(input_type) => (input_type.into_token_stream(), TokenStream::new()),
			None => {
				let type_attrs = &self.type_attrs;
				let events = self.states.iter().flat_map(StateDef::inputs).collect::<BTreeSet<_>>();

				let type_def = quote! {
				  #(#type_attrs)*
				  pub enum Input {
						#(#events),*
				  }
				};

				(quote!(Input), type_def)
			},
		}
	}

	fn state_to_tokens(&self) -> (TokenStream, TokenStream) {
		match &self.sm_attrs.state_type {
			Some(state_type) => (state_type.into_token_stream(), TokenStream::new()),
			None => {
				let type_attrs = &self.type_attrs;
				let states = self.states.iter().flat_map(StateDef::states).collect::<BTreeSet<_>>();

				let type_def = quote! {
				  #(#type_attrs)*
				  pub enum State {
					  #(#states),*
				  }
				};
				(quote!(State), type_def)
			},
		}
	}

	fn output_to_tokens(&self) -> (TokenStream, TokenStream) {
		match &self.sm_attrs.output_type {
			Some(output_type) => (output_type.into_token_stream(), TokenStream::new()),
			None => {
				let type_attrs = &self.type_attrs;
				let outputs = self.states.iter().flat_map(&StateDef::outputs).collect::<BTreeSet<_>>();

				let type_def = quote! {
				  #(#type_attrs)*
				  pub enum Output {
					  #(#outputs),*
				  }
				};
				(quote!(Output), type_def)
			},
		}
	}
}

impl Parse for StateMachineDef {
	fn parse(input: ParseStream) -> Result<Self> {
		// Parse attributes: doc, state_machine, and others
		let attributes = Attribute::parse_outer(input)?;
		let (doc_attrs, attributes) = attributes.into_iter().partition::<Vec<_>, _>(|attr| attr.path().is_ident("doc"));
		let (sm_attrs, type_attrs) =
			attributes.into_iter().partition::<Vec<_>, _>(|attr| attr.path().is_ident("state_machine"));
		let sm_attrs = SMDefAttr::from_iter(sm_attrs);

		let visibility = input.parse()?;
		let name = input.parse()?;

		// Parse **initial** state.
		let initial_state_content;
		parenthesized!(initial_state_content in input);
		let initial_state = initial_state_content.parse()?;

		// Parse optional use statements
		let mut use_statements = Vec::new();
		while input.peek(Token![use]) {
			use_statements.push(input.parse()?);
		}

		let states = Punctuated::<StateDef, Comma>::parse_terminated(input)?;
		let states = states.into_iter().collect();

		Ok(Self { doc_attrs, visibility, name, initial_state, use_statements, states, type_attrs, sm_attrs })
	}
}

impl ToTokens for StateMachineDef {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		let fsm_name = &self.name;
		let visibility = &self.visibility;
		let doc_attrs = &self.doc_attrs;
		let type_attrs = &self.type_attrs;
		let initial_state = &self.initial_state;
		let use_statements = &self.use_statements;

		let before_transition_impl = self.sm_attrs.before_transition_to_tokens();
		let after_transition_impl = self.sm_attrs.after_transition_to_tokens();
		let (input_type, input_impl) = self.input_to_tokens();
		let (state_type, state_impl) = self.state_to_tokens();
		let (output_type, output_impl) = self.output_to_tokens();
		let transition_cases = &self.states;

		#[cfg(feature = "diagram")]
		let diagram = self.diagram();
		#[cfg(not(feature = "diagram"))]
		let diagram = quote!();

		quote! {
		  #(#doc_attrs)*
		  #diagram
		  #visibility mod #fsm_name {
			#(#use_statements)*

			#(#type_attrs)*
			pub struct Impl;

			pub type StateMachine = ::nxt_fsm::StateMachine<Impl>;

			#input_impl
			#state_impl
			#output_impl


			impl ::nxt_fsm::StateMachineImpl for Impl {
				type Input<'__input_lifetime> = #input_type;
				type State = #state_type;
				type Output = #output_type;

				const INITIAL_STATE: Self::State = Self::State::#initial_state;

				fn transition<'__input_lifetime>(state: &Self::State, input: &Self::Input<'__input_lifetime>) -> Option<(Self::State, Option<Self::Output>)> {
					match (state, input) {
						#(#transition_cases)*
						_ => None
					}
				}

				#before_transition_impl
				#after_transition_impl
			}
		  }
		}.to_tokens(tokens);
	}
}

#[cfg(feature = "diagram")]
impl StateMachineDef {
	fn diagram(&self) -> TokenStream {
		let mut diagram = String::with_capacity(2048);

		// Headers & initial state
		diagram.push_str("///```mermaid\n");
		diagram.push_str("///stateDiagram-v2\n");
		diagram.push_str(&format!("///    [*] --> {}\n", self.initial_state));

		// Transitions
		self.states.iter().flat_map(&StateDef::diagram).for_each(|line| diagram.push_str(&line));

		// Close mermaid
		diagram.push_str("///```");

		// Generate code as comment
		let diagram: TokenStream = diagram.parse().expect("Mermaid diagram is invalid");
		quote! {
			#[cfg_attr(doc, ::nxt_fsm::aquamarine)]
			#diagram
		}
	}
}
