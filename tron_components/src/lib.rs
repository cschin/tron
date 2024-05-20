pub mod audio_player;
pub mod audio_recorder;
pub mod button;
pub mod chatbox;
pub mod checklist;
pub mod radio_group;
pub mod range_slider;
pub mod select;
pub mod text;
pub mod d3_plot;

pub use audio_player::TnAudioPlayer;
pub use audio_recorder::TnAudioRecorder;
pub use button::TnButton;
pub use chatbox::TnChatBox;
pub use checklist::{TnCheckBox, TnCheckList};
pub use radio_group::{TnRadioGroup, TnRadioItem};
pub use range_slider::TnRangeSlider;
pub use select::TnSelect;
pub use d3_plot::TnD3Plot;
use serde_json::Value;
pub use text::{TnStreamTextArea, TnTextArea, TnTextInput};

use std::{
    collections::{HashMap, VecDeque},
    pin::Pin,
    sync::{Arc, Weak},
};
use tokio::sync::{mpsc::Receiver, RwLockReadGuard, RwLockWriteGuard};
use tokio::sync::{oneshot, Mutex};

use rand::{thread_rng, Rng};

use axum::{body::Bytes, http::HeaderMap, response::Html};
use bytes::BytesMut;
use serde::Deserialize;

/// Each component is index by a u32 number
pub type TnComponentIndex = u32;
/// This for the HTML element attributes
pub type TnElmAttributes = HashMap<String, String>;
/// For adding extra header for the HTTP responses
pub type TnExtraResponseHeader = HashMap<String, (String, bool)>; // (String, bool) = (value, remove after use?)
/// Simple alias
pub type TnElmTag = String;
/// The type representing a component
pub type TnComponent<'a> = Arc<RwLock<Box<dyn TnComponentBaseTrait<'a>>>>;

/// Represents different types of values that can be associated with a component.
///
/// Variants include:
/// - `None`: No value.
/// - `String(String)`: A single string value.
/// - `VecString(Vec<String>)`: A list of strings.
/// - `VecString2(Vec<(String, String)>)`: A list of string pairs.
/// - `CheckItems(HashMap<String, bool>)`: A map of strings to boolean values, typically used for checkboxes.
/// - `CheckItem(bool)`: A single boolean value, typically used for a checkbox.
/// - `RadioItem(bool)`: A boolean value, typically used for a radio button.
#[derive(Debug, Clone)]
pub enum TnComponentValue {
    None,
    String(String),
    VecString(Vec<String>),
    VecString2(Vec<(String, String)>),
    CheckItems(HashMap<String, bool>),
    CheckItem(bool),
    RadioItem(bool),
}

/// Represents different types of assets that can be associated with a component.
///
/// Variants include:
/// - `None`: Represents no asset.
/// - `VecU8(Vec<u8>)`: A vector of bytes.
/// - `VecString(Vec<String>)`: A vector of strings.
/// - `VecString2(Vec<(String, String)>)`: A vector of string pairs.
/// - `HashMapString(HashMap<String, String>)`: A hash map with string keys and values.
/// - `String(String)`: A single string value.
/// - `Bytes(BytesMut)`: A mutable buffer of bytes.
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
    String(String),
    Bytes(BytesMut),
    U32(u32),
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
/// - `UserDefined(String)`: A user-defined component specified by a string identifier.
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
    Slider,
    RadioGroup,
    RadioItem,
    SimpleScatterPlot,
    D3Plot,
    UserDefined(String),
}

