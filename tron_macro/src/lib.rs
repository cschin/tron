use proc_macro::TokenStream;
use quote::quote;
use syn::{self, parse_macro_input, DeriveInput};

#[proc_macro_derive(ComponentBase)]
pub fn component_base_macro_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    // Build the trait implementation
    let name = &ast.ident;
    let gen = quote! {
        impl ComponentBaseTrait for #name {
            fn id(&self) -> ComponentId {
                self.component_base.id()
            }
            fn tron_id(&self) -> &String {
                self.component_base.tron_id()
            }
        
            fn attributes(&self) -> &ElmAttributes {
                self.component_base.attributes()
            }
            fn children_targets(&self) -> &Option<Vec<u32>> {
                self.component_base.children_targets()
            }
        
            fn set_attribute(&mut self, key: String, val: String) -> &mut Self {
                self.component_base
                    .attributes
                    .insert(key, val);
                self
            }
        
            fn generate_attr_string(&self) -> String {
                self.component_base
                    .attributes
                    .iter()
                    .map(|(k, v)| format!(r#"{}="{}""#, k, v))
                    .collect::<Vec<_>>()
                    .join(" ")
            }
        }
    };
    gen.into()
}


/// Not working yet, we need to parse the AST to get the template type parameter
#[proc_macro_derive(ComponentValue)]
pub fn component_value_macro_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    // Build the trait implementation
    let name = &ast.ident;
    let gen = quote! {
        impl ComponentValueTrait<T> for #name {
            fn value(&self) -> &Option<T> {
                self.component_base.value()
            }
            fn set_value(&mut self, value: T) -> &Self {
                self.component_base.value = Some(value);
                self
            }
        }
    };
    gen.into()
}