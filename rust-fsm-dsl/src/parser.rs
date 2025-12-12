mod guard;
mod input;
mod match_arm;
mod output;
mod transition;

pub use guard::Guard;
pub use input::{InputFields, InputVariant};
pub use match_arm::MatchArm;
pub use output::{Output, OutputSpec};
pub use transition::TransitionDef;

use syn::{
    parenthesized,
    parse::{Parse, ParseStream, Result},
    Attribute, Expr, Ident, ItemUse, Path, Token, Visibility,
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
///     }
/// }
/// ```
pub struct StateMachineDef {
    pub doc: Vec<Attribute>,
    /// The visibility modifier (applies to all generated items)
    pub visibility: Visibility,
    pub name: Ident,
    pub initial_state: Ident,
    pub use_statements: Vec<ItemUse>,
    pub transitions: Vec<TransitionDef>,
    pub attributes: Vec<Attribute>,
    pub input_type: Option<Path>,
    pub state_type: Option<Path>,
    pub output_type: Option<Path>,
    pub before_transition: Option<Expr>,
    pub after_transition: Option<Expr>,
}

impl Parse for StateMachineDef {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut state_machine_attributes = Vec::new();
        let mut doc = Vec::new();
        let attributes = Attribute::parse_outer(input)?
            .into_iter()
            .filter_map(|attribute| {
                if attribute.path().is_ident("state_machine") {
                    state_machine_attributes.push(attribute);
                    None
                } else if attribute.path().is_ident("doc") {
                    doc.push(attribute);
                    None
                } else {
                    Some(attribute)
                }
            })
            .collect();

        let mut input_type = None;
        let mut state_type = None;
        let mut output_type = None;
        let mut before_transition = None;
        let mut after_transition = None;

        for attribute in state_machine_attributes {
            attribute.parse_nested_meta(|meta| {
                let content;
                parenthesized!(content in meta.input);

                if meta.path.is_ident("input")
                    || meta.path.is_ident("state")
                    || meta.path.is_ident("output")
                {
                    let p: Path = content.parse()?;

                    if meta.path.is_ident("input") {
                        input_type = Some(p);
                    } else if meta.path.is_ident("state") {
                        state_type = Some(p);
                    } else if meta.path.is_ident("output") {
                        output_type = Some(p);
                    }
                } else if meta.path.is_ident("before_transition")
                    || meta.path.is_ident("after_transition")
                {
                    let expr: Expr = content.parse()?;

                    if meta.path.is_ident("before_transition") {
                        before_transition = Some(expr);
                    } else if meta.path.is_ident("after_transition") {
                        after_transition = Some(expr);
                    }
                }

                Ok(())
            })?;
        }

        let visibility = input.parse()?;
        let name = input.parse()?;

        let initial_state_content;
        parenthesized!(initial_state_content in input);
        let initial_state = initial_state_content.parse()?;

        // Parse optional use statements
        let mut use_statements = Vec::new();
        while input.peek(Token![use]) {
            use_statements.push(input.parse()?);
        }

        let transitions = input
            .parse_terminated(TransitionDef::parse, Token![,])?
            .into_iter()
            .collect();

        Ok(Self {
            doc,
            visibility,
            name,
            initial_state,
            use_statements,
            transitions,
            attributes,
            input_type,
            state_type,
            output_type,
            before_transition,
            after_transition,
        })
    }
}
