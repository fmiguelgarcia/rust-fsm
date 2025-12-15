use crate::{
    parser::{Guard, InputVariant, Output, OutputSpec},
    MatchCase,
};

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
    /// Parses a transition entry from the input stream.
    ///
    /// Supports three formats:
    /// - Simple transition: `Input => State`
    /// - With output: `Input => State [Output]`
    /// - With guard: `Input if guard => State` or `Input match binding { arms }`
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

impl TransitionDef {
    /// Parse the transition in the simple format
    /// InitialState(Input) => ResultState [Output]
    /// Note: Guards are not supported in simple format (only in compact format)
    fn as_regular_transition(ps: ParseStream) -> Result<Vec<TransitionEntry>> {
        let input_content;
        parenthesized!(input_content in ps);
        let input = InputVariant::parse(&input_content)?;
        ps.parse::<Token![=>]>()?;
        let final_state = ps.parse()?;
        let output = ps.parse::<Output>()?.into();

        let te = TransitionEntry {
            input,
            guard: None,
            final_state,
            output,
        };
        Ok(vec![te])
    }

    /// Parse the transition in the compact format
    /// InitialState => {
    ///     Input1 => State1,
    ///     Input2 => State2 [Output]
    /// }
    fn as_sub_transions(ps: ParseStream) -> Result<Vec<TransitionEntry>> {
        ps.parse::<Token![=>]>()?;
        let entries_content;
        braced!(entries_content in ps);

        let entries: Vec<_> = entries_content
            .parse_terminated(TransitionEntry::parse, Token![,])?
            .into_iter()
            .collect();
        Ok(entries)
    }

    /// Parses transitions for a given initial state.
    ///
    /// Determines whether to parse as regular (simple) or compact format
    /// by looking ahead at the next token.
    fn parse_transitions(initial_state: &Ident, ps: ParseStream) -> Result<Vec<TransitionEntry>> {
        let entries = if ps.lookahead1().peek(Paren) {
            Self::as_regular_transition(ps)?
        } else {
            Self::as_sub_transions(ps)?
        };

        if entries.is_empty() {
            return Err(Error::new_spanned(
                initial_state,
                "No transitions provided for a compact representation",
            ));
        }
        Ok(entries)
    }

    /// Converts transition definitions into match cases for code generation.
    ///
    /// Expands guard arms and creates a `MatchCase` for each possible transition path.
    /// - Simple transitions become one match case
    /// - `if` guards become one match case with the guard expression
    /// - `match` guards expand into multiple match cases, one per arm
    ///
    /// # Examples
    ///
    /// For a simple transition:
    /// ```ignore
    /// Locked(Coin) => Unlocked
    /// ```
    /// Creates one `MatchCase` with no guard.
    ///
    /// For a match guard:
    /// ```ignore
    /// Locked => {
    ///     Coin(u32) match amount {
    ///         0..50 => Locked [RefundInsufficient],
    ///         50.. => Unlocked
    ///     }
    /// }
    /// ```
    /// Creates two `MatchCase` instances, one for each pattern arm.
    pub fn as_match_cases(&self) -> Vec<MatchCase<'_>> {
        let state = &self.initial_state;

        self.transitions
            .iter()
            .flat_map(|entry| {
                let input = &entry.input;
                let next_state = entry.final_state.as_ref();
                let output = entry.output.as_ref();

                match &entry.guard {
                    Some(Guard::Binding(b)) => b
                        .arms
                        .iter()
                        .map(|arm| {
                            let guard_expr = b.guard_expr_for_arm(&input.fields, arm);

                            MatchCase::new(state, input, &arm.final_state)
                                .with_guard(guard_expr)
                                .with_output(arm.output.as_ref())
                        })
                        .collect::<Vec<_>>(),
                    Some(Guard::Closure(guard)) => {
                        let next_state = next_state.expect("if guard must have final_state");
                        let case = MatchCase::new(state, input, next_state)
                            .with_output(output)
                            .with_guard(guard.clone());

                        vec![case]
                    }
                    None => {
                        let next_state =
                            next_state.expect("simple transition must have final_state");
                        let case = MatchCase::new(state, input, next_state).with_output(output);

                        vec![case]
                    }
                }
            })
            .collect()
    }
}

impl Parse for TransitionDef {
    /// Parses a complete transition definition.
    ///
    /// Expects an initial state identifier followed by transition entries
    /// in either regular or compact format.
    fn parse(input: ParseStream) -> Result<Self> {
        let initial_state = input.parse()?;
        let transitions = Self::parse_transitions(&initial_state, input)?;

        Ok(Self {
            initial_state,
            transitions,
        })
    }
}
