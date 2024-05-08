pub mod audio_player;
pub mod audio_recorder;
pub mod button;
pub mod chatbox;
pub mod checklist;
pub mod radio_group;
pub mod range_slider;
pub mod select;
pub mod text;

pub use audio_player::TnAudioPlayer;
pub use audio_recorder::TnAudioRecorder;
pub use button::TnButton;
pub use chatbox::TnChatBox;
pub use checklist::{TnCheckBox, TnCheckList};
pub use radio_group::{TnRadioGroup, TnRadioItem};
pub use range_slider::TnRangeSlider;
pub use select::TnSelect;
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

/// For each component, we can store an value with type of enum `TnComponentValue`.
/// The variants are:
/// - `None`: Represents no value
/// - `String(String)`: Represents a single string value
/// - `VecString(Vec<String>)`: Represents a vector of string values
/// - `VecString2(Vec<(String, String)>`: Represents a vector of tuples where each tuple contains two
/// strings
/// - `CheckItems(HashMap<String, bool>)`: Represents a hashmap where keys are strings and values are
/// booleans
/// - `CheckItem(bool)`: Represents a single boolean
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

/// This `TnAsset` enum has several variants
/// representing different types of data that can be stored:
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

/// The `TnComponentType enum has predefined variants such as `Base`, `AudioPlayer`,
/// `AudioRecorder`, `Button`, `CheckList`, `CheckBox`, `TextArea`, `StreamTextArea`, `TextInput`,
/// `ChatBox`, `Select`, `Slider`, `RadioGroup`, and `RadioItem`. Additionally, it has a variant
/// `UserDefined` that takes a `String` parameter to represent user-defined component types. The enum
/// also derives implementations for `PartialEq`, `Eq`, `Debug`, and
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
    UserDefined(String),
}

/// `TnComponentState` represents different
/// states for a component in a Tron application. The enum has variants `Ready`, `Pending`, `Updating`,
/// `Finished`, and `Disabled`, each representing a different state of the component. These states
/// indicate whether the component is ready to receive new events, waiting for a server response,
/// updating based on server data, finished processing updates, or disabled and not interactive.
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum TnComponentState {
    Ready,    // ready to receive new event on the UI end
    Pending,  // UI event receive, server function dispatched, waiting for server to response
    Updating, // server ready to send, change UI to update to receive server response, can repeate
    Finished, // no more new update to consider, will transition to Ready
    Disabled, // disabled, not interactive and not sending htmx get/post
}

/// `TnComponentAsset`, used for storing addition data in each component, is an `Option` containing a
/// `HashMap` where the keys are of type `String` and the values are of type `TnAsset`.
type TnComponentAsset = Option<HashMap<String, TnAsset>>;

