use crate::parser::{Guard, InputVariant, Output, OutputSpec};

use syn::{
    braced, parenthesized,
    parse::{Error, Parse, ParseStream, Result},
    token::Paren,
    Ident, Token,
};

/// Represents a part of state transition without the initial state. The `Parse`
/// trait is implemented for the compact form.
pub struct TransitionEntry {
    pub input: InputVariant,
    pub guard: Option<Guard>,
    /// Only present for simple transitions or `if` guards (not for `match` guards)
    pub final_state: Option<Ident>,
    /// Only present for simple transitions or `if` guards (not for `match` guards)
    pub output: Option<OutputSpec>,
}

impl Parse for TransitionEntry {
    fn parse(ps: ParseStream) -> Result<Self> {
        let input = InputVariant::parse(ps)?;

        // Check for optional guard: if binding { pattern } or match binding { arms }
        let guard = if ps.peek(Token![if]) || ps.peek(Token![match]) {
            Some(ps.parse::<Guard>()?)
        } else {
            None
        };

        // For match guards, the final states are in the arms; no => after the guard
        let (final_state, output) = match &guard {
            Some(Guard::Binding(_)) => {
                // Match guard - no final state or output here
                (None, None)
            }
            _ => {
                // Simple transition or if guard - expect => final_state [output]
                ps.parse::<Token![=>]>()?;
                let final_state = ps.parse::<Ident>()?;
                let output = ps.parse::<Output>()?.into();
                (Some(final_state), output)
            }
        };

        Ok(Self {
            input,
            guard,
            final_state,
            output,
        })
    }
}

/// Parses the transition in any of the possible formats.
pub struct TransitionDef {
    pub initial_state: Ident,
    pub transitions: Vec<TransitionEntry>,
}

impl Parse for TransitionDef {
    fn parse(input: ParseStream) -> Result<Self> {
        let initial_state = input.parse()?;
        // Parse the transition in the simple format
        // InitialState(Input) => ResultState [Output]
        // Note: Guards are not supported in simple format (only in compact format)
        let transitions = if input.lookahead1().peek(Paren) {
            let input_content;
            parenthesized!(input_content in input);
            let input_value = InputVariant::parse(&input_content)?;
            input.parse::<Token![=>]>()?;
            let final_state = input.parse()?;
            let output = input.parse::<Output>()?.into();

            vec![TransitionEntry {
                input: input_value,
                guard: None,
                final_state,
                output,
            }]
        } else {
            // Parse the transition in the compact format
            // InitialState => {
            //     Input1 => State1,
            //     Input2 => State2 [Output]
            // }
            input.parse::<Token![=>]>()?;
            let entries_content;
            braced!(entries_content in input);

            let entries: Vec<_> = entries_content
                .parse_terminated(TransitionEntry::parse, Token![,])?
                .into_iter()
                .collect();
            if entries.is_empty() {
                return Err(Error::new_spanned(
                    initial_state,
                    "No transitions provided for a compact representation",
                ));
            }
            entries
        };
        Ok(Self {
            initial_state,
            transitions,
        })
    }
}
