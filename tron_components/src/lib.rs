pub mod audio_player;
pub mod audio_recorder;
pub mod button;
pub mod chatbox;
pub mod checklist;
pub mod d3_plot;
pub mod div;
pub mod file_upload;
pub mod radio_group;
pub mod range_slider;
pub mod select;
pub mod text;

pub use audio_player::TnAudioPlayer;
pub use audio_recorder::TnAudioRecorder;
pub use button::TnButton;
pub use chatbox::TnChatBox;
pub use checklist::{TnCheckBox, TnCheckList};
pub use d3_plot::{TnD3Plot, TnD3PlotBuilder};
pub use div::TnDiv;
pub use file_upload::{TnDnDFileUpload, TnFileUpload};
pub use radio_group::{TnRadioGroup, TnRadioItem};
pub use range_slider::TnRangeSlider;
pub use select::TnSelect;
pub use text::{TnStreamTextArea, TnTextArea, TnTextInput};

pub use async_trait::async_trait;

use serde_json::Value;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    pin::Pin,
    sync::{Arc, Weak},
};
use tokio::sync::{oneshot, Mutex};
use tokio::{
    sync::{mpsc::Receiver, RwLockReadGuard, RwLockWriteGuard},
    task::JoinHandle,
};

use rand::{thread_rng, Rng};

use axum::{body::Bytes, http::HeaderMap, response::Html};
use bytes::BytesMut;
use serde::{Deserialize, Serialize};
/// This for the HTML element attributes
pub type TnElmAttributes = HashMap<String, String>;
/// For adding extra header for the HTTP responses
pub type TnExtraResponseHeader = HashMap<String, (String, bool)>; // (String, bool) = (value, remove after use?)
/// Simple alias
pub type TnElmTag = String;

/// The type representing a component
pub trait TnComponentBaseRenderTrait<'a>:
    TnComponentBaseTrait<'a> + TnComponentRenderTrait<'a>
where
    'a: 'static,
{
}

pub type TnComponent<'a> = Arc<RwLock<Box<dyn TnComponentBaseRenderTrait<'a>>>>;

/// Represents different types of values that can be associated with a component.
///
/// Variants include:
/// - `None`: No value.
/// - `String(String)`: A single string value.
/// - `VecString(Vec<String>)`: A list of strings.
/// - `VecString2(Vec<(String, String)>)`: A list of string pairs.
/// - `VecDequeString(VecDeque<String>)`: A double-ended queue of strings.
/// - `CheckItems(HashMap<String, bool>)`: A map of strings to boolean values, typically used for checkboxes.
/// - `CheckItem(bool)`: A single boolean value, typically used for a checkbox.
/// - `RadioItem(bool)`: A boolean value, typically used for a radio button.
#[derive(Debug, Clone, Default)]
pub enum TnComponentValue {
    #[default]
    None,
    String(String),
    VecString(Vec<String>),
    VecString2(Vec<(String, String)>),
    VecDequeString(VecDeque<String>),
    CheckItems(HashMap<String, bool>),
    Bool(bool),
    Value(Value),
}

/// Represents different types of assets that can be associated with a component.
///
/// Variants include:
/// - `None`: Represents no asset.
/// - `VecU8(Vec<u8>)`: A vector of bytes.
/// - `VecF32(Vec<f32>)`: A vector of 32-bit floating-point numbers.
/// - `VecF32_2(Vec<(f32, f32)>)`: A vector of 2D float tuples.
/// - `VecF32_3(Vec<(f32, f32, f32)>)`: A vector of 3D float tuples.
/// - `VecF32_4(Vec<(f32, f32, f32, f32)>)`: A vector of 4D float tuples.
/// - `VecString(Vec<String>)`: A vector of strings.
/// - `VecString2(Vec<(String, String)>)`: A vector of string pairs.
/// - `HashMapString(HashMap<String, String>)`: A hash map with string keys and values.
/// - `HashMapVecU8(HashMap<String, Vec<u8>>)`: A hash map with string keys and byte vector values.
/// - `HashSetU32(HashSet<u32>)`: A hash set of 32-bit unsigned integers.
/// - `String(String)`: A single string value.
/// - `Bytes(BytesMut)`: A mutable buffer of bytes.
/// - `U32(u32)`: A 32-bit unsigned integer.
/// - `Value(Value)`: A JSON value.
/// - `Bool(bool)`: A boolean value.
#[derive(Debug, Clone)]
pub enum TnAsset {
    None,
    VecU8(Vec<u8>),
    VecF32(Vec<f32>),
    VecF32_2(Vec<(f32, f32)>),
    VecF32_3(Vec<(f32, f32, f32)>),
    VecF32_4(Vec<(f32, f32, f32, f32)>),
    VecString(Vec<String>),
    VecString2(Vec<(String, String)>),
    HashMapString(HashMap<String, String>),
    HashMapVecU8(HashMap<String, Vec<u8>>),
    HashSetU32(HashSet<u32>),
    String(String),
    Bytes(BytesMut),
    U32(u32),
    F32(f32),
    Value(Value), //json
    Bool(bool),
}

