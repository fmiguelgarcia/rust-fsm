//! DSL implementation for defining finite state machines for `rust-fsm`. See
//! more in the `rust-fsm` crate documentation.

#![recursion_limit = "128"]
extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use std::{collections::BTreeMap, collections::BTreeSet, iter::FromIterator};
use syn::{parse_macro_input, punctuated::Punctuated, Attribute, Ident, Type};

mod parser;

/// The full information about a state transition. Used to unify the
/// represantion of the simple and the compact forms.
struct Transition<'a> {
    initial_state: &'a Ident,
    input_value: &'a parser::InputVariant,
    final_state: &'a Ident,
    output: &'a Option<Ident>,
}

fn attrs_to_token_stream(attrs: Vec<Attribute>) -> proc_macro2::TokenStream {
    let attrs = attrs.into_iter().map(ToTokens::into_token_stream);
    proc_macro2::TokenStream::from_iter(attrs)
}

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

    let transitions = input.transitions.iter().flat_map(|def| {
        def.transitions.iter().map(move |transition| Transition {
            initial_state: &def.initial_state,
            input_value: &transition.input_value,
            final_state: &transition.final_state,
            output: &transition.output,
        })
    });

    let mut states = BTreeSet::new();
    let mut inputs: BTreeMap<&Ident, Option<&Punctuated<Type, syn::Token![,]>>> = BTreeMap::new();
    let mut outputs = BTreeSet::new();
    let mut transition_cases = Vec::new();
    let mut output_cases = Vec::new();

    #[cfg(feature = "diagram")]
    let mut mermaid_diagram = format!(
        "///```mermaid\n///stateDiagram-v2\n///    [*] --> {}\n",
        input.initial_state
    );

    states.insert(&input.initial_state);

    // Check if we're using a custom input type
    let using_custom_input = input.input_type.is_some();

    for transition in transitions {
        let Transition {
            initial_state,
            final_state,
            input_value,
            output,
        } = transition;

        let input_name = &input_value.name;

        #[cfg(feature = "diagram")]
        mermaid_diagram.push_str(&format!(
            "///    {initial_state} --> {final_state}: {input_name}"
        ));

        // Generate match cases
        // For generated input types, we know exactly which pattern to use based on the DSL
        // For custom input types, we only support unit variants (without tuple fields)
        let input_pattern = if using_custom_input || input_value.fields.is_none() {
            // For custom types, only support unit variant patterns
            // Tuple variants are not supported with custom input types
            quote! { Self::Input::#input_name }
        } else {
            // For generated tuple variants, use the (..) pattern
            quote! { Self::Input::#input_name(..) }
        };

        transition_cases.push(quote! {
            (Self::State::#initial_state, #input_pattern) => {
                Some(Self::State::#final_state)
            }
        });

        if let Some(output_value) = output {
            output_cases.push(quote! {
                (Self::State::#initial_state, #input_pattern) => {
                    Some(Self::Output::#output_value)
                }
            });

            #[cfg(feature = "diagram")]
            mermaid_diagram.push_str(&format!(" [{output_value}]"));
        }

        #[cfg(feature = "diagram")]
        mermaid_diagram.push('\n');

        states.insert(initial_state);
        states.insert(final_state);

        // Store input variant with its fields
        inputs
            .entry(input_name)
            .or_insert(input_value.fields.as_ref());

        if let Some(ref output) = output {
            outputs.insert(output);
        }
    }

    #[cfg(feature = "diagram")]
    mermaid_diagram.push_str("///```");
    #[cfg(feature = "diagram")]
    let mermaid_diagram: proc_macro2::TokenStream = mermaid_diagram.parse().unwrap();

    let initial_state_name = &input.initial_state;

    // Generate input variants with optional tuple fields
    let input_variants = inputs.iter().map(|(name, fields)| {
        if let Some(fields) = fields {
            quote! { #name(#fields) }
        } else {
            quote! { #name }
        }
    });

    let (input_type, input_impl) = match input.input_type {
        Some(t) => (quote!(#t), quote!()),
        None => (
            quote!(Input),
            quote! {
                #attrs
                pub enum Input {
                    #(#input_variants),*
                }
            },
        ),
    };

    let (state_type, state_impl) = match input.state_type {
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

    let (output_type, output_impl) = match input.output_type {
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

    #[cfg(feature = "diagram")]
    let diagram = quote! {
        #[cfg_attr(doc, ::rust_fsm::aquamarine)]
        #mermaid_diagram
    };

    #[cfg(not(feature = "diagram"))]
    let diagram = quote!();

    let output = quote! {
        #doc
        #diagram
        #visibility mod #fsm_name {
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
            }
        }
    };

    output.into()
}
