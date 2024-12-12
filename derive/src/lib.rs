use darling::FromDeriveInput;
use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[derive(FromDeriveInput, Default)]
#[darling(default, attributes(job_default))]
struct OptionalDefaultOpts {
    is_default: Option<bool>,
}

#[proc_macro_derive(OptionalDefault, attributes(job_default))]
/// Derive macro generating an impl of the trait `JobDefault`.
///
/// By default it will return `Some` of itself. To make the job non-default, use the `job_default`
/// attribute with a value for `is_default`
///
/// ```ignore
/// #[derive(Default, JobDefault)]
/// #[job_default(is_default = false)]
/// struct MyJob {}
/// ```
///
/// # Panics
/// The macro will not complete if invalid options are provided to the `job_default` attribute.
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);
    let opts = OptionalDefaultOpts::from_derive_input(&input).expect("Invalid options");
    let DeriveInput {
        ident, generics, ..
    } = input;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let optional_default = opts.is_default.unwrap_or(true);

    let default = if optional_default {
        quote! { Some(Box::new(Default::default())) }
    } else {
        quote! { None }
    };

    let output = quote! {
        impl #impl_generics JobDefault for #ident #ty_generics #where_clause {
            fn optional_default() -> Option<Box<#ident #ty_generics>> {
                #default
            }
        }
    };
    output.into()
}