/// Defines the types of components available in a UI toolkit.
///
/// Variants include:
/// - `Base`: The basic component type.
/// - `AudioPlayer`: A component for playing audio.
/// - `AudioRecorder`: A component for recording audio.
/// - `Button`: A button component.
/// - `CheckList`: A component representing a list of checkboxes.
/// - `CheckBox`: A single checkbox component.
/// - `TextArea`: A multiline text input component.
/// - `StreamTextArea`: A textarea for streaming text inputs.
/// - `TextInput`: A single line text input component.
/// - `ChatBox`: A component for chat functionalities.
/// - `Select`: A dropdown select component.
/// - `Slider`: A slider component.
/// - `RadioGroup`: A group of radio buttons.
/// - `RadioItem`: A single radio button.
/// - `D3Plot`: A component for D3 plotting.
/// - `FileUpload`: A component for file uploads.
/// - `DnDFileUpload`: A component for drag-and-drop file uploads.
/// - `Div`: A generic division or container component.
/// - `UserDefined(String)`: A user-defined component specified by a string identifier.
#[derive(PartialEq, Eq, Debug, Clone, Hash, Default)]
pub enum TnComponentType {
    #[default]
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
    Slider,
    RadioGroup,
    RadioItem,
    D3Plot,
    FileUpload,
    DnDFileUpload,
    Div,
    UserDefined(String),
}
use std::sync::LazyLock;
static COMPONENT_TYPE_SCRIPTS: LazyLock<Mutex<HashMap<TnComponentType, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::default()));

impl TnComponentType {
    pub fn register_script(t: TnComponentType, script: &str) {
        let mut m = COMPONENT_TYPE_SCRIPTS.blocking_lock();
        m.entry(t).or_insert_with(|| script.into());
    }

    pub fn get_script(&self) -> Option<String> {
        let m = COMPONENT_TYPE_SCRIPTS.blocking_lock();
        m.get(self).cloned()
    }
}

/// Represents the various states a UI component can be in during its lifecycle.
///
/// Variants include:
/// - `Ready`: The component is ready to receive new events from the UI.
/// - `Pending`: An event has been received from the UI, and the server function has been dispatched; awaiting server response.
/// - `Updating`: The server is ready to send a response; the UI updates to receive this response. This state can repeat.
/// - `Finished`: No further updates are expected; the component will transition back to the `Ready` state.
/// - `Disabled`: The component is disabled, making it non-interactive and unable to send or receive HTTP requests.
#[derive(Default, PartialEq, Eq, Debug, Clone)]
pub enum TnComponentState {
    #[default]
    Ready, // ready to receive new event on the UI end
    Pending,  // UI event receive, server function dispatched, waiting for server to response
    Updating, // server ready to send, change UI to update to receive server response, can repeate
    Finished, // no more new update to consider, will transition to Ready
    Disabled, // disabled, not interactive and not sending htmx get/post
}

/// Type alias for an optional hash map that maps string keys to `TnAsset` values.
///
/// This type is typically used to associate various assets with a component. Each asset is indexed by a string key.
/// The `Option` wrapper indicates that the component might not have any associated assets.
type TnComponentAsset = Option<HashMap<String, TnAsset>>;

/// Represents the base structure for a UI component.
///
/// Fields:
/// - `tag`: The HTML tag associated with the component.
/// - `type_`: The type of the component as defined by `TnComponentType`.
/// - `id`: A unique identifier for the component.
/// - `tron_id`: A unique hexadecimal identifier for the component.
/// - `attributes`: A hash map containing HTML attributes for the component.
/// - `value`: The value associated with the component, determined by `TnComponentValue`.
/// - `extra_response_headers`: Headers to add to the HTTP response when this component is involved in a request.
/// - `asset`: Optional assets associated with the component, indexed by string keys.
/// - `state`: The current state of the component as defined by `TnComponentState`.
/// - `children`: A vector of child components.
/// - `parent`: A weak reference to the parent component, encapsulated in a read-write lock.
pub struct TnComponentBase<'a: 'static> {
    pub tag: TnElmTag,
    pub type_: TnComponentType,
    pub tron_id: TnComponentId,
    pub attributes: TnElmAttributes,
    pub value: TnComponentValue,
    pub extra_response_headers: TnExtraResponseHeader,
    pub asset: TnComponentAsset,
    pub state: TnComponentState,
    pub children: Vec<TnComponent<'a>>,
    pub parent: Weak<RwLock<Box<dyn TnComponentBaseRenderTrait<'a>>>>,
    pub action: Option<(TnActionExecutionMethod, TnActionFn)>,
}

