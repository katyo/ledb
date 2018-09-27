use range::*;
use types::*;
use value::*;

/// Modification statement
pub enum Action {
    /// field = expr
    Set(ValData),
    /// field ~
    Delete,
    /// field += expr
    Add(ValData),
    /// field -= expr
    Sub(ValData),
    /// field *= expr
    Mul(ValData),
    /// field /= expr
    Div(ValData),
    /// field !
    Toggle,
    /// field ~= pat, sub
    Replace(Expr, Expr),
    /// field[a..b] = [...expr]
    Splice(RangePoint, RangePoint, ValData),
    /// field ~= expr
    Merge(ValData),
}

impl Parse for Action {
    fn parse(input: ParseStream) -> Result<Self> {
        use self::Action::*;

        let lookahead = input.lookahead1();

        if lookahead.peek(Token![=]) {
            // field = expr
            input.parse::<Token![=]>()?;
            input.parse().map(Set)
        } else if lookahead.peek(Token![-]) {
            // field - (delete)
            input.parse::<Token![-]>().map(|_| Delete)
        } else if lookahead.peek(Token![~]) {
            // field ~ ...
            input.parse::<Token![~]>()?;
            if input.peek(Token![=]) {
                // field ~= ...
                input.parse::<Token![=]>()?;
                if input.peek(LitStr) {
                    // field ~= pat sub (replace)
                    let pat = input.parse()?;
                    let sub = input.parse()?;
                    Ok(Replace(pat, sub))
                } else {
                    // field ~= obj (merge)
                    input.parse().map(Merge)
                }
            } else {
                // field ~ (delete)
                Ok(Delete)
            }
        } else if lookahead.peek(Token![+=]) {
            // field += expr
            input.parse::<Token![+=]>()?;
            input.parse().map(Add)
        } else if lookahead.peek(Token![-=]) {
            // field -= expr
            input.parse::<Token![-=]>()?;
            input.parse().map(Sub)
        } else if lookahead.peek(Token![*=]) {
            // field *= expr
            input.parse::<Token![*=]>()?;
            input.parse().map(Mul)
        } else if lookahead.peek(Token![/=]) {
            // field /= expr
            input.parse::<Token![/=]>()?;
            input.parse().map(Div)
        } else if lookahead.peek(Token![!]) {
            // field !
            input.parse::<Token![!]>().map(|_| Toggle)
        } else if lookahead.peek(Bracket) {
            // field[..]
            let slice;
            bracketed!(slice in input);
            let RangeBound(beg, end) = slice.parse()?;
            let lookahead = input.lookahead1();
            let sub = if lookahead.peek(Token![-]) {
                // field[..] -
                input.parse::<Token![-]>()?;
                parse_quote! { vec![] }
            } else if lookahead.peek(Token![~]) {
                // field[..] ~
                input.parse::<Token![~]>()?;
                parse_quote! { vec![] }
            } else if lookahead.peek(Token![=]) {
                // field[..] = values
                input.parse::<Token![=]>()?;
                parse_opt_bracketed_expr(input)?
            } else {
                return Err(lookahead.error());
            };
            Ok(Splice(beg, end, sub))
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for Action {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        use self::Action::*;

        tokens.append_all(match self {
            Set(expr) => quote! { ledb::Action::Set(#expr) },
            Delete => quote! { ledb::Action::Delete },
            Add(expr) => quote! { ledb::Action::Add(#expr) },
            Sub(expr) => quote! { ledb::Action::Sub(#expr) },
            Mul(expr) => quote! { ledb::Action::Mul(#expr) },
            Div(expr) => quote! { ledb::Action::Div(#expr) },
            Toggle => quote! { ledb::Action::Toggle },
            Replace(pat, sub) => quote! { ledb::Action::Replace(ledb::WrappedRegex(#pat.parse().unwrap()), String::from(#sub)) },
            Splice(beg, end, sub) => quote! { ledb::Action::Splice(#beg, #end, #sub) },
            Merge(expr) => quote! { ledb::Action::Merge(#expr) },
        })
    }
}

fn parse_opt_bracketed_expr(input: ParseStream) -> Result<ValData> {
    if input.peek(Bracket) {
        let exprs;
        bracketed!(exprs in input);
        let exprs = Punctuated::<Expr, Comma>::parse_terminated(&exprs)?;
        Ok(parse_quote! { vec![#(#exprs),*] })
    } else {
        input.parse()
    }
}
