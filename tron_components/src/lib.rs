pub mod audio_recorder;
pub mod button;
pub mod text;

use std::{collections::HashMap, pin::Pin, sync::Arc};

pub use audio_recorder::TnAudioRecorder;
pub use button::TnButton;
pub use text::TnTextArea;

use rand::{thread_rng, Rng};

use axum::{body::Bytes, response::Html};

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
pub enum ComponentAsset {
    None,
    VecU8(Vec<u8>),
    String(String),
    Bytes(Bytes),
}

#[derive(Debug, Clone)]
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
    pub assets: Option<HashMap<String, ComponentAsset>>,
    pub state: ComponentState,
    pub children: Option<Vec<&'a ComponentBase<'a>>>, // general storage
}

// pub enum ComponentTypes<'a> {
//     TnButton(TnButton<'a>),
// }

// enum ComponentLayout {
//     Row(Vec<ComponentLayout>),
//     Col(Vec<ComponentLayout>),
//     Components(Vec<String>)
// }

pub struct Components<'a> {
    pub components: HashMap<u32, Box<dyn ComponentBaseTrait<'a>>>, // component ID mapped to Component structs
    pub assets: HashMap<String, Vec<u8>>,
    pub tron_id_to_id: HashMap<String, u32>,
}

impl<'a> Components<'a> {
    pub fn new() -> Self {
        Components {
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
    ) -> &(dyn ComponentBaseTrait<'a> + 'static) {
        let id = self
            .tron_id_to_id
            .get(tron_id)
            .unwrap_or_else(|| panic!("component tron_id:{} not found", tron_id));
        self.components.get(id).unwrap().as_ref()
    }

    pub fn get_mut_component_by_tron_id(
        &mut self,
        tron_id: &str,
    ) -> &mut Box<dyn ComponentBaseTrait<'a> + 'static> {
        let id = self.tron_id_to_id.get(tron_id).unwrap();
        self.components
            .get_mut(id)
            .unwrap_or_else(|| panic!("component tron_id:{} not found", tron_id))
    }

    pub fn render_to_string(&self, tron_id: &str) -> String {
        let component = self.get_component_by_tron_id(tron_id);
        component.render().0
    }
}

impl<'a> Default for Components<'a> {
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
    fn render_to_string(&self) -> String;
    fn get_children(&self) -> &Option<Vec<&'a ComponentBase<'a>>>;
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

    fn assets(&self) -> &Option<HashMap<String, ComponentAsset>> {
        &self.assets
    }

    fn render(&self) -> Html<String> {
        unimplemented!()
    }

    fn render_to_string(&self) -> String {
        self.render().0
    }

    fn get_children(&self) -> &Option<Vec<&'a ComponentBase<'a>>> {
        &self.children
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
    Arc<RwLock<Components<'static>>>,
    Sender<axum::Json<serde_json::Value>>,
    event: TnEvent,
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
