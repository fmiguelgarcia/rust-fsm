//! DSL implementation for defining finite state machines for `rust-fsm`. See
//! more in the `rust-fsm` crate documentation.
#![recursion_limit = "128"]
extern crate proc_macro;

mod event;
mod output;
mod sm_def_attr;
mod state_def;
mod state_machine_def;
mod transition;

use event::Event;
use output::Output;
use sm_def_attr::SMDefAttr;
use state_def::StateDef;
use state_machine_def::StateMachineDef;
use transition::Transition;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::collections::BTreeSet;
use syn::{parse_macro_input, Ident};

#[cfg(feature = "diagram")]
mod diagram;

#[proc_macro]
/// Produce a state machine definition from the provided `rust-fmt` DSL
/// description.
pub fn state_machine(tokens: TokenStream) -> TokenStream {
	let sm_def = parse_macro_input!(tokens as StateMachineDef);

	quote! { #sm_def }.into()
}

pub(crate) fn binding_args(count: usize) -> Vec<Ident> {
	(0..count).map(|idx| format_ident!("__arg{idx}")).collect()
}

pub(crate) trait UsedTypes {
	fn outputs(&self) -> BTreeSet<&Ident> {
		BTreeSet::new()
	}

	fn states(&self) -> BTreeSet<&Ident> {
		BTreeSet::new()
	}

	fn inputs(&self) -> BTreeSet<&Event> {
		BTreeSet::new()
	}
}
