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
        impl<'a> ComponentBaseTrait<'a> for #name<'a> where 'a:'static {
            fn id(&self) -> ComponentId {
                self.inner.id()
            }

            fn tron_id(&self) -> &String {
                self.inner.tron_id()
            }

            fn get_type(&self) -> TnComponentType {
                self.inner.get_type()
            }

            fn attributes(&self) -> &ElmAttributes {
                self.inner.attributes()
            }

            fn set_attribute(&mut self, key: String, val: String) {
                self.inner
                    .attributes
                    .insert(key, val);
            }

            fn remove_attribute(&mut self, key: String) {
                self.inner
                    .attributes
                    .remove(&key);
            }

            fn extra_headers(&self) -> &ExtraResponseHeader {
                self.inner.extra_headers()
            }

            fn set_header(&mut self, key: String, val: String) {
                self.inner
                    .extra_response_headers
                    .insert(key, val);
            }

            fn remove_header(&mut self, key: String) {
                self.inner
                    .extra_response_headers
                    .remove(&key);
            }


            fn generate_attr_string(&self) -> String {
                self.inner.generate_attr_string()
            }

            fn value(&self) -> &ComponentValue {
                &self.inner.value()
            }

            fn get_mut_value(&mut self) -> &mut ComponentValue {
                self.inner.get_mut_value()
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

            fn first_render(&self) -> String {
                self.internal_first_render()
            }

            fn render(&self) -> String {
                self.internal_render()
            }

            fn get_children(&self) -> &Vec<Arc<RwLock<Box<dyn ComponentBaseTrait<'a>>>>>  {
                self.inner.get_children()
            }

            fn add_child(&mut self, child: Arc<RwLock<Box<dyn ComponentBaseTrait<'a>>>>) {
                self.inner.add_child(child);
            }

            fn add_parent(&mut self, parent: Arc<RwLock<Box<dyn ComponentBaseTrait<'a>>>>) {
                self.inner.add_parent(parent);
            }

            fn get_parent(&self) -> Arc<RwLock<Box<dyn ComponentBaseTrait<'a>>>> {
                self.inner.get_parent()
            }

            fn get_script(&self) -> Option<String> {
                self.inner.get_script()
            }
        }
    };
    gen.into()
}
