pub use proc_macro2::{TokenStream, TokenTree};
pub use quote::{ToTokens, TokenStreamExt};
pub use syn::parse::{Parse, ParseStream, Result};
pub use syn::punctuated::Punctuated;
pub use syn::token::{Brace, Bracket, Comma, Dot, Paren};
pub use syn::{Expr, Ident, Lit, LitStr, Type};

pub type Bool = bool;
pub type Callback = Ident;
pub type Collection = Ident;

/// Document field
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
