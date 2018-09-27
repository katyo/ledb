use range::*;
use types::*;
use value::*;

/// Compare expression
pub enum Comp {
    /// field == expr
    Eq(KeyData),
    /// field != expr
    Ne(KeyData),
    /// field in [...exprs]
    In(KeyData),
    /// field !in [...exprs]
    Ni(KeyData),
    /// field < expr
    Lt(KeyData),
    /// field <= expr
    Le(KeyData),
    /// field > expr
    Gt(KeyData),
    /// field >= expr
    Ge(KeyData),
    /// field in a..b
    Bw(RangePointWithBord, RangePointWithBord),
    /// field ?
    Has,
}

impl Comp {
    pub fn not_not(self) -> (Bool, Self) {
        use self::Comp::*;
        match self {
            Ne(expr) => (true, Eq(expr)),
            Ni(expr) => (true, In(expr)),
            comp => (false, comp),
        }
    }
}

impl Parse for Comp {
    fn parse(input: ParseStream) -> Result<Self> {
        use self::Comp::*;
        let lookahead = input.lookahead1();
        if lookahead.peek(Token![==]) {
            // field == value
            input.parse::<Token![==]>()?;
            input.parse().map(Eq)
        } else if lookahead.peek(Token![!=]) {
            // field != value
            input.parse::<Token![!=]>()?;
            input.parse().map(Ne)
        } else if lookahead.peek(Token![in]) {
            // field in ...values
            input.parse::<Token![in]>()?;
            if input.peek(Bracket) {
                // field in [value1, ..., valueN]
                parse_set_from_in(input)
            } else if is_range_from_in(input) {
                parse_range_from_in(input, Bord::Incl)
            } else {
                // field in values
                input.parse().map(In)
            }
        } else if lookahead.peek(Token![!]) {
            // field ! ...
            input.parse::<Token![!]>()?;
            let lookahead = input.lookahead1();
            if lookahead.peek(Token![in]) {
                // field !in ...values
                input.parse::<Token![in]>()?;
                if input.peek(Bracket) {
                    // field !in [value1, ..., valueN]
                    parse_set_from_in(input)
                } else {
                    // field !in values
                    input.parse().map(Ni)
                }
            } else {
                Err(lookahead.error())
            }
        } else if lookahead.peek(Token![<]) {
            if input.peek(Token![in]) {
                // field <in a..b
                input.parse::<Token![in]>()?;
                if is_range_from_in(input) {
                    parse_range_from_in(input, Bord::Excl)
                } else {
                    Err(input.error("Range expression expected"))
                }
            } else {
                // field < val
                input.parse::<Token![<]>()?;
                input.parse().map(Lt)
            }
        } else if lookahead.peek(Token![<=]) {
            // field <= val
            input.parse::<Token![<=]>()?;
            input.parse().map(Le)
        } else if lookahead.peek(Token![<]) {
            // field > val
            input.parse::<Token![>]>()?;
            input.parse().map(Gt)
        } else if lookahead.peek(Token![>=]) {
            // field >= val
            input.parse::<Token![>=]>()?;
            input.parse().map(Ge)
        } else if lookahead.peek(Token![?]) {
            // field ?
            input.parse::<Token![?]>().map(|_| Has)
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for Comp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        use self::Comp::*;
        tokens.append_all(match self {
            Eq(expr) => quote! { ledb::Comp::Eq(#expr) },
            Ne(_expr) => unreachable!(),
            In(expr) => quote! { ledb::Comp::In(#expr) },
            Ni(_expr) => unreachable!(),
            Lt(expr) => quote! { ledb::Comp::Lt(#expr) },
            Le(expr) => quote! { ledb::Comp::Le(#expr) },
            Gt(expr) => quote! { ledb::Comp::Gt(#expr) },
            Ge(expr) => quote! { ledb::Comp::Ge(#expr) },
            Bw(a, b) => quote! { ledb::Comp::Bw(#a, #b) },
            Has => quote! { ledb::Comp::Has },
        });
    }
}

fn parse_set_from_in(input: ParseStream) -> Result<Comp> {
    use self::Comp::*;
    let items;
    bracketed!(items in input);
    Punctuated::<Expr, Comma>::parse_terminated(&items)
        .map(|exprs| In(parse_quote! { vec![#(#exprs),*] }))
}

fn is_range_from_in(input: ParseStream) -> Bool {
    (input.peek2(Token![>]) || input.peek2(Token![#]) || input.peek3(Token![..]))
}

fn parse_range_from_in(input: ParseStream, bord_a: Bord) -> Result<Comp> {
    use self::Comp::*;
    let bord_b = if input.peek(Token![>]) {
        input.parse::<Token![>]>()?;
        Bord::Excl
    } else {
        Bord::Incl
    };
    input
        .parse()
        .map(|RangeBound(a, b): RangeBound| Bw(a.with_bord(bord_a), b.with_bord(bord_b)))
}

#[cfg(test)]
mod test {
    use super::Comp;
    use syn::parse_str;

    #[test]
    fn literal_expr() {
        parse_str::<Comp>("== \"abc\"").unwrap();
        parse_str::<Comp>("== 123").unwrap();
        parse_str::<Comp>("== true").unwrap();
        parse_str::<Comp>("== var").unwrap();
    }

    #[test]
    fn nested_expr() {
        parse_str::<Comp>("== (1+2-3)").unwrap();
        parse_str::<Comp>("== {1+2-3}").unwrap();
    }

    #[test]
    fn invalid_expr() {
        parse_str::<Comp>("== (1+2-3)").unwrap();
    }
}
