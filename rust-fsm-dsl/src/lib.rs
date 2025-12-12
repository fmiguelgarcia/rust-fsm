//! DSL implementation for defining finite state machines for `rust-fsm`. See
//! more in the `rust-fsm` crate documentation.

#![recursion_limit = "128"]
extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use std::{collections::BTreeMap, collections::BTreeSet, iter::FromIterator};
use syn::{parse_macro_input, punctuated::Punctuated, token::Comma, Attribute, Expr, Ident, Type};

use crate::parser::{InputVariant, OutputSpec};

mod parser;

#[cfg(feature = "diagram")]
mod diagram;

/// The full information about a state transition. Used to unify the
/// represantion of the simple and the compact forms.
#[derive(Clone)]
struct Transition<'a> {
    initial_state: &'a Ident,
    input_value: &'a InputVariant,
    guard_expr: Option<Expr>,
    final_state: &'a Ident,
    output: Option<&'a OutputSpec>,
}

impl<'a> Transition<'a> {
    pub fn new(
        init: &'a Ident,
        input: &'a InputVariant,
        final_s: &'a Ident,
        output: Option<&'a OutputSpec>,
    ) -> Self {
        Self {
            initial_state: init,
            input_value: input,
            guard_expr: None,
            final_state: final_s,
            output,
        }
    }
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

    // First, expand guards into separate transition entries
    let transitions: Vec<_> = input
        .transitions
        .iter()
        .flat_map(|def| {
            def.transitions
                .iter()
                .flat_map(move |transition| {
                    // Handle guards differently based on their type
                    match &transition.guard {
                        // Match guard with multiple arms - each arm becomes a separate transition
                        Some(parser::Guard::Binding(binding_guard)) => {
                            binding_guard
                                .arms
                                .iter()
                                .map(|arm| {
                                    let guard_expr = binding_guard.guard_expr_for_arm(&transition.input.fields, arm);
                                    Transition {
                                        initial_state: &def.initial_state,
                                        input_value: &transition.input,
                                        guard_expr: Some(guard_expr),
                                        final_state: &arm.final_state,
                                        output: arm.output.as_ref(),
                                    }
                                })
                                .collect::<Vec<_>>()
                        }
                        // If guard with closure - single transition with guard
                        Some(parser::Guard::Closure(closure_expr)) => {
                            vec![Transition {
                                initial_state: &def.initial_state,
                                input_value: &transition.input,
                                guard_expr: Some(closure_expr.clone()),
                                final_state: transition.final_state.as_ref().expect("if guard must have final_state"),
                                output: transition.output.as_ref(),
                            }]
                        }
                        // No guard - single transition without guard
                        None => {
                            vec![Transition::new(
                                &def.initial_state,
                                &transition.input,
                                transition.final_state.as_ref().expect("simple transition must have final_state"),
                                transition.output.as_ref(),
                            )]
                        }
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect();

    let mut states = BTreeSet::new();
    let mut inputs: BTreeMap<&Ident, &Punctuated<Type, Comma>> = BTreeMap::new();
    let mut outputs = BTreeSet::new();
    let mut transition_cases = Vec::new();
    let mut output_cases = Vec::new();

    states.insert(&input.initial_state);

    // Check if we're using a custom input type
    let using_custom_input = input.input_type.is_some();

    for transition in &transitions {
        let Transition {
            initial_state,
            final_state,
            input_value,
            guard_expr: guard_expr_opt,
            output,
        } = transition;

        let input_name = &input_value.name;

        // Generate match cases
        // For generated input types, we know exactly which pattern to use based on the DSL
        // For custom input types, we only support unit variants (without tuple fields)

        // When there's a guard with tuple variant, we need to bind the fields
        let (input_pattern, guard_code) = if let Some(guard_expr) = guard_expr_opt {
            let param_names = input_param_names(input_value);

            if param_names.is_empty() {
                (
                    quote! { Self::Input::#input_name },
                    quote! { if #guard_expr },
                )
            } else {
                (
                    quote! { Self::Input::#input_name(#(ref #param_names),*) },
                    quote! { if (#guard_expr)(#(#param_names),*) },
                )
            }
        } else {
            // No guard
            let pattern = if using_custom_input || input_value.fields.is_empty() {
                // For custom types or unit variants
                quote! { Self::Input::#input_name }
            } else {
                // For generated tuple variants without guard, use (..) pattern
                quote! { Self::Input::#input_name(..) }
            };
            (pattern, proc_macro2::TokenStream::new())
        };

        transition_cases.push(quote! {
          (Self::State::#initial_state, #input_pattern) #guard_code => {
            Some(Self::State::#final_state)
          },
        });

        if let Some(output_spec) = output {
            match output_spec {
                parser::OutputSpec::Constant(output_value) => {
                    output_cases.push(quote! {
                      (Self::State::#initial_state, #input_pattern) #guard_code => {
                        Some(Self::Output::#output_value)
                      },
                    });
                }
                parser::OutputSpec::Call(call_expr) => {
                    // Generate code to call the closure with input tuple fields
                    let param_names = input_param_names(input_value);

                    let (case_expr, param_names_expr) = if !param_names.is_empty() {
                        // Pattern to destructure the input with references
                        let pattern_for_call =
                            quote! { Self::Input::#input_name(#(ref #param_names),*) };

                        (
                            quote! { (Self::State::#initial_state, #pattern_for_call) #guard_code },
                            quote! { (#(#param_names),*) },
                        )
                    } else {
                        // Unit variant - call closure without arguments
                        (
                            quote! { (Self::State::#initial_state, #input_pattern) #guard_code  },
                            proc_macro2::TokenStream::new(),
                        )
                    };

                    output_cases
                        .push(quote! { #case_expr => { Some((#call_expr) #param_names_expr) }, });
                }
            }
        }

        states.insert(initial_state);
        states.insert(final_state);

        // Store input variant with its fields
        inputs.entry(input_name).or_insert(&input_value.fields);

        // Only collect constant outputs for the Output enum
        if let Some(parser::OutputSpec::Constant(ref output_ident)) = output {
            outputs.insert(output_ident);
        }
    }

    #[cfg(feature = "diagram")]
    let diagram = diagram::build_diagram(&input.initial_state, &transitions);
    #[cfg(not(feature = "diagram"))]
    let diagram = quote!();

    let initial_state_name = &input.initial_state;

    // Generate input variants with optional tuple fields
    let input_variants = inputs.iter().map(|(name, fields)| {
        if !fields.is_empty() {
            quote! { #[allow(unused)] #name(#fields) }
        } else {
            quote! { #[allow(unused)] #name }
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

    // Collect use statements
    let use_statements = &input.use_statements;

    // Generate before_transition implementation if provided
    let before_transition_impl = if let Some(ref expr) = input.before_transition {
        quote! {
            fn before_transition(state: &Self::State, input: &Self::Input) {
                (#expr)(state, input)
            }
        }
    } else {
        quote!()
    };

    // Generate after_transition implementation if provided
    let after_transition_impl = if let Some(ref expr) = input.after_transition {
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

/// Generate parameter names: __arg0, __arg1, etc.
fn input_param_names(input: &parser::InputVariant) -> Vec<Ident> {
    (0..input.fields.len())
        .map(|i| Ident::new(&format!("__arg{i}"), proc_macro2::Span::call_site()))
        .collect()
}
