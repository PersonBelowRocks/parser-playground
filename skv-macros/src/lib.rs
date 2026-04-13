extern crate proc_macro;

use proc_macro2::{Span, TokenStream};
use skv_core::EnumString;
use syn::{Data, DeriveInput, parse_macro_input, spanned::Spanned};

fn comp_error(span: Span, message: impl std::fmt::Display) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(syn::Error::new(span, message).to_compile_error())
}

#[proc_macro_derive(SkvEnum)]
pub fn derive_skv_enum(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let Data::Enum(ref data) = input.data else {
        return comp_error(input.ident.span(), "SkvEnum can only be derived for enums");
    };

    let mut str_to_enum_branches: Vec<TokenStream> = Vec::new();
    let mut enum_to_str_branches: Vec<TokenStream> = Vec::new();
    let mut variant_names: Vec<TokenStream> = Vec::new();

    for variant in data.variants.iter() {
        // ensure there are no fields
        if !variant.fields.is_empty() {
            return comp_error(
                variant.span(),
                "SkvEnum cannot be derived for enums with fields",
            );
        }

        // ensure (lowercase) variant name is a valid enum string
        let lowered = format!("{}", variant.ident).to_ascii_lowercase();
        let Ok(enum_string) = lowered.parse::<EnumString>() else {
            return comp_error(
                variant.ident.span(),
                format!("invalid variant name for SkvEnum: {lowered}"),
            );
        };

        let serialized_name = enum_string.as_ref().to_string();
        let variant_ident = &variant.ident;

        str_to_enum_branches.push(quote::quote!(#serialized_name => Some(Self::#variant_ident)));
        enum_to_str_branches.push(quote::quote!(Self::#variant_ident => <EnumString as std::str::FromStr>::from_str(#serialized_name).unwrap()));
        variant_names.push(quote::quote!(#serialized_name))
    }

    let name = input.ident;

    proc_macro::TokenStream::from(quote::quote!(
    unsafe impl ::skv_core::SkvEnum for #name {
        const ENUM_STRINGS: &'static [&'static str] = &[#(#variant_names),*];

        #[inline]
        fn to_enum_string(&self) -> EnumString {
            match self {
                #(#enum_to_str_branches,)*
            }
        }

        #[inline]
        fn from_enum_string(s: &::skv_core::EnumString) -> Option<Self> {
            match s.as_ref().as_str() {
                #(#str_to_enum_branches,)*
                _ => None
            }
        }
    }
    ))
}
