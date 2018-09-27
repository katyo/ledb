use keywords::*;
use types::*;

use find::Find;
use index::Index;
use insert::Insert;
use remove::Remove;
use update::Update;

pub enum Query {
    Index(Index),
    Find(Find),
    Insert(Insert),
    Update(Update),
    Remove(Remove),
}

impl Parse for Query {
    fn parse(input: ParseStream) -> Result<Self> {
        use self::Query::*;
        let lookahead = input.lookahead1();
        if lookahead.peek(index) {
            input.parse().map(Index)
        } else if lookahead.peek(find) {
            input.parse().map(Find)
        } else if lookahead.peek(insert) {
            input.parse().map(Insert)
        } else if lookahead.peek(update) {
            input.parse().map(Update)
        } else if lookahead.peek(remove) {
            input.parse().map(Remove)
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for Query {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        use self::Query::*;
        tokens.append_all(match self {
            Index(query) => quote! { index, #query },
            Find(query) => quote! { find, #query },
            Insert(query) => quote! { insert, #query },
            Update(query) => quote! { update, #query },
            Remove(query) => quote! { remove, #query },
        });
    }
}

pub struct WrappedQuery {
    callback: Callback,
    query: Query,
}

impl Parse for WrappedQuery {
    fn parse(input: ParseStream) -> Result<Self> {
        let callback = input.parse()?;
        input.parse::<Comma>()?;
        let query = input.parse()?;
        Ok(Self { callback, query })
    }
}

impl ToTokens for WrappedQuery {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { callback, query } = self;
        tokens.append_all(quote! {
            #callback!(#query)
        });
    }
}

#[cfg(test)]
mod test {
    use super::WrappedQuery;
    use syn::parse_str;

    #[test]
    fn index() {
        parse_str::<WrappedQuery>("callback, index for collection field int").unwrap();
        parse_str::<WrappedQuery>(
            "callback, index for collection field string unique, field.subfield float, other_field bool dup",
        ).unwrap();
    }

    #[test]
    fn find() {
        parse_str::<WrappedQuery>("callback, find in collection").unwrap();
        parse_str::<WrappedQuery>("callback, find Doc in collection").unwrap();
    }

    #[test]
    fn insert() {
        parse_str::<WrappedQuery>("callback, insert into collection document").unwrap();
        parse_str::<WrappedQuery>("callback, insert into collection {}").unwrap();
    }

    #[test]
    fn update() {
        parse_str::<WrappedQuery>("callback, update in collection modify field = 1").unwrap();
    }

    #[test]
    fn remove() {
        parse_str::<WrappedQuery>("callback, remove from collection").unwrap();
    }
}
