use filter::Filter;
use keywords::{self, *};
use order::Order;
use types::*;

pub struct Find {
    pub document_type: Type,
    pub collection: Collection,
    pub filter: Option<Filter>,
    pub order: Option<Order>,
}

impl Parse for Find {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<find>()?;
        let document_type = if input.peek(In) {
            parse_quote! {_}
        } else {
            input.parse()?
        };
        input.parse::<In>()?;
        let collection = input.parse()?;
        let filter = if input.peek(Where) {
            input.parse::<Where>()?;
            Some(input.parse()?)
        } else {
            None
        };
        let order = if input.peek(order) {
            input.parse::<keywords::order>()?;
            Some(input.parse()?)
        } else {
            None
        };
        Ok(Find {
            document_type,
            collection,
            filter,
            order,
        })
    }
}

impl ToTokens for Find {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Find {
            document_type,
            collection,
            filter,
            order,
        } = self;
        tokens.append_all(quote! {
            #document_type, #collection, #filter, #order
        })
    }
}

#[cfg(test)]
mod test {
    use super::Find;
    use syn::parse_str;

    #[test]
    fn with_type() {
        parse_str::<Find>("find Doc in collection").unwrap();
    }

    #[test]
    fn with_infer() {
        parse_str::<Find>("find _ in collection").unwrap();
    }

    #[test]
    fn without_type() {
        parse_str::<Find>("find in collection").unwrap();
    }

    #[test]
    fn with_order() {
        parse_str::<Find>("find in collection order asc").unwrap();
        parse_str::<Find>("find in collection order desc").unwrap();
        parse_str::<Find>("find in collection order by field asc").unwrap();
        parse_str::<Find>("find in collection order by field.subfield desc").unwrap();
    }

    #[test]
    fn with_filter() {}

    #[test]
    fn with_filter_and_order() {}
}