/// Represents a service request message structure used for inter-service communication.
///
/// Fields:
/// - `request`: A string specifying the request type or action to be performed.
/// - `payload`: The `TnAsset` associated with the request, representing any data needed for the request.
/// - `response`: A `oneshot::Sender<String>` used to send back a response string from the service handler.
#[derive(Debug)]
pub struct TnServiceRequestMsg {
    pub request: String,
    pub payload: TnAsset,
    pub response: oneshot::Sender<String>,
}

/// Represents a service response message structure used in inter-service communication.
///
/// Fields:
/// - `response`: A string containing the response or outcome of the service request.
/// - `payload`: The `TnAsset` accompanying the response, which can contain additional data relevant to the response.
#[derive(Debug)]
pub struct TnServiceResponseMsg {
    pub response: String,
    pub payload: TnAsset,
}

/// The `TnSseMsgChannel` struct contains a sender and an optional receiver for string messages.
///
/// Properties:
///
/// * `tx`: The `tx` property is a `Sender<String>` which is used for sending messages of type `String`
///   over a channel. This allows one part of the code to send messages to another part asynchronously.
/// * `rx`: The `rx` property is an optional field that holds a `Receiver<String>`. It is initially set
///   to `Some(receiver)` but will be moved out and replaced by `None` at some point.
pub struct TnSseMsgChannel {
    pub tx: Sender<String>,
    pub rx: Option<Receiver<String>>, // this will be moved out and replaced by None
}

/// Alias for a string that uniquely identifies a stream.
pub type TnStreamID = String;

/// Alias for a string representing the protocol used by a stream.
pub type TnStreamProtocol = String;

/// Alias for a double-ended queue of mutable byte buffers, used for streaming data.
pub type TnStreamData = VecDeque<BytesMut>;

/// Alias for a string that names an asset associated with a component or service.
pub type TnAssetName = String;

/// Alias for a string that uniquely identifies a component.
pub type TnComponentId = String;

/// Alias for a string that identifies a service within a system or application.
pub type TnServiceName = String;

/// Alias for a thread-safe, reference-counted hash map that maps component indices
/// to their corresponding `TnComponent` instances.
pub type TnComponentMap<'a> = Arc<RwLock<HashMap<TnComponentId, TnComponent<'a>>>>;

/// Alias for a thread-safe, reference-counted hash map mapping stream identifiers
/// to their protocol and data.
pub type TnStreamMap = Arc<RwLock<HashMap<TnStreamID, (TnStreamProtocol, TnStreamData)>>>;

/// Alias for a thread-safe, reference-counted hash map that maps asset names
/// to their respective `TnAsset` instances.
pub type TnContextAssets = Arc<RwLock<HashMap<TnAssetName, TnAsset>>>;

/// Alias for a thread-safe, reference-counted wrapper around an optional SSE message channel.
pub type TnSseChannel = Arc<RwLock<Option<TnSseMsgChannel>>>;

/// Alias for a tuple consisting of a sender for service requests and a mutex-guarded optional receiver for service responses.
///
/// The `Sender<TnServiceRequestMsg>` component allows sending requests to the service, while the
/// `Mutex<Option<Receiver<TnServiceResponseMsg>>>` secures exclusive access to the potentially present
/// receiver for service responses. This design ensures thread-safe communication within services, allowing
/// for effective request dispatching and conditional response handling in a multi-threaded application environment.
pub type TnService = (
    Sender<TnServiceRequestMsg>,
    Mutex<Option<Receiver<TnServiceResponseMsg>>>,
);

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct UserData {
    pub username: String,
    pub email: String,
}

