use keywords::*;
use types::*;

pub enum KeyType {
    Int,
    Float,
    Bool,
    String,
    Binary,
}

impl Parse for KeyType {
    fn parse(input: ParseStream) -> Result<Self> {
        use self::KeyType::*;
        let lookahead = input.lookahead1();
        if lookahead.peek(integer) {
            input.parse().map(|_: integer| Int)
        } else if lookahead.peek(int) {
            input.parse().map(|_: int| Int)
        } else if lookahead.peek(float) {
            input.parse().map(|_: float| Float)
        } else if lookahead.peek(boolean) {
            input.parse().map(|_: boolean| Bool)
        } else if lookahead.peek(bool) {
            input.parse().map(|_: bool| Bool)
        } else if lookahead.peek(string) {
            input.parse().map(|_: string| String)
        } else if lookahead.peek(str) {
            input.parse().map(|_: str| String)
        } else if lookahead.peek(text) {
            input.parse().map(|_: text| String)
        } else if lookahead.peek(binary) {
            input.parse().map(|_: binary| Binary)
        } else if lookahead.peek(bin) {
            input.parse().map(|_: bin| Binary)
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for KeyType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        use self::KeyType::*;
        tokens.append_all(match self {
            Int => quote!(ledb::KeyType::Int),
            Float => quote!(ledb::KeyType::Float),
            Bool => quote!(ledb::KeyType::Bool),
            String => quote!(ledb::KeyType::String),
            Binary => quote!(ledb::KeyType::Binary),
        });
    }
}

#[cfg(test)]
mod test {
    use super::KeyType;
    use syn::parse_str;

    #[test]
    fn integer() {
        parse_str::<KeyType>("integer").unwrap();
        parse_str::<KeyType>("int").unwrap();
    }

    #[test]
    fn float() {
        parse_str::<KeyType>("float").unwrap();
    }

    #[test]
    fn boolean() {
        parse_str::<KeyType>("boolean").unwrap();
        parse_str::<KeyType>("bool").unwrap();
    }

    #[test]
    fn string() {
        parse_str::<KeyType>("string").unwrap();
        parse_str::<KeyType>("str").unwrap();
        parse_str::<KeyType>("text").unwrap();
    }

    #[test]
    fn binary() {
        parse_str::<KeyType>("binary").unwrap();
        parse_str::<KeyType>("bin").unwrap();
    }

    #[test]
    fn invalid() {
        assert!(parse_str::<KeyType>("number").is_err());
        assert!(parse_str::<KeyType>("num").is_err());
        assert!(parse_str::<KeyType>("_").is_err());
        assert!(parse_str::<KeyType>("bytes").is_err());
    }
}
