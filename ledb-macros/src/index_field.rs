use types::*;

use index_kind::IndexKind;
use key_type::KeyType;

pub struct IndexField {
    field: Field,
    key: KeyType,
    kind: Option<IndexKind>,
}

impl Parse for IndexField {
    fn parse(input: ParseStream) -> Result<Self> {
        let field = input.parse()?;
        let key = input.parse()?;
        let kind = if input.peek(Comma) || input.is_empty() {
            None
        } else {
            Some(input.parse()?)
        };
        Ok(Self { field, key, kind })
    }
}

impl ToTokens for IndexField {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { field, key, kind } = self;
        tokens.append_all(quote! {
            (#field, #kind.unwrap_or_else(ledb::IndexKind::default()), #key)
        })
    }
}
