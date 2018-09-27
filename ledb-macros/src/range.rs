use types::*;
use value::*;

/// Range point origin
pub enum Orig {
    /// from beginning
    Begin,
    /// from end
    End,
}

/// Range point direction
pub enum Dir {
    /// incrementing
    Up,
    /// decrementing
    Down,
}

/// Range point border
pub enum Bord {
    /// include boint
    Incl,
    /// exclude point
    Excl,
}

/// Range point without border
pub struct RangePoint(Orig, Dir, KeyData);

/// Range point with border
pub struct RangePointWithBord(RangePoint, Bord);

impl RangePoint {
    /// Add border to range point
    pub fn with_bord(self, bord: Bord) -> RangePointWithBord {
        RangePointWithBord(self, bord)
    }
}

impl Parse for RangePoint {
    fn parse(input: ParseStream) -> Result<Self> {
        use self::Dir::*;
        use self::Orig::*;

        let orig = if input.peek(Token![#]) {
            input.parse::<Token![#]>()?;
            End
        } else {
            Begin
        };

        let dir = if input.peek(Token![-]) {
            input.parse::<Token![-]>()?;
            Down
        } else if input.peek(Token![+]) {
            input.parse::<Token![+]>()?;
            Up
        } else {
            Up
        };

        let expr = input.parse()?;

        Ok(RangePoint(orig, dir, expr))
    }
}

impl ToTokens for RangePoint {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        use self::Dir::*;
        use self::Orig::*;

        let RangePoint(orig, dir, val) = self;

        tokens.append_all(match (orig, dir) {
            (Begin, Up) => quote!{},
            (Begin, Down) => quote! { - },
            (End, Up) => quote! { -1 + },
            (End, Down) => quote! { -1 - },
        });

        val.to_tokens(tokens)
    }
}

impl ToTokens for RangePointWithBord {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        use self::Bord::*;

        let RangePointWithBord(point, bord) = self;

        point.to_tokens(tokens);

        tokens.append_all(match bord {
            Incl => quote! { , true },
            Excl => quote! { , false },
        })
    }
}

/// Range bound
pub struct RangeBound(pub RangePoint, pub RangePoint);

impl Parse for RangeBound {
    fn parse(input: ParseStream) -> Result<Self> {
        let a = input.parse()?;
        input.parse::<Token![..]>()?;
        let b = input.parse()?;
        Ok(RangeBound(a, b))
    }
}
