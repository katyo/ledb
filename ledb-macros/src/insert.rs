use keywords::*;
use types::*;

pub enum Document {
    Expr(Expr),
    Json(TokenTree),
}

impl Parse for Document {
    fn parse(input: ParseStream) -> Result<Self> {
        use self::Document::*;
        if input.peek(Brace) {
            //let json;
            //let brace = braced!(json in input);
            //json.parse_terminated(TokenTree).map(Json)
            input.parse().map(Json)
        } else {
            input.parse().map(Expr)
        }
    }
}

impl ToTokens for Document {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        use self::Document::*;
        tokens.append_all(match self {
            Expr(expr) => quote! { #expr },
            Json(json) => quote! { &json!(#json) },
        })
    }
}

pub struct Insert {
    pub collection: Collection,
    pub document: Document,
}

impl Parse for Insert {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<insert>()?;
        input.parse::<into>()?;
        let collection = input.parse()?;
        let document = input.parse()?;
        Ok(Insert {
            collection,
            document,
        })
    }
}

impl ToTokens for Insert {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            collection,
            document,
        } = self;
        tokens.append_all(quote! {
            #collection, #document
        })
    }
}

#[cfg(test)]
mod test {
    use super::Insert;
    use syn::parse_str;

    #[test]
    fn typed() {
        parse_str::<Insert>("insert into collection document").unwrap();
    }

    #[test]
    fn json() {
        parse_str::<Insert>("insert into collection { \"field\": \"value\" }").unwrap();
    }
}
