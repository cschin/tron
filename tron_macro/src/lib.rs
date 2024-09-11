use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields};


fn dup_struct(ast: DeriveInput) -> proc_macro2::TokenStream {
    let struct_name = &ast.ident;

    // Extract fields from the AST
    let fields = match ast.data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => &fields.named,
                _ => panic!("Expected named fields"),
            }
        },
        _ => panic!("Expected a struct"),
    };

    // Generate struct initialization
    let init_fields = fields.iter().map(|f| {
        let name = &f.ident;
        quote! { #name: self.#name }
    });

    let struct_init = quote! {
        #struct_name {
            #(#init_fields),*
        }
    };

    struct_init
}


/// Procedural macro to derive the `TnComponentBaseTrait` for a custom component.
///
/// This procedural macro generates an implementation of the `TnComponentBaseTrait` trait for a custom
/// component struct, allowing it to inherit base functionalities such as accessing component ID,
/// type, attributes, headers, value, state, assets, children, parent, and script.
///
/// # Arguments
///
/// * `input`: Token stream representing the input Rust code.
///
/// # Example
///
/// ```
/// // Usage example:
/// #[derive(ComponentBase)]
/// struct MyComponent {
///     // Implementations...
/// }
/// ```
#[proc_macro_derive(ComponentBase)]
pub fn component_base_macro_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    // Build the trait implementation
    let name = &ast.ident;
    let builder_name = format_ident!("{}Builder", name);
    let mut builder_ast = ast.clone();
    builder_ast.ident = builder_name.clone();
    let dup_struct = dup_struct(ast.clone());    

    let gen = quote! {
        impl<'a> TnComponentBaseTrait<'a> for #name<'a> where 'a:'static {

            fn tron_id(&self) -> &String {
                self.base.tron_id()
            }

            fn get_type(&self) -> TnComponentType {
                self.base.get_type()
            }

            fn attributes(&self) -> &TnElmAttributes {
                self.base.attributes()
            }

            fn set_attribute(&mut self, key: &str, val: &str) {
                self.base
                    .attributes
                    .insert(key.into(), val.into());
            }

            fn remove_attribute(&mut self, key: &str) {
                self.base
                    .attributes
                    .remove(key);
            }

            fn extra_headers(&self) -> &TnExtraResponseHeader {
                self.base.extra_headers()
            }

            fn set_header(&mut self, key: &str, val: (String, bool)) {
                self.base
                    .extra_response_headers
                    .insert(key.into(), val);
            }

            fn remove_header(&mut self, key: &str) {
                self.base
                    .extra_response_headers
                    .remove(key.into());
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

            fn create_assets(&mut self) {
                self.base.create_assets();
            }

            fn get_assets(&self) -> Option<&HashMap<String, TnAsset>> {
                self.base.get_assets()
            }


            fn get_mut_assets(&mut self) -> Option<&mut HashMap<String, TnAsset>> {
                self.base.get_mut_assets()
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

            fn set_action(&mut self, m: TnActionExecutionMethod, f: TnActionFn ) {
                self.base.action = Some( (m, f) )
            }

            fn get_action(&self) -> &Option<(TnActionExecutionMethod, TnActionFn)> {
                &self.base.action
            }

        }

        impl <'a:'static> TnComponentBaseRenderTrait<'a> for #name <'a> {} 

        #[derive(Default)]
        #builder_ast

        impl<'a> #name <'a> where 'a:'static {
            // This method will help users to discover the builder
            pub fn builder() -> #builder_name<'static>  {
                 #builder_name::default()
            }
        }
        
        impl<'a:'static> #builder_name<'a>  {
   
            pub fn set_action(mut self, m: TnActionExecutionMethod, f: TnActionFn) -> #builder_name<'static> {
                self.base.set_action(m, f);
                self
            }
            
            pub fn set_attribute(mut self, key: &str, val: &str) -> #builder_name<'static> {
                self.base
                    .attributes
                    .insert(key.to_string(), val.to_string());
                self
            }

            pub fn build(self) -> #name<'a> {
                #dup_struct
            } 

            pub fn add_to_context(self, context: &mut TnContextBase) {
                context.add_component(#dup_struct);
            }
           
        }


    };
    gen.into()
}