/// Represents the various states a UI component can be in during its lifecycle.
///
/// Variants include:
/// - `Ready`: The component is ready to receive new events from the UI.
/// - `Pending`: An event has been received from the UI, and the server function has been dispatched; awaiting server response.
/// - `Updating`: The server is ready to send a response; the UI updates to receive this response. This state can repeat.
/// - `Finished`: No further updates are expected; the component will transition back to the `Ready` state.
/// - `Disabled`: The component is disabled, making it non-interactive and unable to send or receive HTTP requests.
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum TnComponentState {
    Ready,    // ready to receive new event on the UI end
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
/// - `script`: Optional custom script associated with the component.

pub struct TnComponentBase<'a: 'static> {
    pub tag: TnElmTag,
    pub type_: TnComponentType,
    pub id: TnComponentIndex,
    pub tron_id: TnComponentId,
    pub attributes: TnElmAttributes,
    pub value: TnComponentValue,
    pub extra_response_headers: TnExtraResponseHeader,
    pub asset: TnComponentAsset,
    pub state: TnComponentState,
    pub children: Vec<TnComponent<'a>>,
    pub parent: Weak<RwLock<Box<dyn TnComponentBaseTrait<'a>>>>,
    pub script: Option<String>,
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
/// over a channel. This allows one part of the code to send messages to another part asynchronously.
/// * `rx`: The `rx` property is an optional field that holds a `Receiver<String>`. It is initially set
/// to `Some(receiver)` but will be moved out and replaced by `None` at some point.

pub struct TnSseMsgChannel {
    pub tx: Sender<String>,
    pub rx: Option<Receiver<String>>, // this will be moved out and replaced by None
}

/// Alias for a string that uniquely identifies a stream.
type TnStreamID = String;

/// Alias for a string representing the protocol used by a stream.
type TnStreamProtocol = String;

/// Alias for a double-ended queue of mutable byte buffers, used for streaming data.
type TnStreamData = VecDeque<BytesMut>;

/// Alias for a string that names an asset associated with a component or service.
type TnAssetName = String;

/// Alias for a string that uniquely identifies a component.
type TnComponentId = String;

/// Alias for a string that identifies a service within a system or application.
type TnServiceName = String;

/// Alias for a thread-safe, reference-counted hash map that maps component indices
/// to their corresponding `TnComponent` instances.
pub type TnComponentMap<'a> = Arc<RwLock<HashMap<TnComponentIndex, TnComponent<'a>>>>;

/// Alias for a thread-safe, reference-counted hash map mapping stream identifiers
/// to their protocol and data.
pub type TnStreamMap = Arc<RwLock<HashMap<TnStreamID, (TnStreamProtocol, TnStreamData)>>>;

/// Alias for a thread-safe, reference-counted hash map that maps asset names
/// to their respective `TnAsset` instances.
pub type TnContextAsset = Arc<RwLock<HashMap<TnAssetName, TnAsset>>>;

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

/// Represents the base context structure for managing various aspects of a UI-centric application.
///
/// Fields:
/// - `components`: A `TnComponentMap` that maps component IDs to their corresponding component structures.
/// - `stream_data`: A `TnStreamMap` that manages the streaming data associated with various components.
/// - `asset`: A `TnContextAsset`, which is a thread-safe map storing assets linked to the context.
/// - `sse_channel`: A `TnSseChannel` for managing server-sent events communication.
/// - `tnid_to_index`: A hash map mapping component IDs (`TnComponentId`) to their indices (`TnComponentIndex`).
/// - `services`: A hash map that maps service names (`TnServiceName`) to their corresponding service handlers (`TnService`).
pub struct TnContextBase<'a: 'static> {
    pub components: TnComponentMap<'a>, // component ID mapped to Component structs
    pub stream_data: TnStreamMap,
    pub asset: TnContextAsset,
    pub sse_channel: TnSseChannel,
    pub tnid_to_index: HashMap<TnComponentId, TnComponentIndex>,
    pub services: HashMap<TnServiceName, TnService>,
}

/// Provides the implementation for `TnContextBase`, a context management structure for handling
/// UI components and their interaction in an application.
impl<'a: 'static> TnContextBase<'a> {
    /// Constructs a new instance of `TnContextBase` with default settings.
    /// Initializes all internal storages (components, assets, streams, services, and SSE channels) as empty.

    pub fn new() -> Self {
        TnContextBase {
            components: Arc::new(RwLock::new(HashMap::default())),
            asset: Arc::new(RwLock::new(HashMap::default())),
            tnid_to_index: HashMap::default(),
            stream_data: Arc::new(RwLock::new(HashMap::default())),
            services: HashMap::default(),
            sse_channel: Arc::new(RwLock::new(None)),
        }
    }

    /// Adds a new component to the context.
    /// Registers the component with a unique `tron_id` and an `id`, and updates the mapping.

    pub fn add_component(&mut self, new_component: impl TnComponentBaseTrait<'a> + 'static) {
        let tron_id = new_component.tron_id().clone();
        let id = new_component.id();
        let mut component_guard = self.components.blocking_write();
        component_guard.insert(id, Arc::new(RwLock::new(Box::new(new_component))));
        self.tnid_to_index.insert(tron_id, id);
    }

    /// Retrieves the component index using its `tron_id`.
    /// Panics if the `tron_id` is not found, ensuring that calling code handles missing components.
    pub fn get_component_index(&self, tron_id: &str) -> u32 {
        *self
            .tnid_to_index
            .get(tron_id)
            .unwrap_or_else(|| panic!("component tron_id:{} not found", tron_id))
    }

    /// Renders the specified component to a string based on its `tron_id`.
    /// This method accesses the component by index, reads its state, and calls its render function.
    pub fn render_to_string(&self, tron_id: &str) -> String {
        let id = self.get_component_index(tron_id);
        let components_guard = self.components.blocking_read();
        let component = components_guard.get(&id).unwrap().blocking_read();
        component.render()
    }

    /// Performs the initial rendering of a specified component to a string based on its `tron_id`.
    /// Similar to `render_to_string`, but specifically calls the component's first rendering logic.
    pub fn first_render_to_string(&self, tron_id: &str) -> String {
        let id = self.get_component_index(tron_id);
        let component_guard = self.components.blocking_read();
        let component = component_guard.get(&id).unwrap().blocking_read();
        component.first_render()
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
        let context_guard = self.write().await;
        let components_guard = context_guard.components.write().await;
        let comp_id = context_guard.get_component_index(tron_id);
        components_guard.get(&comp_id).unwrap().clone()
    }

    /// Synchronously retrieves a component by its `tron_id`.
    pub fn blocking_get_component(&self, tron_id: &str) -> TnComponent<'static> {
        let context_guard = self.blocking_write();
        let components_guard = context_guard.components.blocking_write();
        let comp_id = context_guard.get_component_index(tron_id);
        components_guard.get(&comp_id).unwrap().clone()
    }

    /// Asynchronously renders a component to a string by its `tron_id`.
    pub async fn render_component(&self, tron_id: &str) -> String {
        let context_guard = self.read().await;
        let components_guard = context_guard.components.read().await;
        let comp_id = context_guard.get_component_index(tron_id);
        let comp = components_guard.get(&comp_id).unwrap().read().await;
        comp.render()
    }

     /// Asynchronously retrieves the value of a component by its `tron_id`.
    pub async fn get_value_from_component(&self, tron_id: &str) -> TnComponentValue {
        let value = {
            let context_guard = self.read().await;
            let components_guard = context_guard.components.read().await;
            let component_id = context_guard.get_component_index(tron_id);
            let components_guard = components_guard.get(&component_id).unwrap().read().await;
            components_guard.value().clone()
        };
        value
    }

    /// Asynchronously sets the value for a component by its `tron_id`.
    pub async fn set_value_for_component(&self, tron_id: &str, v: TnComponentValue) {
        let context_guard = self.read().await;
        let mut components_guard = context_guard.components.write().await;
        let component_id = context_guard.get_component_index(tron_id);
        let mut component = components_guard
            .get_mut(&component_id)
            .unwrap()
            .write()
            .await;
        component.set_value(v);
    }

    /// Asynchronously sets the state for a component by its `tron_id`.
    pub async fn set_state_for_component(&self, tron_id: &str, s: TnComponentState) {
        let context_guard = self.read().await;
        let mut components_guard = context_guard.components.write().await;
        let component_id = context_guard.get_component_index(tron_id);
        let mut component = components_guard
            .get_mut(&component_id)
            .unwrap()
            .write()
            .await;
        component.set_state(s);
    }

    /// Synchronously sets the value for a component by its `tron_id`. 
    pub fn set_value_for_component_blocking(&self, tron_id: &str, v: TnComponentValue) {
        let context_guard = self.blocking_read();
        let mut components_guard = context_guard.components.blocking_write();
        let component_id = context_guard.get_component_index(tron_id);
        let mut component = components_guard
            .get_mut(&component_id)
            .unwrap()
            .blocking_write();
        component.set_value(v);
    }

    /// Synchronously sets the state for a component by its `tron_id`
    pub fn set_state_for_component_blocking(&self, tron_id: &str, s: TnComponentState) {
        let context_guard = self.blocking_read();
        let mut components_guard = context_guard.components.blocking_write();
        let component_id = context_guard.get_component_index(tron_id);
        let mut component = components_guard
            .get_mut(&component_id)
            .unwrap()
            .blocking_write();
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
                server_side_trigger_data: TnServerSideTriggerData {
                    target: tron_id.into(),
                    new_state: "ready".into(),
                },
            };
            send_sse_msg_to_client(&sse_tx, msg).await;
        }
    }

    pub async fn get_service_tx(&self, service_id: &str) -> Sender<TnServiceRequestMsg> {
        let context_guard = self
        .read()
        .await;
        context_guard.services
        .get(service_id)
        .unwrap().0.clone()
    }


    pub async fn get_asset_ref(&self) -> Arc<RwLock<HashMap<String, TnAsset>>> {
        let context_guard = self
        .write()
        .await;
        context_guard.asset.clone()
    }
}