/// The `TnComponentBase` struct represents a base component with various properties and
/// relationships.
///
/// Properties:
///
/// * `tag`: The `tag` property in the `TnComponentBase` struct represents the element tag of the
/// component. It specifies the type of HTML element that this component represents.
/// * `type_`: The `type_` property in the `TnComponentBase` struct represents the type of the
/// component. It is of type `TnComponentType`.
/// * `id`: The `id` property in the `TnComponentBase` struct represents the unique identifier of the
/// component within the application. It is of type `TnComponentIndex`.
/// * `tron_id`: The `tron_id` property in the `TnComponentBase` struct represents the unique identifier
/// for the component within the system. It is of type `TnComponentId`.
/// * `attributes`: The `attributes` property in the `TnComponentBase` struct represents the attributes
/// associated with the component. These attributes are typically key-value pairs that provide
/// additional information or configuration for the component. For example, attributes could include
/// things like `class`, `style`, `data-*` attributes, or
/// * `value`: The `value` property in the `TnComponentBase` struct represents the value associated with
/// the component. It is of type `TnComponentValue`. This value could be any data or information that
/// the component needs to store or operate on.
/// * `extra_response_headers`: The `extra_response_headers` property in the `TnComponentBase` struct
/// likely represents additional headers that can be included in the response when this component is
/// rendered. These headers could be used for various purposes such as specifying caching directives,
/// content type, encoding, or any other custom headers required for the
/// * `asset`: The `asset` property in the `TnComponentBase` struct represents the asset associated with
/// the component. This could include information such as images, videos, or other resources that are
/// part of the component's content or functionality.
/// * `state`: The `state` property in the `TnComponentBase` struct represents the state of the
/// component. It holds information about the current state of the component, such as whether it is
/// active, disabled, hidden, or any other relevant state information that the component needs to manage
/// its behavior and appearance.
/// * `children`: The `children` property in the `TnComponentBase` struct is a vector of `TnComponent`
/// elements. It represents the child components of the current component. Each `TnComponent` element in
/// the vector holds information about a child component, allowing for a hierarchical structure of
/// components within
/// * `parent`: The `parent` property in the `TnComponentBase` struct is a weak reference to the parent
/// component of the current component. It is of type `Weak<RwLock<Box<dyn TnComponentBaseTrait<'a>>`,
/// which allows for a flexible reference to the parent component without creating a
/// * `script`: The `script` property in the `TnComponentBase` struct is an optional field that can hold
/// a `String` value. It allows you to store a script associated with the component if needed. If there
/// is no script associated with the component, the value of this field would be `None
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

/// The `TnServiceRequestMsg` struct represents a service request message with a request string, payload
/// of type `TnAsset`, and a response sender.
///
/// Properties:
///
/// * `request`: The `request` field in the `TnServiceRequestMsg` struct represents the type of request
/// being made. It is a `String` type field.
/// * `payload`: The `payload` property in the `TnServiceRequestMsg` struct represents an object of type
/// `TnAsset`. It contains the data or information associated with the service request message.
/// * `response`: The `response` field in the `TnServiceRequestMsg` struct is of type
/// `oneshot::Sender<String>`. This field is used to send a response back to the requester once the
/// service request has been processed. The `Sender` type is typically used in Rust for sending a single
#[derive(Debug)]
pub struct TnServiceRequestMsg {
    pub request: String,
    pub payload: TnAsset,
    pub response: oneshot::Sender<String>,
}
/// The `TnServiceResponseMsg` struct represents a response message with a string response and a payload
/// of type `TnAsset`.
///
/// Properties:
///
/// * `response`: The `response` property in the `TnServiceResponseMsg` struct is of type `String`. It
/// is used to store the response message from the service.
/// * `payload`: The `payload` property in the `TnServiceResponseMsg` struct is of type `TnAsset`. It
/// contains the data or information associated with the response message.

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

/// A type alias `TnStreamID` for the type `String`. This allows you
/// to refer to `String` as `TnStreamID` in your code for better readability and abstraction.
type TnStreamID = String;
/// `TnStreamProtocol`  is an alias for the existing type `String`.
type TnStreamProtocol = String;
/// A type alias `TnStreamData` for a `VecDeque` of `BytesMut` in Rust. This
/// allows you to refer to this specific type using the alias `TnStreamData` throughout your code,
/// making it more readable and easier to maintain.
type TnStreamData = VecDeque<BytesMut>;
/// `TnAssetName` is an alias for the existing type `String`.
type TnAssetName = String;
/// `TnComponentId` is an alias for the existing type `String`.
type TnComponentId = String;
/// `TnServiceName` is an alias for the existing type `String`.
type TnServiceName = String;

/// `TnComponentMap` is a type alias
/// representing an `Arc` (atomic reference counted smart pointer) wrapping a `RwLock` (read-write lock)
/// containing a `HashMap`. The `HashMap` is mapping `TnComponentIndex` keys to `TnComponent` values,
/// where `TnComponent` is a generic type parameterized by lifetime `'a`.
pub type TnComponentMap<'a> = Arc<RwLock<HashMap<TnComponentIndex, TnComponent<'a>>>>;

