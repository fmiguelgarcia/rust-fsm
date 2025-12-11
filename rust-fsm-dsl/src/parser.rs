use syn::{
    braced, bracketed, parenthesized,
    parse::{Error, Parse, ParseStream, Result},
    punctuated::Punctuated,
    token::{Bracket, Paren},
    Attribute, Expr, Ident, ItemUse, Path, Token, Type, Visibility,
};

/// The output of a state transition
pub enum OutputSpec {
    /// A constant output variant (e.g., [SetupTimer])
    Constant(Ident),
    /// A function call output (e.g., [|x| compute(x)])
    Call(Expr),
}

pub struct Output(Option<OutputSpec>);

impl Parse for Output {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.lookahead1().peek(Bracket) {
            let output_content;
            bracketed!(output_content in input);

            // Check if it starts with a closure (|)
            if output_content.peek(Token![|]) {
                // Parse as closure expression
                let expr: Expr = output_content.parse()?;
                return Ok(Self(Some(OutputSpec::Call(expr))));
            }

            // Parse as constant identifier
            let ident: Ident = output_content.parse()?;
            Ok(Self(Some(OutputSpec::Constant(ident))))
        } else {
            Ok(Self(None))
        }
    }
}

impl From<Output> for Option<OutputSpec> {
    fn from(output: Output) -> Self {
        output.0
    }
}

/// Represents an input variant, which can be a simple identifier or a tuple variant
pub struct InputVariant {
    pub name: Ident,
    pub fields: Punctuated<Type, Token![,]>,
}

impl Parse for InputVariant {
    /// Parse the identifier and optionally tuple fields (for compact format)
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse()?;
        let fields = if input.lookahead1().peek(Paren) {
            let content;
            parenthesized!(content in input);
            content.parse_terminated(Type::parse, Token![,])?
        } else {
            Punctuated::new()
        };

        Ok(Self { name, fields })
    }
}

/// Represents a guard expression for a transition
pub struct Guard {
    pub expr: Expr,
}

/// Represents a part of state transition without the initial state. The `Parse`
/// trait is implemented for the compact form.
pub struct TransitionEntry {
    pub input_value: InputVariant,
    pub guard: Option<Guard>,
    pub final_state: Ident,
    pub output: Option<OutputSpec>,
}

impl Parse for TransitionEntry {
    fn parse(input: ParseStream) -> Result<Self> {
        let input_value = InputVariant::parse(input)?;

        // Check for optional guard: if <expr>
        let guard = if input.peek(Token![if]) {
            input.parse::<Token![if]>()?;
            Some(Guard {
                expr: input.parse()?,
            })
        } else {
            None
        };

        input.parse::<Token![=>]>()?;
        let final_state = input.parse()?;
        let output = input.parse::<Output>()?.into();
        Ok(Self {
            input_value,
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
                input_value,
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

        for attribute in state_machine_attributes {
            attribute.parse_nested_meta(|meta| {
                let content;
                parenthesized!(content in meta.input);
                let p: Path = content.parse()?;

                if meta.path.is_ident("input") {
                    input_type = Some(p);
                } else if meta.path.is_ident("state") {
                    state_type = Some(p);
                } else if meta.path.is_ident("output") {
                    output_type = Some(p);
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
        })
    }
}
