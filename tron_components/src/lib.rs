pub mod audio_player;
pub mod audio_recorder;
pub mod button;
pub mod chatbox;
pub mod checklist;
pub mod select;
pub mod text;

use std::{
    collections::{HashMap, VecDeque},
    pin::Pin,
    sync::{Arc, Weak},
};
use tokio::sync::{mpsc::Receiver, RwLockReadGuard, RwLockWriteGuard};
use tokio::sync::{oneshot, Mutex};

pub use audio_player::TnAudioPlayer;
pub use audio_recorder::TnAudioRecorder;
pub use button::TnButton;
pub use chatbox::TnChatBox;
pub use checklist::{TnCheckBox, TnCheckList};
pub use select::TnSelect;
use serde_json::Value;
pub use text::{TnStreamTextArea, TnTextArea, TnTextInput};

use rand::{thread_rng, Rng};

use axum::body::Bytes;
use bytes::BytesMut;
use serde::Deserialize;

pub type TnComponentId = u32;
pub type TnElmAttributes = HashMap<String, String>;
pub type TnExtraResponseHeader = HashMap<String, String>;
pub type TnElmTag = String;
pub type TnComponent<'a> = Arc<RwLock<Box<dyn TnComponentBaseTrait<'a>>>>;

#[derive(Debug, Clone)]
pub enum TnComponentValue {
    None,
    String(String),
    VecString(Vec<String>),
    VecString2(Vec<(String, String)>),
    CheckItems(HashMap<String, bool>),
    CheckItem(bool),
}
#[derive(Debug, Clone)]
pub enum TnAsset {
    None,
    VecU8(Vec<u8>),
    VecString(Vec<String>),
    VecString2(Vec<(String, String)>),
    HashMapString(HashMap<String, String>),
    String(String),
    Bytes(BytesMut),
    Value(Value), //json
    Bool(bool),
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum TnComponentType {
    Base,
    AudioPlayer,
    AudioRecorder,
    Button,
    CheckList,
    CheckBox,
    TextArea,
    StreamTextArea,
    TextInput,
    ChatBox,
    Select,
    UserDefined(String),
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum TnComponentState {
    Ready,    // ready to receive new event on the UI end
    Pending,  // UI event receive, server function dispatched, waiting for server to response
    Updating, // server ready to send, change UI to update to receive server response, can repeate
    Finished, // no more new update to consider, will transition to Ready
    Disabled, // disabled, not interactive and not sending htmx get/post
}

type TnComponentAsset = Option<HashMap<String, TnAsset>>;

pub struct TnComponentBase<'a: 'static> {
    pub tag: TnElmTag,
    pub type_: TnComponentType,
    pub id: TnComponentId,
    pub tron_id: String,
    pub attributes: TnElmAttributes,
    pub value: TnComponentValue,
    pub extra_response_headers: TnExtraResponseHeader,
    pub asset: TnComponentAsset,
    pub state: TnComponentState,
    pub children: Vec<TnComponent<'a>>,
    pub parent: Weak<RwLock<Box<dyn TnComponentBaseTrait<'a>>>>,
    pub script: Option<String>,
}

#[derive(Debug)]
pub struct TnServiceRequestMsg {
    pub request: String,
    pub payload: TnAsset,
    pub response: oneshot::Sender<String>,
}

#[derive(Debug)]
pub struct TnServiceResponseMsg {
    pub response: String,
    pub payload: TnAsset,
}

pub struct TnSseMsgChannel {
    pub tx: Sender<String>,
    pub rx: Option<Receiver<String>>, // this will be moved out and replaced by None
}

pub type TnComponentMap<'a> = Arc<RwLock<HashMap<u32, TnComponent<'a>>>>;
pub type TnStreamData = Arc<RwLock<HashMap<String, (String, VecDeque<BytesMut>)>>>;
pub type TnContextAsset = Arc<RwLock<HashMap<String, Vec<TnAsset>>>>;
pub type TnSeeChannels = Arc<RwLock<Option<TnSseMsgChannel>>>;
pub type TnService = (
    Sender<TnServiceRequestMsg>,
    Mutex<Option<Receiver<TnServiceResponseMsg>>>,
);
pub struct TnContextBase<'a: 'static> {
    pub components: TnComponentMap<'a>, // component ID mapped to Component structs
    pub stream_data: TnStreamData,
    pub asset: TnContextAsset,
    pub sse_channels: TnSeeChannels,
    pub tron_id_to_id: HashMap<String, u32>,
    pub services: HashMap<String, TnService>,
}

impl<'a: 'static> TnContextBase<'a> {
    pub fn new() -> Self {
        TnContextBase {
            components: Arc::new(RwLock::new(HashMap::default())),
            asset: Arc::new(RwLock::new(HashMap::default())),
            tron_id_to_id: HashMap::default(),
            stream_data: Arc::new(RwLock::new(HashMap::default())),
            services: HashMap::default(),
            sse_channels: Arc::new(RwLock::new(None)),
        }
    }

    pub fn add_component(&mut self, new_component: impl TnComponentBaseTrait<'a> + 'static) {
        let tron_id = new_component.tron_id().clone();
        let id = new_component.id();
        let mut component_guard = self.components.blocking_write();
        component_guard.insert(id, Arc::new(RwLock::new(Box::new(new_component))));
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
        let components_guard = self.components.blocking_read();
        let component = components_guard.get(&id).unwrap().blocking_read();
        component.render()
    }

    pub fn first_render_to_string(&self, tron_id: &str) -> String {
        let id = self.get_component_id(tron_id);
        let component_guard = self.components.blocking_read();
        let component = component_guard.get(&id).unwrap().blocking_read();
        component.first_render()
    }
}

#[derive(Clone)]
pub struct TnContext {
    pub base: Arc<RwLock<TnContextBase<'static>>>,
}

impl TnContext {
    pub async fn read(&self) -> RwLockReadGuard<TnContextBase<'static>> {
        self.base.read().await
    }

