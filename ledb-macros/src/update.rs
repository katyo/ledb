use filter::Filter;
use keywords::*;
use modify::Modify;
use types::*;

pub struct Update {
    pub collection: Collection,
    pub filter: Option<Filter>,
    pub modify: Modify,
}

impl Parse for Update {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<update>()?;
        input.parse::<In>()?;
        let collection = input.parse()?;

        let (filter, modify) = if input.peek(modify) {
            input.parse::<modify>()?;
            let modify = input.parse()?;
            let filter = if input.peek(Where) {
                input.parse::<Where>()?;
                Some(input.parse()?)
            } else {
                None
            };
            (filter, modify)
        } else {
            let filter = if input.peek(Where) {
                input.parse::<Where>()?;
                Some(input.parse()?)
            } else {
                None
            };
            input.parse::<modify>()?;
            let modify = input.parse()?;
            (filter, modify)
        };

        Ok(Update {
            collection,
            filter,
            modify,
        })
    }
}

impl ToTokens for Update {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Update {
            collection,
            filter,
            modify,
        } = self;
        tokens.append_all(quote! {
            #collection, #filter, #modify
        })
    }
}

#[cfg(test)]
mod test {
    use super::Update;
    use syn::parse_str;

    #[test]
    fn without_filter() {
        parse_str::<Update>("update in collection modify field = 1").unwrap();
    }
}
