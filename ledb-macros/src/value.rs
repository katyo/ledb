use types::*;

pub enum KeyData {
    Single(Expr),
    Multiple(Vec<Expr>),
}

impl Parse for KeyData {
    fn parse(input: ParseStream) -> Result<Self> {
        use self::KeyData::*;
        if input.peek(Bracket) {
            let items;
            bracketed!(items in input);
            Punctuated::<Expr, Comma>::parse_terminated(&items)
                .map(|items| Multiple(items.into_iter().collect()))
        } else {
            parse_expr(input).map(Single)
        }
    }
}

impl ToTokens for KeyData {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        use self::KeyData::*;
        tokens.append_all(match self {
            Single(expr) => quote! { KeyData::from(#expr) },
            Multiple(exprs) => quote! { vec![#(KeyData::from(#exprs)),*] },
        })
    }
}

pub enum ValData {
    Expr(Expr),
    Json(TokenTree),
}

impl Parse for ValData {
    fn parse(input: ParseStream) -> Result<Self> {
        use self::ValData::*;
        if input.peek(Brace) {
            input.parse().map(Json)
        } else {
            input.parse().map(Expr)
        }
    }
}

impl ToTokens for ValData {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        use self::ValData::*;
        tokens.append_all(match self {
            Expr(expr) => quote! { ledb::to_value(#expr).unwrap() },
            Json(json) => quote! { ledb::to_value(json!(#json)).unwrap() },
        })
    }
}

fn parse_expr(input: ParseStream) -> Result<Expr> {
    let lookahead = input.lookahead1();
    if lookahead.peek(Lit) {
        input.parse()
    } else if lookahead.peek(Ident) {
        //Punctuated::parse_separated_nonempty(input).map(Field);
        input.parse()
    } else if lookahead.peek(Paren) {
        let nested;
        parenthesized!(nested in input);
        let expr = nested.parse()?;
        if nested.is_empty() {
            Ok(expr)
        } else {
            Err(nested.error("Invalid sub expression"))
        }
    } else if lookahead.peek(Brace) {
        let nested;
        braced!(nested in input);
        let expr = nested.parse()?;
        if nested.is_empty() {
            Ok(expr)
        } else {
            Err(nested.error("Invalid sub expression"))
        }
    } else {
        Err(lookahead.error())
    }
}
