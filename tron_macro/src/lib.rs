use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_derive(ComponentBase)]
pub fn component_base_macro_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    // Build the trait implementation
    let name = &ast.ident;
    let gen = quote! {
        impl<'a> ComponentBaseTrait<'a> for #name<'a> {
            fn id(&self) -> ComponentId {
                self.inner.id()
            }
            fn tron_id(&self) -> &String {
                self.inner.tron_id()
            }

            fn attributes(&self) -> &ElmAttributes {
                self.inner.attributes()
            }

            fn set_attribute(&mut self, key: String, val: String) {
                self.inner
                    .attributes
                    .insert(key, val);
            }

            fn generate_attr_string(&self) -> String {
                self.inner.generate_attr_string()
            }

            fn value(&self) -> &ComponentValue {
                &self.inner.value()
            }

            fn set_value(&mut self, new_value: ComponentValue) {
                self.inner.set_value(new_value);
            }

            fn state(&self) -> &ComponentState {
                &self.inner.state()
            }

            fn set_state(&mut self, new_state: ComponentState) {
                self.inner.set_state(new_state);
            }

            fn get_assets(&self) -> Option<&HashMap<String, TnAsset>> {
                self.inner.get_assets()
            }


            fn get_mut_assets(&mut self) -> Option<&mut HashMap<String, TnAsset>> {
                self.inner.get_mut_assets()
            }

            fn render(&self) -> Html<String> {
                self.internal_render()
            }

            fn render_to_string(&self) -> String {
                self.render().0
            }

            fn get_children(&self) -> Option<&Vec<&'a ComponentBase<'a>>> {
                self.inner.get_children()
            }

        }
    };
    gen.into()
}