/// `TnStreamMap` is an alias for
/// `Arc<RwLock<HashMap<TnStreamID, (TnStreamProtocol, TnStreamData)>>>`. This type represents a shared,
/// thread-safe reference-counted smart pointer (`Arc`) to a read-write lock (`RwLock`) guarding a hash
/// map where the keys are of type `TnStreamID` and the values are tuples of type `(TnStreamProtocol,
/// TnStreamData)`. This construct is commonly used in Rust for concurrent access to shared data
/// structures
pub type TnStreamMap = Arc<RwLock<HashMap<TnStreamID, (TnStreamProtocol, TnStreamData)>>>;

/// `TnContextAsset` is a reference-counted,
/// thread-safe pointer to a `HashMap` containing key-value pairs of `TnAssetName` and `TnAsset`. This
/// type alias is defined using the `Arc` (atomic reference counting) and `RwLock` (read-write lock)
/// types to ensure safe concurrent access to the underlying data structure.
pub type TnContextAsset = Arc<RwLock<HashMap<TnAssetName, TnAsset>>>;

/// `TnSeeChannel` is an alias for
/// `Arc<RwLock<Option<TnSseMsgChannel>>>`. This type alias represents an atomic reference-counted smart
/// pointer `Arc` that provides shared ownership of a value, and a reader-writer lock `RwLock` that
/// allows multiple readers or one writer at a time, containing an optional value of type
/// `TnSseMsgChannel`.
pub type TnSeeChannel = Arc<RwLock<Option<TnSseMsgChannel>>>;

/// `TnService` is a tuple containing two elements:
/// 1. `Sender<TnServiceRequestMsg>`: This is a sender channel for sending messages of type
/// `TnServiceRequestMsg`.
/// 2. `Mutex<Option<Receiver<TnServiceResponseMsg>>>`: This is a mutex-wrapped optional receiver
/// channel for receiving messages of type `TnServiceResponseMsg`.
pub type TnService = (
    Sender<TnServiceRequestMsg>,
    Mutex<Option<Receiver<TnServiceResponseMsg>>>,
);
/// The `TnContextBase` contains various fields such as component maps, stream data,
/// assets, channels, mappings, and services.
/// 
/// Properties:
/// 
/// * `components`: The `components` field in the `TnContextBase` struct is a `TnComponentMap` that maps
/// component IDs to `Component` structs. This allows for easy access and management of components
/// within the context.
/// * `stream_data`: The `stream_data` property in the `TnContextBase` struct is a `TnStreamMap`, which
/// likely represents a mapping of some sort related to streaming data. It could be used to store and
/// manage streams of data within the context.
/// * `asset`: The `asset` property in the `TnContextBase` struct represents the asset associated with
/// the context. It likely contains information or data related to the assets being managed or utilized
/// within the context of the application or system.
/// * `sse_channel`: The `sse_channel` property in the `TnContextBase` struct represents a channel for
/// Server-Sent Events (SSE). This channel is likely used for sending real-time updates or notifications
/// from the server to the client over HTTP. It allows for asynchronous communication between the server
/// and the client without
/// * `tnid_to_index`: The `tnid_to_index` property is a HashMap that maps `TnComponentId` keys to
/// `TnComponentIndex` values. This mapping is used to associate a unique identifier for a component
/// (`TnComponentId`) with its corresponding index in the context (`TnComponentIndex`).
/// * `services`: The `services` property in the `TnContextBase` struct is a HashMap that maps
/// `TnServiceName` keys to `TnService` values. This allows for efficient lookup and storage of services
/// within the context.
pub struct TnContextBase<'a: 'static> {
    pub components: TnComponentMap<'a>, // component ID mapped to Component structs
    pub stream_data: TnStreamMap,
    pub asset: TnContextAsset,
    pub sse_channel: TnSeeChannel,
    pub tnid_to_index: HashMap<TnComponentId, TnComponentIndex>,
    pub services: HashMap<TnServiceName, TnService>,
}

