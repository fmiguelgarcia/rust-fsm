use syn::{
    bracketed,
    parse::{Parse, ParseStream, Result},
    token::Bracket,
    Expr, Ident, Token,
};

/// The output of a state transition
pub enum OutputSpec {
    /// A constant output variant (e.g., [SetupTimer])
    Constant(Ident),
    /// A function call output (e.g., [|x| compute(x)])
    Call(Expr),
}

impl Parse for OutputSpec {
    fn parse(input: ParseStream) -> Result<Self> {
        let output_content;
        bracketed!(output_content in input);

        // Check if it starts with a closure (|)
        if output_content.peek(Token![|]) {
            // Parse as closure expression
            let expr: Expr = output_content.parse()?;
            return Ok(Self::Call(expr));
        }

        // Parse as constant identifier
        let ident: Ident = output_content.parse()?;
        Ok(Self::Constant(ident))
    }
}

pub struct Output(Option<OutputSpec>);

impl Parse for Output {
    fn parse(input: ParseStream) -> Result<Self> {
        let spec = if input.lookahead1().peek(Bracket) {
            Some(input.parse::<OutputSpec>()?)
        } else {
            None
        };

        Ok(Self(spec))
    }
}

impl From<Output> for Option<OutputSpec> {
    fn from(output: Output) -> Self {
        output.0
    }
}
