use proc_macro::TokenStream;
use quote::quote;
use syn::{FnArg, ItemFn, LitStr, Pat, parse_macro_input};

#[proc_macro_attribute]
pub fn danneotest(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);

    // Parse module name from attribute if provided: #[danneotest("my_module")]
    let module_name = if !attr.is_empty() {
        let attr_tokens: proc_macro2::TokenStream = attr.into();
        match syn::parse2::<LitStr>(attr_tokens) {
            Ok(lit) => lit.value(),
            Err(_) => "".to_string(),
        }
    } else {
        "".to_string()
    };

    let vis = &input.vis;
    let attrs = &input.attrs;
    let mut sig = input.sig.clone();
    let body = &input.block;

    let mut state_arg_name = None;
    if let Some(FnArg::Typed(pat_type)) = sig.inputs.first() {
        if let Pat::Ident(pat_ident) = &*pat_type.pat {
            state_arg_name = Some(pat_ident.ident.clone());
        }
    }

    // Prepare signature for #[tokio::test] (no arguments)
    sig.inputs.clear();

    let state_init = if let Some(arg_name) = state_arg_name {
        quote! {
            let #arg_name = danneo_core::cli::test_runner::TestRunner::boot_test_environment(#module_name).await
                .expect("Failed to boot test environment");
        }
    } else {
        quote! {
            let _state = danneo_core::cli::test_runner::TestRunner::boot_test_environment(#module_name).await
                .expect("Failed to boot test environment");
        }
    };

    let expanded = quote! {
        #[tokio::test]
        #(#attrs)*
        #vis #sig {
            #state_init

            // Wrap in a block to allow the user's code to run in its own scope
            {
                #body
            }
        }
    };

    TokenStream::from(expanded)
}