/// Implements the default trait for creating a default instance of `TnContextBase<'a>`.
impl<'a: 'static> Default for TnContextBase<'a>
where
    'a: 'static,
{
     fn default() -> Self {
        Self::new()
    }
}


pub trait TnComponentBaseTrait<'a: 'static>: Send + Sync {
    fn id(&self) -> TnComponentIndex;
    fn tron_id(&self) -> &TnComponentId;
    fn get_type(&self) -> TnComponentType;

    fn attributes(&self) -> &TnElmAttributes;
    fn set_attribute(&mut self, key: String, value: String);
    fn remove_attribute(&mut self, key: String);
    fn generate_attr_string(&self) -> String;

    fn extra_headers(&self) -> &TnExtraResponseHeader;
    fn set_header(&mut self, key: String, value: (String, bool));
    fn remove_header(&mut self, key: String);
    fn clear_header(&mut self);

    fn value(&self) -> &TnComponentValue;
    fn get_mut_value(&mut self) -> &mut TnComponentValue;
    fn set_value(&mut self, value: TnComponentValue);

    fn state(&self) -> &TnComponentState;
    fn set_state(&mut self, state: TnComponentState);

    fn create_assets(&mut self);
    fn get_assets(&self) -> Option<&HashMap<String, TnAsset>>;
    fn get_mut_assets(&mut self) -> Option<&mut HashMap<String, TnAsset>>;