    pub async fn write(&self) -> RwLockWriteGuard<TnContextBase<'static>> {
        self.base.write().await
    }

    pub fn blocking_read(&self) -> RwLockReadGuard<TnContextBase<'static>> {
        self.base.blocking_read()
    }

    pub fn blocking_write(&self) -> RwLockWriteGuard<TnContextBase<'static>> {
        self.base.blocking_write()
    }

    pub async fn get_component(&self, tron_id: &str) -> TnComponent<'static> {
        let context_guard = self.write().await;
        let components_guard = context_guard.components.write().await;
        let comp_id = context_guard.get_component_id(tron_id);
        components_guard.get(&comp_id).unwrap().clone()
    }

    pub async fn get_value_from_component(&self, tron_id: &str) -> TnComponentValue {
        let value = {
            let context_guard = self.read().await;
            let components_guard = context_guard.components.read().await;
            let component_id = context_guard.get_component_id(tron_id);
            let components_guard = components_guard.get(&component_id).unwrap().read().await;
            components_guard.value().clone()
        };
        value
    }

    pub async fn set_value_for_component(&self, tron_id: &str, v: TnComponentValue) {
        let context_guard = self.read().await;
        let mut components_guard = context_guard.components.write().await;
        let component_id = context_guard.get_component_id(tron_id);
        let mut component = components_guard
            .get_mut(&component_id)
            .unwrap()
            .write()
            .await;
        component.set_value(v);
    }

    pub async fn set_state_for_component(&self, tron_id: &str, s: TnComponentState) {
        let context_guard = self.read().await;
        let mut components_guard = context_guard.components.write().await;
        let component_id = context_guard.get_component_id(tron_id);
        let mut component = components_guard
            .get_mut(&component_id)
            .unwrap()
            .write()
            .await;
        component.set_state(s);
    }

    pub async fn get_sse_tx_with_context(&self) -> Sender<String> {
        // unlock two layers of Rwlock !!
        let sse_tx = self
            .read()
            .await
            .sse_channels
            .read()
            .await
            .as_ref()
            .unwrap()
            .tx
            .clone();
        sse_tx
    }
}

impl<'a: 'static> Default for TnContextBase<'a>
where
    'a: 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

pub trait TnComponentBaseTrait<'a: 'static>: Send + Sync {
    fn id(&self) -> TnComponentId;
    fn tron_id(&self) -> &String;
    fn get_type(&self) -> TnComponentType;

    fn attributes(&self) -> &TnElmAttributes;
    fn set_attribute(&mut self, key: String, value: String);
    fn remove_attribute(&mut self, key: String);
    fn generate_attr_string(&self) -> String;

    fn extra_headers(&self) -> &TnExtraResponseHeader;
    fn set_header(&mut self, key: String, value: String);
    fn remove_header(&mut self, key: String);
    fn clear_header(&mut self);

    fn value(&self) -> &TnComponentValue;
    fn get_mut_value(&mut self) -> &mut TnComponentValue;
    fn set_value(&mut self, value: TnComponentValue);

    fn state(&self) -> &TnComponentState;
    fn set_state(&mut self, state: TnComponentState);

    fn get_assets(&self) -> Option<&HashMap<String, TnAsset>>;
    fn get_mut_assets(&mut self) -> Option<&mut HashMap<String, TnAsset>>;

    fn first_render(&self) -> String;
    fn render(&self) -> String;

    fn get_children(&self) -> &Vec<TnComponent<'a>>;
    fn add_child(&mut self, child: TnComponent<'a>);

    fn add_parent(&mut self, parent: TnComponent<'a>);
    fn get_parent(&self) -> TnComponent<'a>;

    fn get_script(&self) -> Option<String>;
}

impl<'a: 'static> TnComponentBase<'a> {
    pub fn new(tag: String, id: TnComponentId, tron_id: String, type_: TnComponentType) -> Self {
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
            type_,
            id,
            tron_id,
            attributes,
            extra_response_headers: HashMap::default(),
            value: TnComponentValue::None,
            asset: None,
            state: TnComponentState::Ready,
            ..Default::default()
        }
    }
}

