pub mod audio_player;
pub mod audio_recorder;
pub mod button;
pub mod text;

use std::{
    collections::{HashMap, VecDeque},
    pin::Pin,
    sync::Arc,
};
use tokio::sync::mpsc::Receiver;
use tokio::sync::{oneshot, Mutex};

pub use audio_player::TnAudioPlayer;
pub use audio_recorder::TnAudioRecorder;
pub use button::TnButton;
use serde_json::Value;
pub use text::TnTextArea;

use rand::{thread_rng, Rng};

use axum::{body::Bytes, response::Html};
use bytes::BytesMut;

pub type ComponentId = u32;
pub type ElmAttributes = HashMap<String, String>;
pub type ElmTag = String;

use serde::Deserialize;

#[derive(Debug, Clone)]
pub enum ComponentValue {
    None,
    String(String),
}
#[derive(Debug, Clone)]
pub enum TnAsset {
    None,
    VecU8(Vec<u8>),
    String(String),
    Bytes(BytesMut),
    Value(Value), //json
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum ComponentState {
    Ready,    // ready to receive new event on the UI end
    Pending,  // UI event receive, server function dispatched, waiting for server to response
    Updating, // server ready to send, change UI to update to receive server response, can repeate
    Finished, // no more new update to consider, will transition to Ready
    Disabled, // disabled, not interactive and not sending htmx get/post
}
#[derive(Debug)]
pub struct ComponentBase<'a> {
    pub tag: ElmTag,
    pub id: ComponentId,
    pub tron_id: String,
    pub attributes: ElmAttributes,
    pub value: ComponentValue,
    pub assets: Option<HashMap<String, TnAsset>>,
    pub state: ComponentState,
    pub children: Option<Vec<&'a ComponentBase<'a>>>, // general storage
}

#[derive(Debug)]
pub struct ServiceRequestMessage {
    pub request: String,
    pub payload: TnAsset,
    pub response: oneshot::Sender<String>,
}

#[derive(Debug)]
pub struct ServiceResponseMessage {
    pub response: String,
    pub payload: TnAsset,
}

pub struct SseMessageChannel {
    pub tx: Sender<String>,
    pub rx: Option<Receiver<String>>, // this will be moved out and replaced by None
}

pub struct Context<'a> {
    pub components: Arc<RwLock<HashMap<u32, Box<dyn ComponentBaseTrait<'a>>>>>, // component ID mapped to Component structs
    pub stream_data: Arc<RwLock<HashMap<String, (String, VecDeque<BytesMut>)>>>,
    pub assets: Arc<RwLock<HashMap<String, Vec<TnAsset>>>>,
    pub sse_channels: Arc<RwLock<Option<SseMessageChannel>>>,
    pub tron_id_to_id: HashMap<String, u32>,
    pub services: HashMap<
        String,
        (
            Sender<ServiceRequestMessage>,
            Mutex<Option<Receiver<ServiceResponseMessage>>>,
        ),
    >,
}

impl<'a> Context<'a> {
    pub fn new() -> Self {
        Context {
            components: Arc::new(RwLock::new(HashMap::default())),
            assets: Arc::new(RwLock::new(HashMap::default())),
            tron_id_to_id: HashMap::default(),
            stream_data: Arc::new(RwLock::new(HashMap::default())),
            services: HashMap::default(),
            sse_channels: Arc::new(RwLock::new(None)),
        }
    }

    pub fn add_component(&mut self, new_component: impl ComponentBaseTrait<'a> + 'static) {
        let tron_id = new_component.tron_id().clone();
        let id = new_component.id();
        let mut component_guard = self.components.blocking_write();
        component_guard.insert(id, Box::new(new_component));
        self.tron_id_to_id.insert(tron_id, id);
    }

    pub fn get_component_id(&self, tron_id: &str) -> u32 {
        *self
            .tron_id_to_id
            .get(tron_id)
            .unwrap_or_else(|| panic!("component tron_id:{} not found", tron_id))
    }