impl<'a: 'static> TnContextBase<'a> {
/// The `new` function initializes a `TnContextBase` struct with default values for its fields.
/// 
/// Returns:
/// 
/// A new instance of the `TnContextBase` struct is being returned with all its fields initialized with
/// default values.
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

/// The `add_component` function in Rust adds a new component to a data structure while maintaining
/// thread safety.
/// 
/// Arguments:
/// 
/// * `new_component`: `new_component` is a parameter of type `impl TnComponentBaseTrait<'a> + 'static`,
/// which means it is a type that implements the `TnComponentBaseTrait` trait with a lifetime parameter
/// `'a` and also has a static lifetime.
    pub fn add_component(&mut self, new_component: impl TnComponentBaseTrait<'a> + 'static) {
        let tron_id = new_component.tron_id().clone();
        let id = new_component.id();
        let mut component_guard = self.components.blocking_write();
        component_guard.insert(id, Arc::new(RwLock::new(Box::new(new_component))));
        self.tnid_to_index.insert(tron_id, id);
    }

/// This Rust function retrieves the index of a component based on its tron_id.
/// 
/// Arguments:
/// 
/// * `tron_id`: The `tron_id` parameter is a reference to a string that represents the identifier of a
/// component.
/// 
/// Returns:
/// 
/// The function `get_component_index` returns a `u32` value, which is the index associated with the
/// provided `tron_id` in the `tnid_to_index` map. If the `tron_id` is not found in the map, it will
/// panic with a message indicating that the component with the given `tron_id` was not found.
    pub fn get_component_index(&self, tron_id: &str) -> u32 {
        *self
            .tnid_to_index
            .get(tron_id)
            .unwrap_or_else(|| panic!("component tron_id:{} not found", tron_id))
    }

/// The function `render_to_string` renders a component to a string based on a given ID.
/// 
/// Arguments:
/// 
/// * `tron_id`: The `tron_id` parameter is a reference to a string that is used to identify a component
/// within the data structure.
/// 
/// Returns:
/// 
/// The `render_to_string` method returns a `String` value, which is the result of calling the `render`
/// method on a component retrieved from the `components` data structure based on the provided
/// `tron_id`.
    pub fn render_to_string(&self, tron_id: &str) -> String {
        let id = self.get_component_index(tron_id);
        let components_guard = self.components.blocking_read();
        let component = components_guard.get(&id).unwrap().blocking_read();
        component.render()
    }

/// This Rust function `first_render_to_string` retrieves a component by ID and calls its `first_render`
/// method to return a string.
/// 
/// Arguments:
/// 
/// * `tron_id`: The `tron_id` parameter is a reference to a string that is used to identify a component
/// within the system.
/// 
/// Returns:
/// 
/// The `first_render()` method is being called on the `component` and the result of that method call is
/// being returned as a `String`.
    pub fn first_render_to_string(&self, tron_id: &str) -> String {
        let id = self.get_component_index(tron_id);
        let component_guard = self.components.blocking_read();
        let component = component_guard.get(&id).unwrap().blocking_read();
        component.first_render()
    }
}

/// The `TnContext` struct is an `Arc` wrapped `RwLock` of
/// `TnContextBase` with a static lifetime.
/// 
/// Properties:
/// 
/// * `base`: The `base` property in the `TnContext` struct is a field of type
/// `Arc<RwLock<TnContextBase<'static>>>`. This field holds a reference-counted smart pointer `Arc`
/// wrapping a reader-writer lock `RwLock` that contains a `Tn
#[derive(Clone)]
pub struct TnContext {
    pub base: Arc<RwLock<TnContextBase<'static>>>,
}

impl TnContext {
/// The `read` function in Rust asynchronously acquires a read lock on a RwLock and returns a guard to
/// the locked data.
/// 
/// Returns:
/// 
/// A `RwLockReadGuard` containing a reference to a `TnContextBase` instance with a static lifetime is
/// being returned.
    pub async fn read(&self) -> RwLockReadGuard<TnContextBase<'static>> {
        self.base.read().await
    }