    fn first_render(&self) -> String;
    fn render(&self) -> String;

    fn get_children(&self) -> &Vec<TnComponent<'a>>;
    fn get_mut_children(&mut self) -> &mut Vec<TnComponent<'a>>;
    fn add_child(&mut self, child: TnComponent<'a>);

    fn add_parent(&mut self, parent: TnComponent<'a>);
    fn get_parent(&self) -> TnComponent<'a>;

    fn get_script(&self) -> Option<String>;
}

/// Trait representing the base functionalities of a Tron component.
///
/// This trait defines methods for accessing and manipulating various properties of a Tron component,
/// such as its ID, type, attributes, headers, value, state, assets, children, parent, and script.
/// Implementors of this trait must provide concrete implementations for these methods.
impl<'a: 'static> TnComponentBase<'a> {
   
    pub fn new(
        tag: String,
        index: TnComponentIndex,
        tron_id: TnComponentId,
        type_: TnComponentType,
    ) -> Self {
        let mut attributes = HashMap::<String, String>::default();
        attributes.insert("id".into(), tron_id.clone());
        attributes.insert("hx-post".to_string(), format!("/tron/{}", index));
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
            id: index,
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

/// Provides an implementation of `TnComponentBaseTrait` for `TnComponentBase`.
///
/// This trait implementation includes methods to access and manipulate component properties.
/// Methods include:
/// - `id()`: Returns the component's ID.
/// - `tron_id()`: Returns a reference to the component's Tron ID.
/// - `get_type()`: Returns the component's type.
/// - `attributes()`: Returns a reference to the component's attributes.
/// - `set_attribute(key, val)`: Sets an attribute for the component.
/// - `remove_attribute(key)`: Removes an attribute from the component.
/// - `extra_headers()`: Returns a reference to the component's extra response headers.
/// - `set_header(key, val)`: Sets a response header for the component.
/// - `remove_header(key)`: Removes a response header.
/// - `clear_header()`: Clears all response headers.
/// - `generate_attr_string()`: Generates a string of HTML attributes for the component.
/// - `value()`: Returns a reference to the component's value.
/// - `get_mut_value()`: Returns a mutable reference to the component's value.
/// - `set_value(new_value)`: Sets the component's value.
/// - `set_state(new_state)`: Sets the component's state and updates the state attribute.
/// - `state()`: Returns a reference to the component's state.
/// - `get_assets()`: Returns an optional reference to the component's assets.
/// - `get_mut_assets()`: Returns an optional mutable reference to the component's assets.
/// - `get_children()`: Returns a reference to the component's child components.
/// - `get_mut_children()`: Returns a mutable reference to the component's child components.
/// - `add_child(child)`: Adds a child to the component.
/// - `add_parent(parent)`: Sets the parent for the component.
/// - `get_parent()`: Returns the component's parent.
/// - `first_render()`: First render logic (unimplemented).
/// - `render()`: Render logic (unimplemented).
/// - `get_script()`: Returns an optional clone of the component's script.
impl<'a: 'static> TnComponentBaseTrait<'a> for TnComponentBase<'a>
where
    'a: 'static,
{
    fn id(&self) -> u32 {
        self.id
    }

    fn tron_id(&self) -> &TnComponentId {
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

    fn set_header(&mut self, key: String, val: (String, bool)) {
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

    fn create_assets(&mut self) {
        self.asset = Some(HashMap::default());
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

    fn get_mut_children(&mut self) -> &mut Vec<TnComponent<'a>> {
        &mut self.children
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

/// Represents a Tron event.
///
/// This struct encapsulates various properties of a Tron event, including its trigger, type, state, target, and header target.
///
#[derive(Eq, PartialEq, Hash, Clone, Debug, Deserialize, Default)]
pub struct TnEvent {
    pub e_trigger: String,
    pub e_type: String,  // maybe use Enum
    pub e_state: String, // should use the Component::State enum
    #[allow(dead_code)]
    #[serde(default)]
    pub e_target: String,
    #[allow(dead_code)]
    #[serde(default)]
    pub h_target: Option<String>,
}
use tokio::sync::{mpsc::Sender, RwLock};
use tron_utils::{send_sse_msg_to_client, TnServerSideTriggerData, TnSseTriggerMsg};

/// Represents an HTML response along with its headers, wrapped in an optional tuple.

pub type TnHtmlResponse = Option<(HeaderMap, Html<String>)>;

/// Represents an asynchronous action function that processes Tron events.
///
/// This type defines a function signature for processing Tron events asynchronously. It takes a
/// `TnContext`, a `TnEvent`, and a payload of type `Value` as input parameters and returns a future
/// wrapping the HTML response along with its headers, represented by `TnHtmlResponse`.
pub type TnActionFn = fn(
    TnContext,
    event: TnEvent,
    payload: Value,
)
    -> Pin<Box<dyn futures_util::Future<Output = TnHtmlResponse> + Send + Sync>>;

/// Represents the method of execution for actions associated with Tron events.
#[derive(Clone)]
pub enum TnActionExecutionMethod {
    /// Spawn the action asynchronously.
    Spawn,
    /// Await the action's completion.
    Await,
}


/// Represents a collection of event actions associated with Tron component indices.
///
/// This type defines a hashmap where each key is a `TnComponentIndex`, and each value is a tuple
/// containing an `ActionExecutionMethod` and an `Arc` wrapped around a `TnActionFn`. It is used to store
/// and manage event actions associated with Tron components.
pub type TnEventActions = HashMap<TnComponentIndex, (TnActionExecutionMethod, Arc<TnActionFn>)>;
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
