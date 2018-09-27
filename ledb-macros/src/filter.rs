use filter_comp::Comp;
use types::*;

pub enum Cond {
    Not(Box<Filter>),
    And(Vec<Filter>),
    Or(Vec<Filter>),
}

impl ToTokens for Cond {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        use self::Cond::*;
        tokens.append_all(match self {
            Not(filter) => quote!(ledb::Cond::Not(Box::new(#filter))),
            And(filters) => quote!(ledb::Cond::And(vec![#(#filters),*])),
            Or(filters) => quote!(ledb::Cond::Or(vec![#(#filters),*])),
        });
    }
}

pub enum Filter {
    Cond(Cond),
    Comp(Field, Comp),
}

struct CondOr(Punctuated<CondAnd, Token![||]>);

impl From<CondOr> for Filter {
    fn from(CondOr(filters): CondOr) -> Self {
        if filters.len() > 1 {
            Filter::Cond(Cond::Or(filters.into_iter().map(Filter::from).collect()))
        } else {
            filters.into_iter().next().unwrap().into()
        }
    }
}

impl Parse for CondOr {
    fn parse(input: ParseStream) -> Result<Self> {
        Punctuated::parse_separated_nonempty(input).map(CondOr)
    }
}

struct CondAnd(Punctuated<CondNot, Token![&&]>);

impl From<CondAnd> for Filter {
    fn from(CondAnd(filters): CondAnd) -> Self {
        if filters.len() > 1 {
            Filter::Cond(Cond::And(filters.into_iter().map(Filter::from).collect()))
        } else {
            filters.into_iter().next().unwrap().into()
        }
    }
}

impl Parse for CondAnd {
    fn parse(input: ParseStream) -> Result<Self> {
        Punctuated::parse_separated_nonempty(input).map(CondAnd)
    }
}

struct CondNot(Option<Token![!]>, CompField);

impl From<CondNot> for Filter {
    fn from(CondNot(not, CompField(field, comp)): CondNot) -> Self {
        let (inv, comp) = comp.not_not();
        let filter = Filter::Comp(field, comp);
        if not.is_some() != inv {
            Filter::Cond(Cond::Not(Box::new(filter)))
        } else {
            filter
        }
    }
}

impl Parse for CondNot {
    fn parse(input: ParseStream) -> Result<Self> {
        let not = if input.peek(Token![!]) {
            Some(input.parse()?)
        } else {
            None
        };
        let filter = if input.peek(Paren) {
            let nested;
            parenthesized!(nested in input);
            let filter = nested.parse()?;
            if !nested.is_empty() {
                return Err(nested.error("Invalid nested filter"));
            }
            filter
        } else {
            input.parse()?
        };
        Ok(CondNot(not, filter))
    }
}

pub struct CompField(Field, Comp);

impl Parse for CompField {
    fn parse(input: ParseStream) -> Result<Self> {
        let field = input.parse()?;
        let comp = input.parse()?;
        Ok(CompField(field, comp))
    }
}

impl Parse for Filter {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse().map(|cond: CondOr| cond.into())
    }
}

impl ToTokens for Filter {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        use self::Filter::*;
        tokens.append_all(match self {
            Cond(cond) => quote!(ledb::Filter::Cond(#cond)),
            Comp(field, comp) => quote!(ledb::Filter::Comp(#field, #comp)),
        });
    }
}

#[cfg(test)]
mod test {
    use super::Filter;
    use syn::parse_str;

    #[test]
    fn eq() {
        parse_str::<Filter>("field == \"abc\"").unwrap();
    }

    #[test]
    fn ge() {
        parse_str::<Filter>("field >= 123").unwrap();
    }

    #[test]
    fn nested() {
        parse_str::<Filter>("field.subfield < 123").unwrap();
    }

    #[test]
    fn or() {
        parse_str::<Filter>("some.field == \"abc\" || other.field > 123").unwrap();
    }

    #[test]
    fn and() {
        parse_str::<Filter>("some.field == \"abc\" && other.field > 123").unwrap();
    }

    #[test]
    fn not() {
        parse_str::<Filter>("! field == \"abc\"").unwrap();
        parse_str::<Filter>("!(field == \"abc\")").unwrap();
        parse_str::<Filter>("field != \"abc\"").unwrap();
    }
}