impl<'a: 'static> Default for TnComponentBase<'a> {
    fn default() -> TnComponentBase<'a> {
        let mut rng = thread_rng();
        let id: u32 = rng.gen();
        let tron_id = format!("{:x}", id);
        TnComponentBase {
            tag: "div".into(),
            type_: TnComponentType::Base,
            id,
            tron_id,
            attributes: HashMap::default(),
            extra_response_headers: HashMap::default(),
            value: TnComponentValue::None,
            asset: None,
            state: TnComponentState::Ready,
            children: Vec::default(),
            parent: Weak::default(),
            script: Option::default(),
        }
    }
}

impl<'a: 'static> TnComponentBaseTrait<'a> for TnComponentBase<'a>
where
    'a: 'static,
{
    fn id(&self) -> u32 {
        self.id
    }

    fn tron_id(&self) -> &String {
        &self.tron_id
    }

    fn get_type(&self) -> TnComponentType {
        self.type_.clone()
    }

    fn attributes(&self) -> &TnElmAttributes {
        &self.attributes
    }

    fn set_attribute(&mut self, key: String, val: String) {
        self.attributes.insert(key, val);
    }

    fn remove_attribute(&mut self, key: String) {
        self.attributes.remove(&key);
    }

    fn extra_headers(&self) -> &TnExtraResponseHeader {
        &self.extra_response_headers
    }

    fn set_header(&mut self, key: String, val: String) {
        self.extra_response_headers.insert(key, val);
    }

    fn remove_header(&mut self, key: String) {
        self.extra_response_headers.remove(&key);
    }

    fn clear_header(&mut self) {
        self.extra_response_headers.clear();
    }

    fn generate_attr_string(&self) -> String {
        self.attributes
            .iter()
            .map(|(k, v)| {
                if v.is_empty() {
                    k.clone()
                } else {
                    format!(r#"{}="{}""#, k, tron_utils::html_escape_double_quote(v))
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn value(&self) -> &TnComponentValue {
        &self.value
    }

    fn get_mut_value(&mut self) -> &mut TnComponentValue {
        &mut self.value
    }

    fn set_value(&mut self, _new_value: TnComponentValue) {
        self.value = _new_value;
    }

    fn set_state(&mut self, new_state: TnComponentState) {
        self.state = new_state;
        let state = match self.state {
            TnComponentState::Ready => "ready",
            TnComponentState::Pending => "pending",
            TnComponentState::Updating => "updating",
            TnComponentState::Finished => "finished",
            TnComponentState::Disabled => "disabled",
        };
        self.attributes.insert("state".into(), state.into());
    }

    fn state(&self) -> &TnComponentState {
        &self.state
    }

    fn get_assets(&self) -> Option<&HashMap<String, TnAsset>> {
        if let Some(assets) = self.asset.as_ref() {
            Some(assets)
        } else {
            None
        }
    }

    fn get_mut_assets(&mut self) -> Option<&mut HashMap<String, TnAsset>> {
        if let Some(assets) = self.asset.as_mut() {
            Some(assets)
        } else {
            None
        }
    }

    fn get_children(&self) -> &Vec<TnComponent<'a>> {
        &self.children
    }

    fn add_child(&mut self, child: TnComponent<'a>) {
        self.children.push(child);
    }

    fn add_parent(&mut self, parent: TnComponent<'a>) {
        self.parent = Arc::downgrade(&parent);
    }

    fn get_parent(&self) -> TnComponent<'a> {
        Weak::upgrade(&self.parent).unwrap()
    }

    fn first_render(&self) -> String {
        unimplemented!()
    }

    fn render(&self) -> String {
        unimplemented!()
    }

    fn get_script(&self) -> Option<String> {
        self.script.clone()
    }
}
// For Event Dispatcher
#[derive(Eq, PartialEq, Hash, Clone, Debug, Deserialize, Default)]
pub struct TnEvent {
    pub e_target: String,
    pub e_type: String,  // maybe use Enum
    pub e_state: String, // should use the Component::State enum
}
use tokio::sync::{mpsc::Sender, RwLock};
pub type ActionFn = fn(
    TnContext,
    event: TnEvent,
    payload: Value,
) -> Pin<Box<dyn futures_util::Future<Output = ()> + Send + Sync>>;

#[derive(Clone)]
pub enum ActionExecutionMethod {
    Spawn,
    Await,
}

pub type TnEventActions = HashMap<TnEvent, (ActionExecutionMethod, Arc<ActionFn>)>;

#[cfg(test)]
mod tests {
    use crate::{TnButton, TnComponentBaseTrait};

    #[test]
    fn test_simple_button() {
        let mut btn = TnButton::new(12, "12".into(), "12".into());
        btn.set_attribute("hx-get".to_string(), format!("/tron/{}", 12));
        //println!("{}", btn.generate_hx_attr_string());
    }
}
