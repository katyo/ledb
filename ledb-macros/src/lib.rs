//#![recursion_limit = "128"]
//#![feature(proc_macro_diagnostic)]

#[macro_use]
extern crate proc_macro_hack;

extern crate proc_macro;
extern crate proc_macro2;

#[macro_use]
extern crate syn;

#[macro_use]
extern crate quote;

mod filter;
mod filter_comp;
mod find;
mod index;
mod index_field;
mod index_kind;
mod insert;
mod key_type;
mod keywords;
mod modify;
mod modify_action;
mod order;
mod order_kind;
mod query;
mod range;
mod remove;
mod types;
mod update;
mod value;

/*
// Let's leave it until better times...
#[proc_macro]
pub fn _query_dsl(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    use query::WrappedQuery;
    let query = parse_macro_input!(input as WrappedQuery);
    let expanded = quote! { #query };
    expanded.into()
}
 */

/*
// Today we can only go by the dirty hack
proc_macro_expr_impl! {
    pub fn _query_dsl_impl(input: &str) -> String {
        use query::WrappedQuery;
        use syn::parse::Result;
        use syn::parse_str;
        use quote::ToTokens;

        let query: Result<WrappedQuery> = parse_str(input);

        format!("{}", query
                .map(|val| val.into_token_stream())
                .unwrap_or_else(|err| err.to_compile_error()))
    }
}
 */

use find::Find;
use index::Index;
use insert::Insert;
use query::Query;
use remove::Remove;
use types::*;
use update::Update;

enum Extract {
    DocumentType,
    Collection,
    CollectionName,
    Fields,
    Filter,
    Order,
    Modify,
    Document,
}

mod extract {
    custom_keyword!(DOCUMENT_TYPE);
    custom_keyword!(COLLECTION);
    custom_keyword!(COLLECTION_NAME);
    custom_keyword!(FIELDS);
    custom_keyword!(FILTER);
    custom_keyword!(ORDER);
    custom_keyword!(MODIFY);
    custom_keyword!(DOCUMENT);
}

impl Parse for Extract {
    fn parse(input: ParseStream) -> Result<Self> {
        use Extract::*;
        let lookahead = input.lookahead1();
        if lookahead.peek(extract::DOCUMENT_TYPE) {
            input
                .parse::<extract::DOCUMENT_TYPE>()
                .map(|_| DocumentType)
        } else if lookahead.peek(extract::COLLECTION) {
            input.parse::<extract::COLLECTION>().map(|_| Collection)
        } else if lookahead.peek(extract::COLLECTION_NAME) {
            input
                .parse::<extract::COLLECTION_NAME>()
                .map(|_| CollectionName)
        } else if lookahead.peek(extract::FIELDS) {
            input.parse::<extract::FIELDS>().map(|_| Fields)
        } else if lookahead.peek(extract::FILTER) {
            input.parse::<extract::FILTER>().map(|_| Filter)
        } else if lookahead.peek(extract::ORDER) {
            input.parse::<extract::ORDER>().map(|_| Order)
        } else if lookahead.peek(extract::MODIFY) {
            input.parse::<extract::MODIFY>().map(|_| Modify)
        } else if lookahead.peek(extract::DOCUMENT) {
            input.parse::<extract::DOCUMENT>().map(|_| Document)
        } else {
            Err(lookahead.error())
        }
    }
}

struct ExtractQuery(Extract, Query);

impl Parse for ExtractQuery {
    fn parse(input: ParseStream) -> Result<Self> {
        let extract = input.parse()?;
        input.parse::<Token![,]>()?;
        let query = input.parse()?;
        Ok(ExtractQuery(extract, query))
    }
}

impl ToTokens for ExtractQuery {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match &self.1 {
            Query::Index(Index { collection, fields }) => match self.0 {
                Extract::Collection => collection.to_tokens(tokens),
                Extract::CollectionName => quote! { stringify!(#collection) }.to_tokens(tokens),
                Extract::Fields => quote! { vec![#(#fields),*] }.to_tokens(tokens),
                _ => (),
            },
            Query::Find(Find {
                document_type,
                collection,
                filter,
                order,
            }) => match &self.0 {
                Extract::DocumentType => document_type.to_tokens(tokens),
                Extract::Collection => collection.to_tokens(tokens),
                Extract::CollectionName => quote! { stringify!(#collection) }.to_tokens(tokens),
                Extract::Filter => filter.to_tokens(tokens),
                Extract::Order => order.to_tokens(tokens),
                _ => (),
            },
            Query::Insert(Insert {
                collection,
                document,
            }) => match self.0 {
                Extract::Collection => collection.to_tokens(tokens),
                Extract::CollectionName => quote! { stringify!(#collection) }.to_tokens(tokens),
                Extract::Document => document.to_tokens(tokens),
                _ => (),
            },
            Query::Update(Update {
                collection,
                filter,
                modify,
            }) => match self.0 {
                Extract::Collection => collection.to_tokens(tokens),
                Extract::CollectionName => quote! { stringify!(#collection) }.to_tokens(tokens),
                Extract::Filter => filter.to_tokens(tokens),
                Extract::Modify => modify.to_tokens(tokens),
                _ => (),
            },
            Query::Remove(Remove { collection, filter }) => match self.0 {
                Extract::Collection => collection.to_tokens(tokens),
                Extract::CollectionName => quote! { stringify!(#collection) }.to_tokens(tokens),
                Extract::Filter => filter.to_tokens(tokens),
                _ => (),
            },
        }
    }
}

proc_macro_expr_impl! {
    pub fn _query_dsl_extract_impl(input: &str) -> String {
        use quote::ToTokens;
        use syn::parse_str;

        format!("{}", parse_str::<ExtractQuery>(input)
                .map(|val| val.into_token_stream())
                .unwrap_or_else(|err| err.to_compile_error()))
    }
}
