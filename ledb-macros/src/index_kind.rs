use keywords::*;
use types::*;

pub enum IndexKind {
    Unique,
    Duplicate,
}

impl Parse for IndexKind {
    fn parse(input: ParseStream) -> Result<Self> {
        use self::IndexKind::*;
        let lookahead = input.lookahead1();
        if lookahead.peek(unique) {
            input.parse().map(|_: unique| Unique)
        } else if lookahead.peek(uni) {
            input.parse().map(|_: uni| Unique)
        } else if lookahead.peek(duplicate) {
            input.parse().map(|_: duplicate| Duplicate)
        } else if lookahead.peek(dup) {
            input.parse().map(|_: dup| Duplicate)
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for IndexKind {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        use self::IndexKind::*;
        tokens.append_all(match self {
            Unique => quote!(ledb::IndexKind::Unique),
            Duplicate => quote!(ledb::IndexKind::Duplicate),
        });
    }
}

#[cfg(test)]
mod test {
    use super::IndexKind;
    use syn::parse_str;

    #[test]
    fn unique() {
        parse_str::<IndexKind>("unique").unwrap();
        parse_str::<IndexKind>("uni").unwrap();
    }

    #[test]
    fn duplicate() {
        parse_str::<IndexKind>("duplicate").unwrap();
        parse_str::<IndexKind>("dup").unwrap();
    }

    #[test]
    fn invalid() {
        assert!(parse_str::<IndexKind>("uniq").is_err());
        assert!(parse_str::<IndexKind>("dupl").is_err());
    }
}
