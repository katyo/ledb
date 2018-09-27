use keywords::*;
use types::*;

pub enum OrderKind {
    Asc,
    Desc,
}

impl Parse for OrderKind {
    fn parse(input: ParseStream) -> Result<Self> {
        use self::OrderKind::*;
        let lookahead = input.lookahead1();
        if lookahead.peek(desc) {
            input.parse().map(|_: desc| Desc)
        } else if lookahead.peek(Token![<]) {
            input.parse().map(|_: Token![<]| Desc)
        } else if lookahead.peek(asc) {
            input.parse().map(|_: asc| Asc)
        } else if lookahead.peek(Token![>]) {
            input.parse().map(|_: Token![>]| Asc)
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for OrderKind {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        use self::OrderKind::*;
        tokens.append_all(match self {
            Asc => quote!(ledb::OrderKind::Asc),
            Desc => quote!(ledb::OrderKind::Desc),
        });
    }
}

#[cfg(test)]
mod test {
    use super::OrderKind;
    use syn::parse_str;

    #[test]
    fn asc() {
        parse_str::<OrderKind>("asc").unwrap();
        parse_str::<OrderKind>("desc").unwrap();
    }

    #[test]
    fn desc() {
        parse_str::<OrderKind>(">").unwrap();
        parse_str::<OrderKind>("<").unwrap();
    }

    #[test]
    fn invalid() {
        assert!(parse_str::<OrderKind>("true").is_err());
        assert!(parse_str::<OrderKind>("false").is_err());
        assert!(parse_str::<OrderKind>("_").is_err());
        assert!(parse_str::<OrderKind>("&").is_err());
    }
}