/// The `write` function in Rust asynchronously acquires a write lock on a `TnContextBase` object.
/// 
/// Returns:
/// 
/// A `RwLockWriteGuard` containing a reference to a `TnContextBase` instance with a static lifetime is
/// being returned.
    pub async fn write(&self) -> RwLockWriteGuard<TnContextBase<'static>> {
        self.base.write().await
    }

/// The `blocking_read` function returns a read guard for a `TnContextBase` object in Rust.
/// 
/// Returns:
/// 
/// A `RwLockReadGuard` containing a reference to a `TnContextBase` instance with a static lifetime is
/// being returned.
    pub fn blocking_read(&self) -> RwLockReadGuard<TnContextBase<'static>> {
        self.base.blocking_read()
    }

/// The `blocking_write` function returns a `RwLockWriteGuard` for a `TnContextBase` object.
/// 
/// Returns:
/// 
/// A RwLockWriteGuard containing a TnContextBase<'static> instance is being returned.
    pub fn blocking_write(&self) -> RwLockWriteGuard<TnContextBase<'static>> {
        self.base.blocking_write()
    }

/// This Rust function asynchronously retrieves a component based on a given ID.
/// 
/// Arguments:
/// 
/// * `tron_id`: The `tron_id` parameter is a reference to a string that represents the identifier of a
/// component.
/// 
/// Returns:
/// 
/// The `get_component` function is returning a `TnComponent` object with a lifetime of `'static`.
    pub async fn get_component(&self, tron_id: &str) -> TnComponent<'static> {
        let context_guard = self.write().await;
        let components_guard = context_guard.components.write().await;
        let comp_id = context_guard.get_component_index(tron_id);
        components_guard.get(&comp_id).unwrap().clone()
    }

/// This Rust function retrieves a component associated with a given ID in a blocking manner.
/// 
/// Arguments:
/// 
/// * `tron_id`: The `tron_id` parameter is a reference to a string that represents the identifier of a
/// component in the context.
/// 
/// Returns:
/// 
/// The `blocking_get_component` function is returning a `TnComponent<'static>` object.
    pub fn blocking_get_component(&self, tron_id: &str) -> TnComponent<'static> {
        let context_guard = self.blocking_write();
        let components_guard = context_guard.components.blocking_write();
        let comp_id = context_guard.get_component_index(tron_id);
        components_guard.get(&comp_id).unwrap().clone()
    }

/// The `render_component` function in Rust asynchronously renders a component based on a given ID
/// within a context.
/// 
/// Arguments:
/// 
/// * `tron_id`: The `tron_id` parameter is a reference to a string that is used to identify a component
/// within the context.
/// 
/// Returns:
/// 
/// The `render_component` function returns a `String` value, which is the result of calling the
/// `render` method on the component referenced by the `comp` variable.
    pub async fn render_component(&self, tron_id: &str) -> String {
        let context_guard = self.read().await;
        let components_guard = context_guard.components.read().await;
        let comp_id = context_guard.get_component_index(tron_id);
        let comp = components_guard.get(&comp_id).unwrap().read().await;
        comp.render()
    }

/// The function `get_value_from_component` in Rust retrieves a value from a component identified by a
/// given ID in an asynchronous manner.
/// 
/// Arguments:
/// 
/// * `tron_id`: The `tron_id` parameter is a reference to a string that represents the ID of a
/// component in the context.
/// 
/// Returns:
/// 
/// The `get_value_from_component` function returns a `TnComponentValue` which is the value of a
/// component identified by the provided `tron_id`.
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

/// This Rust function asynchronously sets a value for a component identified by a given ID.
/// 
/// Arguments:
/// 
/// * `tron_id`: The `tron_id` parameter is a string that represents the identifier of a component in
/// the system.
/// * `v`: The parameter `v` in the `set_value_for_component` function represents the value that you
/// want to set for a specific component identified by the `tron_id`. It is of type `TnComponentValue`.
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

