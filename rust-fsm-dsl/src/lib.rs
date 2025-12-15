//! DSL implementation for defining finite state machines for `rust-fsm`. See
//! more in the `rust-fsm` crate documentation.

#![recursion_limit = "128"]
extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use std::collections::BTreeSet;
use syn::parse_macro_input;

mod code_gen;
use code_gen::attrs_to_token_stream;
mod match_case;
use match_case::MatchCase;
mod parser;
use parser::TransitionDef;

use crate::parser::InputVariant;

#[cfg(feature = "diagram")]
mod diagram;

#[proc_macro]
/// Produce a state machine definition from the provided `rust-fmt` DSL
/// description.
pub fn state_machine(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as parser::StateMachineDef);

    let doc = attrs_to_token_stream(input.doc);
    let attrs = attrs_to_token_stream(input.attributes);

    if input.transitions.is_empty() {
        let output = quote! {
            compile_error!("rust-fsm: at least one state transition must be provided");
        };
        return output.into();
    }

    let fsm_name = input.name;
    let visibility = input.visibility;

    // First, expand guards into separate transition entries
    let match_cases: Vec<_> = input
        .transitions
        .iter()
        .flat_map(TransitionDef::as_match_cases)
        .collect();

    // Types: State, Input, and Output
    let mut states = match_cases
        .iter()
        .flat_map(|mc| [mc.state, mc.next_state])
        .collect::<BTreeSet<_>>();
    states.insert(&input.initial_state);

    // Inputs: Remove duplicated variants due to multiple match cases.
    let mut inputs = match_cases.iter().map(|mc| mc.input).collect::<Vec<_>>();
    inputs.sort_by_key(|i| &i.name);
    inputs.dedup_by_key(|i| &i.name);
    let inputs = inputs
        .into_iter()
        .map(InputVariant::code_gen)
        .collect::<Vec<_>>();

    let outputs = match_cases
        .iter()
        .filter_map(MatchCase::constant_output)
        .collect::<BTreeSet<_>>();

    // Transitions & outpus
    let transition_cases = match_cases
        .iter()
        .map(MatchCase::code_gen_transition_case)
        .collect::<Vec<_>>();
    let output_cases = match_cases
        .iter()
        .filter_map(MatchCase::code_gen_output_case)
        .collect::<Vec<_>>();

    #[cfg(feature = "diagram")]
    let diagram = diagram::build_diagram(&input.initial_state, &match_cases);
    #[cfg(not(feature = "diagram"))]
    let diagram = quote!();

    let initial_state_name = &input.initial_state;

    let (input_type, input_impl) = match input.def_attrs.input_type {
        Some(t) => (quote!(#t), quote!()),
        None => (
            quote!(Input),
            quote! {
                #attrs
                pub enum Input {
                    #(#inputs),*
                }
            },
        ),
    };

    let (state_type, state_impl) = match input.def_attrs.state_type {
        Some(t) => (quote!(#t), quote!()),
        None => (
            quote!(State),
            quote! {
                #attrs
                pub enum State {
                    #(#states),*
                }
            },
        ),
    };

    let (output_type, output_impl) = match input.def_attrs.output_type {
        Some(t) => (quote!(#t), quote!()),
        None => {
            // Many attrs and derives may work incorrectly (or simply not work) for empty enums, so we just skip them
            // altogether if the output alphabet is empty.
            let attrs = if outputs.is_empty() {
                quote!()
            } else {
                attrs.clone()
            };
            (
                quote!(Output),
                quote! {
                    #attrs
                    pub enum Output {
                        #(#outputs),*
                    }
                },
            )
        }
    };

    // Collect use statements
    let use_statements = &input.use_statements;

    // Generate before_transition implementation if provided
    let before_transition_impl = if let Some(ref expr) = input.def_attrs.before_transition {
        quote! {
            fn before_transition(state: &Self::State, input: &Self::Input) {
                (#expr)(state, input)
            }
        }
    } else {
        quote!()
    };

    // Generate after_transition implementation if provided
    let after_transition_impl = if let Some(ref expr) = input.def_attrs.after_transition {
        quote! {
            fn after_transition(
                pre_state: &Self::State,
                input: &Self::Input,
                state: &Self::State,
                output: Option<&Self::Output>,
            ) {
                (#expr)(pre_state, input, state, output)
            }
        }
    } else {
        quote!()
    };

    let output = quote! {
        #doc
        #diagram
        #visibility mod #fsm_name {
            #(#use_statements)*

            #attrs
            pub struct Impl;

            pub type StateMachine = ::rust_fsm::StateMachine<Impl>;

            #input_impl
            #state_impl
            #output_impl

            impl ::rust_fsm::StateMachineImpl for Impl {
                type Input = #input_type;
                type State = #state_type;
                type Output = #output_type;
                const INITIAL_STATE: Self::State = Self::State::#initial_state_name;

                fn transition(state: &Self::State, input: &Self::Input) -> Option<Self::State> {
                    match (state, input) {
                        #(#transition_cases)*
                        _ => None,
                    }
                }

                fn output(state: &Self::State, input: &Self::Input) -> Option<Self::Output> {
                    match (state, input) {
                        #(#output_cases)*
                        _ => None,
                    }
                }

                #before_transition_impl
                #after_transition_impl
            }
        }
    };

    output.into()
}
