pub mod button;

use std::collections::HashMap;
use tokio::sync::Mutex;
use tron_macro::*;

pub use button::TnButton;
use rand::{thread_rng, Rng};

use axum::response::Html;

pub type ComponentId = u32;
pub type ElmAttributes = HashMap<String, String>;
pub type ElmTag = String;

#[derive(Debug)]
pub struct ComponentBase<V, A> {
    pub tag: ElmTag,
    pub id: ComponentId,
    pub tron_id: String,
    pub attributes: ElmAttributes,
    pub value: Option<V>,
    pub assets: Option<HashMap<String, A>>,
    pub children_targets: Option<Vec<ComponentId>>, // general storage
}

pub enum ComponentTypes {
    TnButton(TnButton),
}
pub struct ApplicationStates {
    pub components: HashMap<u32, ComponentTypes>, // component ID mapped to Component structs
    pub assets: HashMap<String, Vec<u8>>,
}

pub trait ComponentBaseTrait {
    fn id(&self) -> ComponentId;
    fn tron_id(&self) -> &String;
    fn attributes(&self) -> &ElmAttributes;
    fn children_targets(&self) -> &Option<Vec<ComponentId>>;
    fn set_attribute(&mut self, key: String, value: String) -> &mut Self;
    fn generate_attr_string(&self) -> String;
}

pub trait ComponentValueTrait<V> {
    fn value(&self) -> &Option<V>;
    fn set_value(&mut self, value: V) -> &Self;
}
pub trait ComponentAssetTrait<A> {
    fn assets(&self) -> &Option<HashMap<String, A>>;
}

pub trait ComponentRenderTraits {
    fn render(&self) -> Html<String>;
}

impl<V, A> ComponentBase<V, A> {
    pub fn new(tag: ElmTag, id: ComponentId, tron_id: String) -> Self {
        Self {
            tag,
            id,
            tron_id,
            attributes: HashMap::default(),
            value: None,
            assets: None,
            children_targets: None,
        }
    }
}

impl<V, A> Default for ComponentBase<V, A> {
    fn default() -> ComponentBase<V, A> {
        let mut rng = thread_rng();
        let id: u32 = rng.gen();
        let tron_id = format!("{:x}", id);
        ComponentBase {
            tag: "div".to_string(),
            id,
            tron_id,
            attributes: HashMap::default(),
            value: None,
            assets: None,
            children_targets: None,
        }
    }
}

impl<V, A> ComponentBaseTrait for ComponentBase<V, A> {
    fn id(&self) -> u32 {
        self.id
    }
    fn tron_id(&self) -> &String {
        &self.tron_id
    }

    fn attributes(&self) -> &ElmAttributes {
        &self.attributes
    }

    fn children_targets(&self) -> &Option<Vec<u32>> {
        &self.children_targets
    }

    fn set_attribute(&mut self, key: String, val: String) -> &mut Self {
        self.attributes.insert(key, val);
        self
    }
    fn generate_attr_string(&self) -> String {
        self.attributes
            .iter()
            .map(|(k, v)| format!(r#"{}="{}""#, k, v))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl<V, A> ComponentValueTrait<V> for ComponentBase<V, A> {
    fn value(&self) -> &Option<V> {
        &self.value
    }
    fn set_value(&mut self, value: V) -> &Self {
        self.value = Some(value);
        self
    }
}

impl<V, A> ComponentAssetTrait<A> for ComponentBase<V, A> {
    fn assets(&self) -> &Option<HashMap<String, A>> {
        &self.assets
    }
}

impl<V, A> ComponentRenderTraits for ComponentBase<V, A> {
    fn render(&self) -> Html<String> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use crate::{ComponentBaseTrait, ComponentValueTrait, TnButton};

    #[test]
    fn test_simple_button() {
        let mut btn = TnButton::new(12, "12".to_string(), "12".to_string());
        btn.set_attribute("hx-get".to_string(), format!("/tron/{}", 12));
        //println!("{}", btn.generate_hx_attr_string());
    }
}