/// Represents the base context structure for managing various aspects of a UI-centric application.
///
/// Fields:
/// - `components`: A `TnComponentMap` that maps component IDs to their corresponding component structures.
/// - `scripts`: A `HashMap` mapping `TnComponentType` to their associated scripts as `String`s.
/// - `stream_data`: A `TnStreamMap` that manages the streaming data associated with various components.
/// - `assets`: A `TnContextAssets`, which is a thread-safe map storing assets linked to the context.
/// - `sse_channel`: A `TnSseChannel` for managing server-sent events communication.
/// - `service_handles`: A vector of `JoinHandle`s for managing service threads.
/// - `services`: A hash map that maps service names (`TnServiceName`) to their corresponding service handlers (`TnService`).
pub struct TnContextBase<'a: 'static> {
    pub components: TnComponentMap<'a>, // component ID mapped to Component structs
    pub scripts: HashMap<TnComponentType, String>,
    pub stream_data: TnStreamMap,
    pub assets: TnContextAssets,
    pub sse_channel: TnSseChannel,
    pub service_handles: Vec<JoinHandle<()>>,
    pub services: HashMap<TnServiceName, TnService>,
    pub user_data: Arc<RwLock<Option<String>>>,
}

/// Provides the implementation for `TnContextBase`, a context management structure for handling
/// UI components and their interaction in an application.
impl TnContextBase<'static> {
    /// Constructs a new instance of `TnContextBase` with default settings.
    /// Initializes all internal storages (components, assets, streams, services, and SSE channels) as empty.
    pub fn new() -> Self {
        TnContextBase {
            components: Arc::new(RwLock::new(HashMap::default())),
            scripts: HashMap::new(),
            service_handles: Vec::new(),
            assets: Arc::new(RwLock::new(HashMap::default())),
            stream_data: Arc::new(RwLock::new(HashMap::default())),
            services: HashMap::default(),
            sse_channel: Arc::new(RwLock::new(None)),
            user_data: Arc::new(RwLock::new(None)),
        }
    }

    /// Adds a new component to the context.
    /// Registers the component with a unique `tron_id` and an `id`, and updates the mapping.
    pub fn add_component(
        &mut self,
        new_component: impl TnComponentBaseRenderTrait<'static> + 'static,
    ) {
        let tron_id = new_component.tron_id().clone();
        let component_type = new_component.get_type();

        let mut component_guard = self.components.blocking_write();
        component_guard.insert(
            tron_id.clone(),
            Arc::new(RwLock::new(Box::new(new_component))),
        );

        if let Some(script) = component_type.get_script() {
            self.scripts
                .entry(component_type.clone())
                .or_insert_with(|| script);
        }
    }

    /// Renders the specified component to a string based on its `tron_id`.
    /// This method accesses the component by index, reads its state, and calls its render function.
    pub async fn get_rendered_string(&self, tron_id: &str) -> String {
        let components_guard = self.components.read().await;
        let component = components_guard.get(tron_id).unwrap().read().await;
        component.render().await
    }

    /// Performs the initial rendering of a specified component to a string based on its `tron_id`.
    /// Similar to `render_to_string`, but specifically calls the component's first rendering logic.
    pub async fn get_initial_rendered_string(&self, tron_id: &str) -> String {
        let component_guard = self.components.read().await;
        let component = component_guard.get(tron_id).unwrap().read().await;
        component.initial_render().await
    }

    pub async fn get_user_data(&self) -> Option<UserData> {
        let user_data = self.user_data.read().await;
        user_data
            .as_ref()
            .map(|json_str| serde_json::from_str(json_str).unwrap())
    }
}

/// Represents a container for managing the overall context of an application.
///
/// This structure wraps the `TnContextBase` in an `Arc<RwLock<>>` to ensure thread-safe access
/// and modification across multiple parts of an application. The `Clone` trait implementation
/// allows `TnContext` to be easily shared among threads without duplicating the underlying data.
///
/// Fields:
/// - `base`: An `Arc<RwLock<TnContextBase<'static>>>` that holds the core context structure,
///   allowing for concurrent read/write access to the components, assets, and other elements
///   managed by `TnContextBase`.
#[derive(Clone)]
pub struct TnContext {
    pub base: Arc<RwLock<TnContextBase<'static>>>,
}

