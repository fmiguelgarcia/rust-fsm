use crate::{parser, Transition};

use proc_macro2::TokenStream;
use quote::quote;
use std::collections::{BTreeMap, BTreeSet};
use syn::{parse_quote, Expr, File, Ident, Item};

static WRAP_FUNC_QED: &str = "Wrap function exists .qed";

/// Sanitize and format guard expressions for use in Mermaid diagrams.
///
/// When the `pretty-print` feature is enabled, this uses prettyplease to format
/// the expression in idiomatic Rust style. Otherwise, it uses basic sanitization
/// to make the expression safe for Mermaid.
pub fn sanitize_expr(expr: &Expr) -> String {
    // Wrap the expression in a function that returns it, so prettyplease can format it
    #[cfg(feature = "pretty-print")]
    let (content, start, end) = {
        let item: Item = parse_quote! {
          fn __guard_expr() -> bool {
            #expr
          }
        };
        let file = File {
            shebang: None,
            attrs: Vec::new(),
            items: vec![item],
        };

        let content = prettyplease::unparse(&file);
        // Extract just the expression part from the function body
        // The format will be: "fn __guard_expr() -> bool {\n    EXPR\n}\n"
        let start = content.find('{').expect(WRAP_FUNC_QED);
        let end = content.rfind('}').expect(WRAP_FUNC_QED);
        (content, start, end)
    };

    #[cfg(feature = "pretty-print")]
    let expr_str = content[start + 1..end].trim();

    #[cfg(not(feature = "pretty-print"))]
    let expr_str = quote! { #expr };

    // Mermaid fixes
    expr_str.replace(":", "")
}

pub fn build_diagram(initial_state: &Ident, transitions: &[Transition]) -> TokenStream {
    // Track transitions per state to detect nested states
    let mut transitions_per_state: BTreeMap<&Ident, Vec<&Transition>> = BTreeMap::new();

    // Group transitions by initial state
    for transition in transitions {
        transitions_per_state
            .entry(transition.initial_state)
            .or_default()
            .push(transition);
    }

    let mut diagram = format!(
        "///```mermaid\n///stateDiagram-v2\n///    [*] --> {}\n",
        initial_state
    );

    // Group transitions by (state, input_name) to detect guards
    let mut transitions_by_state_input: BTreeMap<(&Ident, &Ident), Vec<&Transition>> =
        BTreeMap::new();
    for transition in transitions {
        transitions_by_state_input
            .entry((transition.initial_state, &transition.input_value.name))
            .or_default()
            .push(transition);
    }

    // First: identify choice states for inputs with guards
    let mut choice_states_generated = BTreeSet::new();
    for ((state, input_name), input_transitions) in &transitions_by_state_input {
        // Check if this input has multiple transitions with guards
        let has_guards = input_transitions.iter().any(|t| t.guard.is_some());
        if has_guards && input_transitions.len() > 1 {
            choice_states_generated.insert((*state, *input_name));
        }
    }

    // Second: generate nested state definitions
    for (state, state_transitions) in &transitions_per_state {
        // Collect all self-referencing inputs (those that loop back to the same state)
        let self_loop_inputs: BTreeSet<&Ident> = state_transitions
            .iter()
            .filter(|t| t.final_state == *state)
            .map(|t| &t.input_value.name)
            .collect();

        // Check if this is a nested state (multiple self-loop inputs)
        if self_loop_inputs.len() > 1 {
            // Generate nested state representation
            diagram.push_str(&format!("///state {} {{\n", state));

            for input_name in &self_loop_inputs {
                diagram.push_str(&format!("///    [*] --> {}\n", input_name));

                // Check if this input has a choice state
                let key = (*state, *input_name);
                if choice_states_generated.contains(&key) {
                    // Transition to choice state
                    let choice_state_name = format!("{}_guard_{}", state, input_name);
                    diagram.push_str(&format!(
                        "///    {} --> {}\n",
                        input_name, choice_state_name
                    ));
                } else {
                    // Normal self-loop
                    diagram.push_str(&format!("///{} --> [*]\n", input_name));
                }
            }

            diagram.push_str("///}\n");
        }
    }

    // Third: generate choice states for inputs with guards
    for (state, input_name) in transitions_by_state_input.keys() {
        let key = (*state, *input_name);
        if choice_states_generated.contains(&key) {
            let choice_state_name = format!("{}_guard_{}", state, input_name);
            diagram.push_str(&format!("///state {} <<choice>>\n", choice_state_name));
        }
    }

    // Second pass: generate transitions between states
    for (state, state_transitions) in &transitions_per_state {
        // Collect all self-referencing inputs
        let self_loop_inputs: BTreeSet<&Ident> = state_transitions
            .iter()
            .filter(|t| t.final_state == *state)
            .map(|t| &t.input_value.name)
            .collect();

        // Check if this is a nested state
        let is_nested = self_loop_inputs.len() > 1;

        // Group transitions by input to handle guards
        let mut processed_inputs = BTreeSet::new();

        // Generate transitions to other states
        for transition in state_transitions.iter() {
            let input_name = &transition.input_value.name;
            let key = (*state, input_name);

            // Skip if already processed this input (for guarded transitions)
            if processed_inputs.contains(&key) {
                continue;
            }

            // Check if this input has a choice state
            if choice_states_generated.contains(&key) {
                let choice_state_name = format!("{}_guard_{}", state, input_name);
                let input_transitions = &transitions_by_state_input[&key];

                // Check if this input is part of a self-loop (any transition returns to same state)
                let has_self_loop = input_transitions.iter().any(|t| t.final_state == *state);

                // Only generate transition from state to choice state if:
                // - The state is NOT nested, OR
                // - The input doesn't have ANY self-loop (all transitions go to different states)
                // This avoids duplicating transitions that are already inside the nested state
                if !is_nested || !has_self_loop {
                    diagram.push_str(&format!("///    {} --> {}\n", state, choice_state_name));
                }

                // Generate transitions from choice state to final states
                for guarded_transition in input_transitions {
                    let guard_label = guarded_transition
                        .guard
                        .as_ref()
                        .map(|guard| {
                            let expr_str = sanitize_expr(&guard.expr);
                            format!(":{expr_str}")
                        })
                        .unwrap_or_default();

                    diagram.push_str(&format!(
                        "///    {} --> {}{}\n",
                        choice_state_name, guarded_transition.final_state, guard_label
                    ));
                }

                processed_inputs.insert(key);
            } else if transition.final_state != *state {
                // Show the input name as the transition label for non-guarded transitions
                diagram.push_str(&format!(
                    "///    {} --> {}: {}\n",
                    state, transition.final_state, input_name
                ));
                processed_inputs.insert(key);
            } else if !is_nested {
                // For single self-loops that aren't part of a nested state
                let fields_expr = if !transition.input_value.fields.is_empty() {
                    let fields = &transition.input_value.fields;
                    format!(" ({})", quote! { #fields })
                } else {
                    String::new()
                };
                let output_str =
                    if let Some(parser::OutputSpec::Constant(output_value)) = transition.output {
                        format!(" [{output_value}]")
                    } else {
                        String::new()
                    };
                diagram.push_str(&format!(
                    "///    {} --> {}: {}{}{}\n",
                    state, transition.final_state, input_name, fields_expr, output_str
                ));
                processed_inputs.insert(key);
            }
        }
    }

    diagram.push_str("///```");
    let diagram: TokenStream = diagram
        .parse()
        .inspect_err(|m_err| eprintln!("Mermaid diagram error: {m_err:?}\n\n{}", diagram))
        .expect("Mermaid diagram is invalid");

    quote! {
        #[cfg_attr(doc, ::rust_fsm::aquamarine)]
        #diagram
    }
}