/// This Rust function sets the state for a component in a concurrent environment.
/// 
/// Arguments:
/// 
/// * `tron_id`: The `tron_id` parameter is a string that represents the identifier of a component in
/// the system.
/// * `s`: The parameter `s` in the `set_state_for_component` function is of type `TnComponentState`. It
/// is the state that you want to set for a specific component identified by the `tron_id`.
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

/// This Rust function sets a value for a component in a blocking manner.
/// 
/// Arguments:
/// 
/// * `tron_id`: The `tron_id` parameter is a string that represents the ID of a component in the
/// context.
/// * `v`: TnComponentValue
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

/// This Rust function sets the state for a component in a blocking manner.
/// 
/// Arguments:
/// 
/// * `tron_id`: The `tron_id` parameter is a string that represents the identifier of a component in
/// the system.
/// * `s`: TnComponentState
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

/// The function `get_sse_tx` returns a clone of the Sender from a nested RwLock structure after
/// unlocking two layers of RwLock.
/// 
/// Returns:
/// 
/// A `Sender<String>` is being returned from the `get_sse_tx` function.
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

/// The `set_ready_for` function in Rust sets a component to the ready state and triggers a server-side
/// event to update the button state for a specified ID.
/// 
/// Arguments:
/// 
/// * `tron_id`: The `tron_id` parameter in the `set_ready_for` function is a string that represents the
/// identifier of a TRON (presumably a component or entity). This identifier is used to set the state of
/// the specified TRON component to "ready" and trigger a server-side event to update the
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
}

impl<'a: 'static> Default for TnContextBase<'a>
where
    'a: 'static,
{
    /// The function `default` in Rust returns a new instance of the current type.
    /// 
    /// Returns:
    /// 
    /// The `default` function is returning an instance of the current struct or enum by calling the
    /// `new` associated function.
    fn default() -> Self {
        Self::new()
    }
}
/// `TnComponentBaseTrait` defines the traits with associated methods for managing
/// components in a UI framework. The trait includes functions for handling component attributes,
/// headers, values, state, assets, rendering, children, parent-child relationships, and scripts. The
/// trait is generic over a lifetime parameter `'a` and requires implementations to be `Send` and
/// `Sync`.

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

impl<'a: 'static> TnComponentBase<'a> {
    /// The function `new` in Rust creates a new instance of a struct with specified attributes and
    /// default values.
    /// 
    /// Arguments:
    /// 
    /// * `tag`: The `tag` parameter in the `new` function represents the HTML tag name of the component
    /// being created. It could be values like "div", "span", "button", etc., indicating the type of
    /// HTML element that will be created for this component.
    /// * `index`: The `index` parameter in the `new` function is of type `TnComponentIndex`. It is used
    /// to specify the index of the component.
    /// * `tron_id`: The `tron_id` parameter in the `new` function represents the unique identifier for
    /// a component in a web application. It is used to identify and target specific components for
    /// interaction or manipulation within the application.
    /// * `type_`: The `type_` parameter in the `new` function represents the type of the component
    /// being created. It is of type `TnComponentType`. This parameter is used to specify the type of
    /// the component being initialized within the function.
    /// 
    /// Returns:
    /// 
    /// A new instance of a struct with the specified tag, index, tron_id, and type, along with default
    /// attributes and other fields initialized.
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
// For Event Dispatcher
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
pub type TnHtmlResponse = Option<(HeaderMap, Html<String>)>;
pub type ActionFn = fn(
    TnContext,
    event: TnEvent,
    payload: Value,
)
    -> Pin<Box<dyn futures_util::Future<Output = TnHtmlResponse> + Send + Sync>>;

#[derive(Clone)]
pub enum ActionExecutionMethod {
    Spawn,
    Await,
}

pub type TnEventActions = HashMap<TnComponentIndex, (ActionExecutionMethod, Arc<ActionFn>)>;
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