/// Provides methods for managing and interacting with `TnContextBase`, facilitating asynchronous and blocking access to its properties and functions.
impl TnContext {
    /// Asynchronously acquires a read lock on the `TnContextBase`.
    pub async fn read(&self) -> RwLockReadGuard<TnContextBase<'static>> {
        self.base.read().await
    }

    /// Asynchronously acquires a write lock on the `TnContextBase`.
    pub async fn write(&self) -> RwLockWriteGuard<TnContextBase<'static>> {
        self.base.write().await
    }

    /// Synchronously acquires a read lock on the `TnContextBase`.
    pub fn blocking_read(&self) -> RwLockReadGuard<TnContextBase<'static>> {
        self.base.blocking_read()
    }

    /// Synchronously acquires a write lock on the `TnContextBase`.
    pub fn blocking_write(&self) -> RwLockWriteGuard<TnContextBase<'static>> {
        self.base.blocking_write()
    }

    /// Asynchronously retrieves a component by its `tron_id`.
    pub async fn get_component(&self, tron_id: &str) -> TnComponent<'static> {
        let context_guard = self.read().await;
        let components_guard = context_guard.components.read().await;
        components_guard.get(tron_id).unwrap().clone()
    }

    /// Asynchronously retrieves a component by its `id`.
    pub async fn get_component_by_id(&self, id: &String) -> TnComponent<'static> {
        let context_guard = self.read().await;
        let components_guard = context_guard.components.read().await;
        components_guard.get(id).unwrap().clone()
    }

    /// Synchronously retrieves a component by its `tron_id`.
    pub fn blocking_get_component(&self, tron_id: &str) -> TnComponent<'static> {
        let context_guard = self.blocking_read();
        let components_guard = context_guard.components.blocking_read();
        components_guard.get(tron_id).unwrap().clone()
    }

    /// Asynchronously renders a component to a string by its `tron_id`.
    pub async fn render_component(&self, tron_id: &str) -> String {
        let context_guard = self.read().await;
        let components_guard = context_guard.components.read().await;
        let mut comp = components_guard.get(tron_id).unwrap().write().await;
        comp.pre_render(&context_guard).await;
        let out = comp.render().await;
        comp.post_render(&context_guard).await;
        out
    }

    /// Asynchronously retrieves the value of a component by its `tron_id`.
    pub async fn get_value_from_component(&self, tron_id: &str) -> TnComponentValue {
        let value = {
            let context_guard = self.read().await;
            let components_guard = context_guard.components.read().await;
            let components_guard = components_guard.get(tron_id).unwrap().read().await;
            components_guard.value().clone()
        };
        value
    }

    /// Asynchronously sets the value for a component by its `tron_id`.
    pub async fn set_value_for_component(&self, tron_id: &str, v: TnComponentValue) {
        let context_guard = self.read().await;
        let mut components_guard = context_guard.components.write().await;
        let mut component = components_guard.get_mut(tron_id).unwrap().write().await;
        component.set_value(v);
    }

    /// Asynchronously sets the state for a component by its `tron_id`.
    pub async fn set_state_for_component(&self, tron_id: &str, s: TnComponentState) {
        let context_guard = self.read().await;
        let mut components_guard = context_guard.components.write().await;
        let mut component = components_guard.get_mut(tron_id).unwrap().write().await;
        component.set_state(s);
    }

    /// Synchronously sets the value for a component by its `tron_id`.
    pub fn set_value_for_component_blocking(&self, tron_id: &str, v: TnComponentValue) {
        let context_guard = self.blocking_read();
        let mut components_guard = context_guard.components.blocking_write();
        let mut component = components_guard.get_mut(tron_id).unwrap().blocking_write();
        component.set_value(v);
    }

    /// Synchronously sets the state for a component by its `tron_id`
    pub fn set_state_for_component_blocking(&self, tron_id: &str, s: TnComponentState) {
        let context_guard = self.blocking_read();
        let mut components_guard = context_guard.components.blocking_write();
        let mut component = components_guard.get_mut(tron_id).unwrap().blocking_write();
        component.set_state(s);
    }

    /// Asynchronously gets the Server Side Event sender.
    pub async fn get_sse_tx(&self) -> Sender<String> {
        // unlock two layers of Rwlock !!
        let sse_tx = self
            .read()
            .await
            .sse_channel
            .read()
            .await
            .as_ref()
            .unwrap()
            .tx
            .clone();
        sse_tx
    }

    /// Asynchronously sets a component to a ready state by its `tron_id`
    pub async fn set_ready_for(&self, tron_id: &str) {
        {
            let sse_tx = self.get_sse_tx().await;
            // set the reset button back to the ready state
            self.set_state_for_component(tron_id, TnComponentState::Ready)
                .await;
            let msg = TnSseTriggerMsg {
                // update the button state
                server_event_data: TnServerEventData {
                    target: tron_id.into(),
                    new_state: "ready".into(),
                },
            };
            send_sse_msg_to_client(&sse_tx, msg).await;
        }
    }

    /// Retrieves the sender for a service's request channel by service ID.
    pub async fn get_service_tx(&self, service_id: &str) -> Sender<TnServiceRequestMsg> {
        let context_guard = self.read().await;
        context_guard.services.get(service_id).unwrap().0.clone()
    }

    /// Returns a reference to the shared asset map.
    pub async fn get_asset_ref(&self) -> Arc<RwLock<HashMap<String, TnAsset>>> {
        let context_guard = self.write().await;
        context_guard.assets.clone()
    }

    /// Aborts all running services and clears the service registry.
    pub async fn abort_all_services(&mut self) {
        let mut guard = self.write().await;
        guard.service_handles.iter().for_each(|handle| {
            handle.abort();
        });
        guard.services.clear();
    }
}

