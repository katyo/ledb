use proc_macro2::{Span, TokenStream};
use syn::{Ident};
use quote::quote;

pub fn wrap_in_const(
    trait_: &str,
    type_: &Ident,
    code: TokenStream,
) -> TokenStream {
    let dummy_const = Ident::new(
        &format!("_IMPL_{}_FOR_{}", trait_, unraw(&type_)),
        Span::call_site(),
    );

    let use_types = quote! {
        #[cfg_attr(feature = "cargo-clippy", allow(useless_attribute))]
        #[allow(rust_2018_idioms)]
        extern crate ledb_types as _ledb_types;
    };

    quote! {
        const #dummy_const: () = {
            #use_types
            #code
        };
    }
}

#[allow(deprecated)]
fn unraw(ident: &Ident) -> String {
    // str::trim_start_matches was added in 1.30, trim_left_matches deprecated
    // in 1.33. We currently support rustc back to 1.15 so we need to continue
    // to use the deprecated one.
    ident.to_string().trim_left_matches("r#").to_owned()
}
