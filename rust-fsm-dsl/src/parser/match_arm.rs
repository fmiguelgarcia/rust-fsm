use crate::parser::{Output, OutputSpec};

use syn::{
    parse::{Parse, ParseStream, Result},
    Expr, Ident, Token,
};

/// Represents a single arm in a guard expression
pub struct MatchArm {
    pub pattern: Expr,
    pub final_state: Ident,
    pub output: Option<OutputSpec>,
}

impl Parse for MatchArm {
    fn parse(input: ParseStream) -> Result<Self> {
        let pattern = input.parse()?;
        input.parse::<Token![=>]>()?;
        let final_state = input.parse()?;
        let output = input.parse::<Output>()?.into();
        Ok(Self {
            pattern,
            final_state,
            output,
        })
    }
}
