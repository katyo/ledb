use modify_action::Action;
use types::*;

pub struct ModField(Field, Action);

impl Parse for ModField {
    fn parse(input: ParseStream) -> Result<Self> {
        let field = input.parse()?;
        let action = input.parse()?;
        Ok(ModField(field, action))
    }
}

impl ToTokens for ModField {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens);
        self.1.to_tokens(tokens);
    }
}

pub struct Modify(Punctuated<ModField, Comma>);

impl Parse for Modify {
    fn parse(input: ParseStream) -> Result<Self> {
        Punctuated::parse_separated_nonempty(input).map(Modify)
    }
}

impl ToTokens for Modify {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Modify(modify) = self;
        tokens.append_all(quote! { #(#modify),* })
    }
}

#[cfg(test)]
mod test {
    use super::Modify;
    use syn::parse_str;

    #[test]
    fn set() {
        parse_str::<Modify>("field = 1").unwrap();
        parse_str::<Modify>("field = -0.5").unwrap();
        parse_str::<Modify>("field = \"abc\"").unwrap();
        parse_str::<Modify>("field = true").unwrap();
    }
}
