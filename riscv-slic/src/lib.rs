use proc_macro::TokenStream;
use proc_macro2::{Group, Ident, TokenStream as TokenStream2, TokenTree};
use quote::quote;

mod export;
mod exti;
mod slic;

/// Helper function to parse groups as vector of identities
fn group_to_idents(input: Group) -> Vec<Ident> {
    let input_iterator = input.stream().into_iter();

    let mut idents: Vec<Ident> = Vec::new();
    // Even tokens must be interrupt source identifiers, and odd tokens must be commas
    for (i, token) in input_iterator.enumerate() {
        if i % 2 == 0 {
            if let TokenTree::Ident(ident) = token {
                idents.push(ident);
                continue;
            }
            panic!("invalid input; must be interrupt idents separated by comma");
        } else {
            if let TokenTree::Punct(punct) = &token {
                if punct.as_char() == ',' {
                    continue;
                }
            }
            panic!("invalid input; must be interrupt idents separated by comma");
        }
    }
    idents
}

// Ex. codegen!(pac, [HW1, HW2], [SW1, SW2])
// Ex. codegen!(e310x, [GPIO1, RTC], [Task1, Task2])
#[proc_macro]
pub fn codegen(input: TokenStream) -> TokenStream {
    let input: TokenStream2 = input.into();
    let mut input_iterator = input.into_iter();

    // Get the device PAC
    let pac = match input_iterator.next() {
        Some(TokenTree::Ident(ident)) => Some(ident),
        _ => None,
    };
    let pac = pac.unwrap();

    // Consume the comma separator
    let separator = match input_iterator.next() {
        Some(TokenTree::Punct(punct)) => Some(punct.as_char()),
        _ => None,
    };
    assert_eq!(separator.unwrap(), ',');
    // Get the hw handlers
    let hw_handlers = match input_iterator.next() {
        Some(TokenTree::Group(array)) => Some(array),
        _ => None,
    };
    let hw_handlers = group_to_idents(hw_handlers.unwrap());

    // Consume the comma separator
    let separator = match input_iterator.next() {
        Some(TokenTree::Punct(punct)) => Some(punct.as_char()),
        _ => None,
    };
    assert_eq!(separator.unwrap(), ',');
    // Get the sw handlers
    let sw_handlers = match input_iterator.next() {
        Some(TokenTree::Group(array)) => Some(array),
        _ => None,
    };
    let sw_handlers = group_to_idents(sw_handlers.unwrap());
    // Assert that we reached the end
    assert!(input_iterator.next().is_none());

    let swi_export = export::export_swi(&pac);
    let slic_code = slic::slic_mod(&hw_handlers, &sw_handlers);

    let exti_export = export::export_exti(&pac);
    let exti_code = exti::exti_mod(&pac, &hw_handlers);

    quote! {
        pub mod slic {
            use heapless::binary_heap::{BinaryHeap, Max};
            #swi_export
            #slic_code
            #exti_export
            #exti_code
        }
    }
    .into()
}