/// Implements the default trait for creating a default instance of `TnContextBase<'a>`.
impl Default for TnContextBase<'static> {
    fn default() -> Self {
        Self::new()
    }
}

/// Defines the base functionality for Tron components.
/// This trait provides methods for managing component properties,
/// rendering, handling attributes and headers, managing children and parents,
/// and setting actions.
pub trait TnComponentBaseTrait<'a: 'static>: Send + Sync {
    fn tron_id(&self) -> &TnComponentId;
    fn get_type(&self) -> TnComponentType;

    fn attributes(&self) -> &TnElmAttributes;
    fn set_attr(&mut self, key: &str, value: &str);
    fn update_attrs(&mut self, attrs: TnElmAttributes);
    fn remove_attribute(&mut self, key: &str);
    fn generate_attr_string(&self) -> String;

    fn extra_headers(&self) -> &TnExtraResponseHeader;
    fn set_header(&mut self, key: &str, value: (String, bool));
    fn remove_header(&mut self, key: &str);
    fn clear_header(&mut self);

    fn value(&self) -> &TnComponentValue;
    fn get_mut_value(&mut self) -> &mut TnComponentValue;
    fn set_value(&mut self, value: TnComponentValue);

    fn state(&self) -> &TnComponentState;
    fn set_state(&mut self, state: TnComponentState);

    fn create_assets(&mut self);
    fn get_assets(&self) -> Option<&HashMap<String, TnAsset>>;
    fn get_mut_assets(&mut self) -> Option<&mut HashMap<String, TnAsset>>;

    fn get_children(&self) -> &Vec<TnComponent<'a>>;
    fn get_mut_children(&mut self) -> &mut Vec<TnComponent<'a>>;
    fn add_child(&mut self, child: TnComponent<'a>);

    fn add_parent(&mut self, parent: TnComponent<'a>);
    fn get_parent(&self) -> TnComponent<'a>;

    fn set_action(&mut self, m: TnActionExecutionMethod, f: TnActionFn);
    fn get_action(&self) -> &Option<(TnActionExecutionMethod, TnActionFn)>;

    // fn get_script(&self) -> Option<String>;
}

/// Trait representing the base functionalities of a Tron component.
///
/// This trait defines methods for accessing and manipulating various properties of a Tron component,
/// such as its ID, type, attributes, headers, value, state, assets, children, parent, and script.
/// Implementors of this trait must provide concrete implementations for these methods.
impl TnComponentBase<'static> {
    pub fn init(&mut self, tag: String, tron_id: TnComponentId, type_: TnComponentType) {
        self.tag = tag;
        self.type_ = type_;
        self.tron_id = tron_id.clone();
        self.attributes = HtmlAttributes::builder()
            .add("id", &tron_id)
            .add("hx-post", &format!("/tron/{}", tron_id))
            .add("hx-target", &format!("#{}", tron_id))
            .add("hx-swap", "outerHTML")
            .add("hx-vals", r#"js:{event_data:get_event(event)}"#)
            .add("hx-ext", "json-enc")
            .add("state", "ready")
            .build()
            .take();
    }
}

/// Creates a default instance of `TnComponentBase`.
///
/// This function generates a new, unique component ID using a random number generator
/// and converts this ID to a hexadecimal string to use as the `tron_id`. It initializes
/// a `TnComponentBase` with the following default values:
/// - `tag`: "div"
/// - `type_`: `TnComponentType::Base`
/// - `attributes`: an empty `HashMap`
/// - `extra_response_headers`: an empty `HashMap`
/// - `value`: `TnComponentValue::None`
/// - `asset`: `None`
/// - `state`: `TnComponentState::Ready`
/// - `children`: an empty `Vec`
/// - `parent`: a `Weak` pointer set to default
/// - `script`: an `Option` set to `None`
///
/// # Examples
/// ```
/// let component = TnComponentBase::default();
/// assert_eq!(component.tag, "div");
/// assert!(component.tron_id.len() > 0);
/// assert_eq!(component.type_, TnComponentType::Base);
/// assert_eq!(component.state, TnComponentState::Ready);
/// ```
///
/// This is useful for initializing components with a unique ID and default settings.
impl Default for TnComponentBase<'static> {
    fn default() -> TnComponentBase<'static> {
        let mut rng = thread_rng();
        let id: u32 = rng.gen();
        let tron_id = format!("{:x}", id);
        TnComponentBase {
            tag: "div".into(),
            type_: TnComponentType::Base,
            tron_id,
            attributes: HashMap::default(),
            extra_response_headers: HashMap::default(),
            value: TnComponentValue::None,
            asset: None,
            state: TnComponentState::Ready,
            children: Vec::default(),
            parent: Weak::default(),
            action: None, //script: Option::default(),
        }
    }
}

