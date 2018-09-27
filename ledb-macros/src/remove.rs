use filter::Filter;
use keywords::*;
use types::*;

pub struct Remove {
    pub collection: Collection,
    pub filter: Option<Filter>,
}

impl Parse for Remove {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<remove>()?;
        input.parse::<from>()?;
        let collection = input.parse()?;

        let filter = if input.peek(Where) {
            input.parse::<Where>()?;
            Some(input.parse()?)
        } else {
            None
        };

        Ok(Remove { collection, filter })
    }
}

impl ToTokens for Remove {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Remove { collection, filter } = self;
        tokens.append_all(quote! {
            #collection, #filter
        })
    }
}

#[cfg(test)]
mod test {
    use super::Remove;
    use syn::parse_str;

    #[test]
    fn without_filter() {
        parse_str::<Remove>("remove from collection").unwrap();
    }
}
