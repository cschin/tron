#![feature(type_alias_impl_trait)]

pub mod button;
pub mod text;

use std::{
    collections::{self, HashMap},
    future::Future,
    pin::Pin,
    process::Output,
    sync::Arc,
    task::{Context, Poll},
};

pub use button::TnButton;
use rand::{thread_rng, Rng};

use axum::{body::Bytes, response::Html};

pub type ComponentId = u32;
pub type ElmAttributes = HashMap<String, String>;
pub type ElmTag = String;

#[derive(Debug)]
pub enum ComponentValue {
    None,
    String(String),
}
#[derive(Debug)]
pub enum ComponentAsset {
    None,
    VecU8(Vec<u8>),
    String(String),
    Bytes(Bytes),
}

#[derive(Debug)]
pub enum ComponentState {
    Ready,   // ready to receive new event on the UI end
    Pending, // UI event receive, server function dispatched, waiting for server to response
    Update,  // server ready to send, change UI to update to receive server response, can repeate
    Finish,  // no more new update to consider, will transition to Ready
    Disable, // disabled, not interactive and not sending htmx get/post
}
#[derive(Debug)]
pub struct ComponentBase<'a> {
    pub tag: ElmTag,
    pub id: ComponentId,
    pub tron_id: String,
    pub attributes: ElmAttributes,
    pub value: ComponentValue,
    pub assets: Option<HashMap<String, ComponentAsset>>,
    pub state: ComponentState,
    pub children: Option<Vec<&'a ComponentBase<'a>>>, // general storage
}

pub enum ComponentTypes<'a> {
    TnButton(TnButton<'a>),
}
pub struct ApplicationStates<'a> {
    pub components: HashMap<u32, Box<dyn ComponentBaseTrait<'a>>>, // component ID mapped to Component structs
    pub assets: HashMap<String, Vec<u8>>,
    tron_id_to_id: HashMap<String, u32>,
}

impl<'a> ApplicationStates<'a> {
    pub fn new() -> Self {
        ApplicationStates {
            components: HashMap::default(),
            assets: HashMap::default(),
            tron_id_to_id: HashMap::default(),
        }
    }

    pub fn add_component(&mut self, new_component: impl ComponentBaseTrait<'a> + 'static) {
        let tron_id = new_component.tron_id().clone();
        let id = new_component.id();
        self.components.insert(id, Box::new(new_component));
        self.tron_id_to_id.insert(tron_id, id);
    }

    pub fn get_component_by_tron_id(
        &self,
        tron_id: &str,
    ) -> &Box<dyn ComponentBaseTrait<'a> + 'static> {
        let id = self.tron_id_to_id.get(tron_id).unwrap();
        self.components.get(id).unwrap()
    }
}

impl<'a> Default for ApplicationStates<'a> {
    fn default() -> Self {
        Self::new()
    }
}

pub trait ComponentBaseTrait<'a>: Send + Sync {
    fn id(&self) -> ComponentId;
    fn tron_id(&self) -> &String;
    fn attributes(&self) -> &ElmAttributes;
    fn set_attribute(&mut self, key: String, value: String);
    fn generate_attr_string(&self) -> String;
    fn value(&self) -> &ComponentValue;
    fn set_value(&mut self, value: ComponentValue);
    fn state(&self) -> &ComponentState;
    fn set_state(&mut self, state: ComponentState);
    fn assets(&self) -> &Option<HashMap<String, ComponentAsset>>;
    fn render(&self) -> Html<String>;
    fn get_children(&self) -> &Option<Vec<&'a ComponentBase<'a>>>;
}

impl<'a> ComponentBase<'a> {
    pub fn new(tag: ElmTag, id: ComponentId, tron_id: String) -> Self {
        Self {
            tag,
            id,
            tron_id,
            attributes: HashMap::default(),
            value: ComponentValue::None,
            assets: None,
            state: ComponentState::Ready,
            children: None,
        }
    }
}

impl<'a> Default for ComponentBase<'a> {
    fn default() -> ComponentBase<'a> {
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
            state: ComponentState::Ready,
            children: None,
        }
    }
}

impl<'a> ComponentBaseTrait<'a> for ComponentBase<'a> {
    fn id(&self) -> u32 {
        self.id
    }

    fn tron_id(&self) -> &String {
        &self.tron_id
    }

    fn attributes(&self) -> &ElmAttributes {
        &self.attributes
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

    fn set_value(&mut self, _new_value: ComponentValue) {
        self.value = _new_value;
    }

    fn set_state(&mut self, new_state: ComponentState) {
        self.state = new_state;
    }

    fn state(&self) -> &ComponentState {
        &self.state
    }

    fn assets(&self) -> &Option<HashMap<String, ComponentAsset>> {
        &self.assets
    }

    fn render(&self) -> Html<String> {
        unimplemented!()
    }

    fn get_children(&self) -> &Option<Vec<&'a ComponentBase<'a>>> {
        &self.children
    }
}

// Event Dispatcher
#[derive(Eq, PartialEq, Hash)]
pub struct TnEvent {
    pub evt_target: String,
    pub evt_type: String, // maybe use Enum
}
use tokio::sync::{mpsc::Sender, RwLock};
use tower_sessions::{session_store, Expiry, MemoryStore, Session, SessionManagerLayer};
pub type ActionFn = fn(
    Arc<RwLock<ApplicationStates<'static>>>,
    Sender<axum::Json<serde_json::Value>>,
) -> Pin<Box<dyn futures_util::Future<Output = ()> + Send + Sync>>;
pub type TnEventFunction = Arc<ActionFn>;

pub type TnEventMap = HashMap<TnEvent, TnEventFunction>;

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