/// Implementation of the TnComponentBaseTrait for TnComponentBase
///
/// This implementation provides methods for managing various aspects of a Tron component,
/// including its identifier, attributes, state, value, children, and actions. It also
/// includes placeholder methods for rendering and lifecycle operations.
///
/// Key features:
/// - Getters and setters for component properties (id, type, attributes, state, etc.)
/// - Methods for managing extra response headers
/// - Attribute string generation
/// - Child component management
/// - Parent component reference
/// - Asset management
/// - Action setting and retrieval
/// - Placeholder methods for rendering and lifecycle operations (first_render, pre_render, render, post_render)
impl TnComponentBaseTrait<'static> for TnComponentBase<'static> {
    /// Returns a reference to the component's Tron identifier
    fn tron_id(&self) -> &TnComponentId {
        &self.tron_id
    }

    /// Returns the component's type
    fn get_type(&self) -> TnComponentType {
        self.type_.clone()
    }

    /// Returns a reference to the component's attributes
    fn attributes(&self) -> &TnElmAttributes {
        &self.attributes
    }

    /// Sets an attribute for the component
    fn set_attr(&mut self, key: &str, val: &str) {
        self.attributes.insert(key.into(), val.into());
    }

    /// Sets an attributes for the component
    fn update_attrs(&mut self, attrs: TnElmAttributes) {
        for (k, v) in attrs {
            self.attributes.insert(k, v);
        }
    }

    /// Removes an attribute from the component
    fn remove_attribute(&mut self, key: &str) {
        self.attributes.remove(key);
    }

    /// Returns a reference to the component's extra response headers
    fn extra_headers(&self) -> &TnExtraResponseHeader {
        &self.extra_response_headers
    }

    /// Sets an extra response header for the component
    fn set_header(&mut self, key: &str, val: (String, bool)) {
        self.extra_response_headers.insert(key.into(), val);
    }

    /// Removes an extra response header from the component
    fn remove_header(&mut self, key: &str) {
        self.extra_response_headers.remove(key);
    }

    /// Clears all extra response headers from the component
    fn clear_header(&mut self) {
        self.extra_response_headers.clear();
    }

    /// Generates a string representation of the component's attributes
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

    /// Returns a reference to the component's value
    fn value(&self) -> &TnComponentValue {
        &self.value
    }

    /// Returns a mutable reference to the component's value
    fn get_mut_value(&mut self) -> &mut TnComponentValue {
        &mut self.value
    }

    /// Sets the component's value
    fn set_value(&mut self, _new_value: TnComponentValue) {
        self.value = _new_value;
    }

    /// Sets the component's state and updates the 'state' attribute
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

    /// Returns a reference to the component's state
    fn state(&self) -> &TnComponentState {
        &self.state
    }

    /// Creates the assets HashMap for the component
    fn create_assets(&mut self) {
        self.asset = Some(HashMap::default());
    }

    /// Returns a reference to the component's assets, if they exist
    fn get_assets(&self) -> Option<&HashMap<String, TnAsset>> {
        self.asset.as_ref()
    }

    /// Returns a mutable reference to the component's assets, if they exist
    fn get_mut_assets(&mut self) -> Option<&mut HashMap<String, TnAsset>> {
        self.asset.as_mut()
    }

    /// Returns a reference to the component's children
    fn get_children(&self) -> &Vec<TnComponent<'static>> {
        &self.children
    }

    /// Returns a mutable reference to the component's children
    fn get_mut_children(&mut self) -> &mut Vec<TnComponent<'static>> {
        &mut self.children
    }

    /// Adds a child component to this component
    fn add_child(&mut self, child: TnComponent<'static>) {
        self.children.push(child);
    }

    /// Sets the parent component for this component
    fn add_parent(&mut self, parent: TnComponent<'static>) {
        self.parent = Arc::downgrade(&parent);
    }

    /// Returns the parent component of this component
    fn get_parent(&self) -> TnComponent<'static> {
        Weak::upgrade(&self.parent).unwrap()
    }

    /// Sets the action for the component
    fn set_action(&mut self, m: TnActionExecutionMethod, f: TnActionFn) {
        self.action = Some((m, f));
    }

    /// Returns a reference to the component's action, if it exists
    fn get_action(&self) -> &Option<(TnActionExecutionMethod, TnActionFn)> {
        &self.action
    }
}

