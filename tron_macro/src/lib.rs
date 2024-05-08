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
        impl<'a> TnComponentBaseTrait<'a> for #name<'a> where 'a:'static {
            fn id(&self) -> TnComponentIndex {
                self.base.id()
            }

            fn tron_id(&self) -> &String {
                self.base.tron_id()
            }

            fn get_type(&self) -> TnComponentType {
                self.base.get_type()
            }

            fn attributes(&self) -> &TnElmAttributes {
                self.base.attributes()
            }

            fn set_attribute(&mut self, key: String, val: String) {
                self.base
                    .attributes
                    .insert(key, val);
            }

            fn remove_attribute(&mut self, key: String) {
                self.base
                    .attributes
                    .remove(&key);
            }

            fn extra_headers(&self) -> &TnExtraResponseHeader {
                self.base.extra_headers()
            }

            fn set_header(&mut self, key: String, val: (String, bool)) {
                self.base
                    .extra_response_headers
                    .insert(key, val);
            }

            fn remove_header(&mut self, key: String) {
                self.base
                    .extra_response_headers
                    .remove(&key);
            }

            fn clear_header(&mut self) {
                self.base.extra_response_headers.clear();
            }

            fn generate_attr_string(&self) -> String {
                self.base.generate_attr_string()
            }

            fn value(&self) -> &TnComponentValue {
                &self.base.value()
            }

            fn get_mut_value(&mut self) -> &mut TnComponentValue {
                self.base.get_mut_value()
            }
            fn set_value(&mut self, new_value: TnComponentValue) {
                self.base.set_value(new_value);
            }

            fn state(&self) -> &TnComponentState {
                &self.base.state()
            }

            fn set_state(&mut self, new_state: TnComponentState) {
                self.base.set_state(new_state);
            }

            fn get_assets(&self) -> Option<&HashMap<String, TnAsset>> {
                self.base.get_assets()
            }


            fn get_mut_assets(&mut self) -> Option<&mut HashMap<String, TnAsset>> {
                self.base.get_mut_assets()
            }

            fn first_render(&self) -> String {
                self.internal_first_render()
            }

            fn render(&self) -> String {
                self.internal_render()
            }

            fn get_children(&self) -> &Vec<TnComponent<'a>>  {
                self.base.get_children()
            }

            fn get_mut_children(&mut self) -> &mut Vec<TnComponent<'a>>  {
                self.base.get_mut_children()
            }

            fn add_child(&mut self, child: TnComponent<'a>) {
                self.base.add_child(child);
            }

            fn add_parent(&mut self, parent: TnComponent<'a>) {
                self.base.add_parent(parent);
            }

            fn get_parent(&self) -> TnComponent<'a> {
                self.base.get_parent()
            }

            fn get_script(&self) -> Option<String> {
                self.base.get_script()
            }
        }
    };
    gen.into()
}