    pub fn render_to_string(&self, tron_id: &str) -> String {
        let id = self.get_component_id(tron_id);
        let component_guard = self.components.blocking_read();
        let component = component_guard.get(&id).unwrap();
        component.render().0
    }
}

impl<'a> Default for Context<'a> {
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
    fn get_assets(&self) -> Option<&HashMap<String, TnAsset>>;
    fn get_mut_assets(&mut self) -> Option<&mut HashMap<String, TnAsset>>;
    fn render(&self) -> Html<String>;
    fn render_to_string(&self) -> String;
    fn get_children(&self) -> Option<&Vec<&'a ComponentBase<'a>>>;
}

impl<'a> ComponentBase<'a> {
    pub fn new(tag: String, id: ComponentId, tron_id: String) -> Self {
        let mut attributes = HashMap::<String, String>::default();
        attributes.insert("id".into(), tron_id.clone());
        attributes.insert("hx-post".to_string(), format!("/tron/{}", id));
        attributes.insert("hx-target".to_string(), format!("#{}", tron_id));
        attributes.insert("hx-swap".to_string(), "outerHTML".into());

        attributes.insert(
            "hx-vals".into(),
            r##"js:{event_data:get_event(event)}"##.into(),
        );
        attributes.insert("hx-ext".into(), "json-enc".into());
        attributes.insert("state".to_string(), "ready".to_string());
        Self {
            tag,
            id,
            tron_id,
            attributes,
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
            .map(|(k, v)| format!(r#"{}="{}""#, k, utils::html_escape_double_quote(v)))
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
        let state = match self.state {
            ComponentState::Ready => "ready",
            ComponentState::Pending => "pending",
            ComponentState::Updating => "updating",
            ComponentState::Finished => "finished",
            ComponentState::Disabled => "disabled",
        };
        self.attributes.insert("state".into(), state.into());
    }

    fn state(&self) -> &ComponentState {
        &self.state
    }

    fn get_assets(&self) -> Option<&HashMap<String, TnAsset>> {
        if let Some(assets) = self.assets.as_ref() {
            Some(assets)
        } else {
            None
        }
    }

    fn get_mut_assets(&mut self) -> Option<&mut HashMap<String, TnAsset>> {
        if let Some(assets) = self.assets.as_mut() {
            Some(assets)
        } else {
            None
        }
    }

    fn render(&self) -> Html<String> {
        unimplemented!()
    }

    fn render_to_string(&self) -> String {
        self.render().0
    }

    fn get_children(&self) -> Option<&Vec<&'a ComponentBase<'a>>> {
        if let Some(children) = self.children.as_ref() {
            Some(children)
        } else {
            None
        }
    }
}
// For Event Dispatcher
#[derive(Eq, PartialEq, Hash, Clone, Debug, Deserialize)]
pub struct TnEvent {
    pub e_target: String,
    pub e_type: String,  // maybe use Enum
    pub e_state: String, // should use the Component::State enum
}
use tokio::sync::{mpsc::Sender, RwLock};
pub type ActionFn = fn(
    Arc<RwLock<Context<'static>>>,
    event: TnEvent,
    payload: Value,
) -> Pin<Box<dyn futures_util::Future<Output = ()> + Send + Sync>>;

#[derive(Clone)]
pub enum ActionExecutionMethod {
    Spawn,
    Await,
}

pub type TnEventActions = HashMap<TnEvent, (ActionExecutionMethod, Arc<ActionFn>)>;

pub mod utils {
    pub fn html_escape_double_quote(input: &str) -> String {
        let mut output = String::new();
        for c in input.chars() {
            if c == '"' {
                output.push_str("&quot;");
            } else {
                output.push(c)
            }
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use crate::{ComponentBaseTrait, TnButton};

    #[test]
    fn test_simple_button() {
        let mut btn = TnButton::new(12, "12".into(), "12".into());
        btn.set_attribute("hx-get".to_string(), format!("/tron/{}", 12));
        //println!("{}", btn.generate_hx_attr_string());
    }
}
