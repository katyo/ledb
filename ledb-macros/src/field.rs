use keywords::*;
use types::*;

pub struct Field(Punctuated<Ident, Dot>);

impl Parse for Field {
    fn parse(input: ParseStream) -> Result<Self> {
        Punctuated::parse_separated_nonempty(input).map(Field)
    }
}

impl ToTokens for Field {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens);
    }
}
