pub mod button;

use std::collections::HashMap;


pub use button::TnButton;
use rand::{thread_rng, Rng};

use axum::response::Html;

pub type ComponentId = u32;
pub type ElmAttributes = HashMap<String, String>;
pub type ElmTag = String;

#[derive(Debug)]
pub enum ComponentValue {
    None,
    String(String)
}
#[derive(Debug)]
pub enum ComponentAsset {
    None,
    VecU8(Vec<u8>)
}
#[derive(Debug)]
pub struct ComponentBase {
    pub tag: ElmTag,
    pub id: ComponentId,
    pub tron_id: String,
    pub attributes: ElmAttributes,
    pub value: ComponentValue,
    pub assets: Option<HashMap<String, ComponentAsset>>,
    pub children_targets: Option<Vec<ComponentId>>, // general storage
}

pub enum ComponentTypes {
    TnButton(TnButton),
}
pub struct ApplicationStates {
    pub components: HashMap<u32, Box<dyn ComponentBaseTrait>>, // component ID mapped to Component structs
    pub assets: HashMap<String, Vec<u8>>,
}


pub trait ComponentBaseTrait: Send + Sync {
    fn id(&self) -> ComponentId;
    fn tron_id(&self) -> &String;
    fn attributes(&self) -> &ElmAttributes;
    fn children_targets(&self) -> &Option<Vec<ComponentId>>;
    fn set_attribute(&mut self, key: String, value: String);
    fn generate_attr_string(&self) -> String;
    fn value(&self) -> &ComponentValue;
    fn set_value(&mut self, value: ComponentValue);
    fn assets(&self) -> &Option<HashMap<String, ComponentAsset>>;
    fn render(&self) -> Html<String>;
}



impl ComponentBase {
    pub fn new(tag: ElmTag, id: ComponentId, tron_id: String) -> Self {
        Self {
            tag,
            id,
            tron_id,
            attributes: HashMap::default(),
            value: ComponentValue::None,
            assets: None,
            children_targets: None,
        }
    }
}

impl Default for ComponentBase {
    fn default() -> ComponentBase {
        let mut rng = thread_rng();
        let id: u32 = rng.gen();
        let tron_id = format!("{:x}", id);
        ComponentBase {
            tag: "div".to_string(),
            id,
            tron_id,
            attributes: HashMap::default(),
            value: ComponentValue::None,
            assets: None,
            children_targets: None,
        }
    }
}

impl ComponentBaseTrait for ComponentBase {
    
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

    fn set_attribute(&mut self, key: String, val: String) {
        self.attributes.insert(key, val);
    }

    fn generate_attr_string(&self) -> String {
        self.attributes
            .iter()
            .map(|(k, v)| format!(r#"{}="{}""#, k, v))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn value(&self) -> &ComponentValue {
        &self.value
    }

    fn set_value(&mut self, _value: ComponentValue) {
        unimplemented!()
    }

    fn assets(&self) -> &Option<HashMap<String, ComponentAsset>> {
        &self.assets
    }

    fn render(&self) -> Html<String> {
        unimplemented!()
    }
}



#[cfg(test)]
mod tests {
    use crate::{ComponentBaseTrait, TnButton};

    #[test]
    fn test_simple_button() {
        let mut btn = TnButton::new(12, "12".to_string(), "12".to_string());
        btn.set_attribute("hx-get".to_string(), format!("/tron/{}", 12));
        //println!("{}", btn.generate_hx_attr_string());
    }
}
