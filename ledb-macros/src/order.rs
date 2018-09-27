use keywords::*;
use types::*;

use order_kind::OrderKind;

pub enum Order {
    Primary(OrderKind),
    Field(Field, OrderKind),
}

impl Parse for Order {
    fn parse(input: ParseStream) -> Result<Self> {
        use self::Order::*;
        if input.peek(by) {
            input.parse::<by>()?;
            let field = input.parse()?;
            let order = input.parse()?;
            Ok(Field(field, order))
        } else {
            input.parse().map(Primary)
        }
    }
}

impl ToTokens for Order {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        use self::Order::*;
        tokens.append_all(match self {
            Primary(kind) => quote!(ledb::Order::Primary(#kind)),
            Field(field, kind) => quote!(ledb::Order::Field(#field, #kind)),
        });
    }
}

#[cfg(test)]
mod test {
    use super::Order;
    use syn::parse_str;

    #[test]
    fn primary() {
        parse_str::<Order>("asc").unwrap();
        parse_str::<Order>("desc").unwrap();
    }

    #[test]
    fn field() {
        parse_str::<Order>("by field asc").unwrap();
        parse_str::<Order>("by field.subfield desc").unwrap();
    }

    #[test]
    fn invalid() {
        assert!(parse_str::<Order>("field asc").is_err());
        assert!(parse_str::<Order>("field.subfield desc").is_err());
        assert!(parse_str::<Order>("by field/subfield desc").is_err());
    }
}
