use std::fmt;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse::{Parse, ParseStream}, token::Comma, LitInt};


struct Interface7zipAttributes {
    pub group: u16,
    pub item: u16,
}
impl Parse for Interface7zipAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let group_lit: LitInt = input.parse()?;
        let group = group_lit.base10_parse()?;

        let _: Comma = input.parse()?;

        let item_lit: LitInt = input.parse()?;
        let item = item_lit.base10_parse()?;

        Ok(Self {
            group,
            item,
        })
    }
}
impl fmt::Display for Interface7zipAttributes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "23170F69-40C1-278A-0000-{:04X}{:04X}0000", self.group, self.item)
    }
}


#[proc_macro_attribute]
pub fn interface_7zip(
    attributes: proc_macro::TokenStream,
    original_type: proc_macro::TokenStream,
) -> TokenStream {
    let parsed_attributes = syn::parse_macro_input!(attributes as Interface7zipAttributes);
    let guid_string = parsed_attributes.to_string();

    let new_attributes = quote! {
        #[::windows_core::interface(#guid_string)]
    };

    let mut output = proc_macro::TokenStream::new();
    output.extend(TokenStream::from(new_attributes.into_token_stream()));
    output.extend(original_type);

    output
}
