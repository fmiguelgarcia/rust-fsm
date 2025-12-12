use syn::{
    parenthesized,
    parse::{Parse, ParseStream, Result},
    punctuated::Punctuated,
    token::Paren,
    Ident, Token, Type,
};

pub type InputFields = Punctuated<Type, Token![,]>;

/// Represents an input variant, which can be a simple identifier or a tuple variant
pub struct InputVariant {
    pub name: Ident,
    pub fields: InputFields,
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