pub struct TnComponentBaseBuilder<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl<'a: 'static> TnComponentBase<'a> {
    pub fn builder(base: TnComponentBase) -> TnComponentBaseBuilder<'a> {
        TnComponentBaseBuilder { base }
    }
}

impl<'a: 'static> TnComponentBaseBuilder<'a> {
    pub fn init(
        mut self,
        tag: String,
        tron_id: TnComponentId,
        type_: TnComponentType,
    ) -> TnComponentBaseBuilder<'a> {
        self.base.init(tag, tron_id, type_);
        self
    }

    /// Sets the component's value
    pub fn set_value(mut self, new_value: TnComponentValue) -> TnComponentBaseBuilder<'a> {
        self.base.value = new_value;
        self
    }

    /// Sets an attribute for the component
    pub fn set_attr(mut self, key: &str, val: &str) -> TnComponentBaseBuilder<'a> {
        self.base.attributes.insert(key.into(), val.into());
        self
    }

    /// Sets the action for the component
    pub fn set_action(
        mut self,
        m: TnActionExecutionMethod,
        f: TnActionFn,
    ) -> TnComponentBaseBuilder<'a> {
        self.base.action = Some((m, f));
        self
    }

    /// Creates the assets HashMap for the component
    pub fn create_assets(mut self) -> TnComponentBaseBuilder<'a> {
        self.base.asset = Some(HashMap::default());
        self
    }

    pub fn build(self) -> TnComponentBase<'a> {
        self.base
    }
}

#[async_trait]
pub trait TnComponentRenderTrait<'a>: Send + Sync {
    async fn initial_render(&self) -> String;
    // we pass &TnContextBase not the &TnContext to ensure read-only access in pre_render
    async fn pre_render(&mut self, ctx_base: &TnContextBase);
    async fn render(&self) -> String;
    // we pass &TnContextBase not the &TnContext to ensure read-only access in post_render
    async fn post_render(&mut self, ctx_base: &TnContextBase);
}

/// Represents a Tron event.
///
/// This struct encapsulates various properties of a Tron event, including its trigger, type, state, target, and header target.
///
#[derive(Eq, PartialEq, Hash, Clone, Debug, Deserialize, Default)]
pub struct TnEvent {
    pub e_trigger: String,
    pub e_type: String,  // maybe use Enum in the future
    pub e_state: String, // should use the Component::State enum
    #[allow(dead_code)]
    #[serde(default)]
    pub e_target: String,
    #[allow(dead_code)]
    #[serde(default)]
    pub h_target: Option<String>,
}

use tokio::sync::{mpsc::Sender, RwLock};
use tron_utils::{send_sse_msg_to_client, HtmlAttributes, TnServerEventData, TnSseTriggerMsg};

/// Represents an HTML response along with its headers, wrapped in an optional tuple.
pub type TnHtmlResponse = Option<(HeaderMap, Html<String>)>;

/// Represents an asynchronous action function that processes Tron events.
///
/// This type defines a function signature for processing Tron events asynchronously. It takes a
/// `TnContext`, a `TnEvent`, and a payload of type `Value` as input parameters and returns a future
/// wrapping the HTML response along with its headers, represented by `TnHtmlResponse`.
pub type TnActionFn = fn(TnContext, event: TnEvent, payload: Value) -> TnFutureHTMLResponse;

/// Represents the method of execution for actions associated with Tron events.
#[derive(Clone)]
pub enum TnActionExecutionMethod {
    /// Spawn the action asynchronously.
    Spawn,
    /// Await the action's completion.
    Await,
}

#[macro_export]
macro_rules! tn_boxed_future_type {
    ($name:ident, $output_type:ty) => {
        pub type $name = std::pin::Pin<Box<dyn std::future::Future<Output = $output_type> + Send>>;
    };
}
tn_boxed_future_type!(TnFutureString, String);
tn_boxed_future_type!(TnFutureUnit, ());
tn_boxed_future_type!(TnFutureHTMLResponse, TnHtmlResponse);

#[macro_export]
macro_rules! tn_future {
    ( $($code:tt)* ) => {
        Box::pin( async move { $($code)* })
    };
}

#[cfg(test)]
mod tests {
    use crate::{button::TnButton, TnComponentBaseTrait};

    #[test]
    fn test_simple_button() {
        let btn = TnButton::builder()
            .init("12".into(), "12".into())
            .set_attr("hx-get", &format!("/tron/{}", 12))
            .build();
        println!("{}", btn.generate_attr_string());
    }
}
