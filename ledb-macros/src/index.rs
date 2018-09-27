use keywords::*;
use types::*;

use index_field::IndexField;

pub struct Index {
    pub collection: Collection,
    pub fields: Punctuated<IndexField, Comma>,
}

impl Parse for Index {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<index>()?;
        input.parse::<For>()?;
        let collection = input.parse()?;
        let fields = Punctuated::parse_terminated(input)?;
        Ok(Self { collection, fields })
    }
}

impl ToTokens for Index {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { collection, fields } = self;
        tokens.append_all(quote! {
            #collection, &[#(#fields),*]
        })
    }
}

#[cfg(test)]
mod test {
    use super::Index;
    use syn::parse_str;

    #[test]
    fn single_without_kind() {
        parse_str::<Index>("index for collection field int").unwrap();
        parse_str::<Index>("index for collection field str").unwrap();
    }

    #[test]
    fn single_with_kind() {
        parse_str::<Index>("index for collection field int unique").unwrap();
        parse_str::<Index>("index for collection field int duplicate").unwrap();
    }

    #[test]
    fn multiple() {
        parse_str::<Index>(
            "index for collection field int unique, field.subfield string, key.subkey bool dup",
        ).unwrap();
    }

    #[test]
    fn invalid() {
        assert!(parse_str::<Index>("index collection field int").is_err());
        assert!(parse_str::<Index>("index for collection field").is_err());
    }
}
