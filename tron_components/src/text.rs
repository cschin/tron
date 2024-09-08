use super::*;
use serde::Serialize;
use tron_macro::*;
use tron_utils::{send_sse_msg_to_client, TnServerSideTriggerData, TnSseTriggerMsg};

/// Represents a TextArea component.
#[non_exhaustive]
#[derive(ComponentBase)]
pub struct TnTextArea<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl TnTextAreaBuilder<'static> {
    /// Creates a new TextArea component with the specified ID, name, and value.
    pub fn init(mut self, id: TnComponentIndex, name: String, value: String) -> Self {
        self.base
            .init("textarea".into(), id, name, TnComponentType::TextArea);
        self.base.set_value(TnComponentValue::String(value));
        self.base.set_attribute("disabled".into(), "".into());
        self.base
            .set_attribute("hx-trigger".into(), "server_side_trigger".into());
        self.base.set_attribute("type".into(), "text".into());
        self
    }
}

impl Default for TnTextArea<'static> {
    /// Creates a default TextArea component with an empty value.
    fn default() -> Self {
        Self {
            base: TnComponentBase {
                value: TnComponentValue::String("".into()),
                ..Default::default()
            },
        }
    }
}

impl TnTextArea<'static> {
    /// Renders the TextArea component.
    pub fn internal_render(&self) -> String {
        format!(
            r##"<{} {}>{}</{}>"##,
            self.base.tag,
            self.generate_attr_string(),
            match self.value() {
                TnComponentValue::String(s) => &s,
                _ => "textarea",
            },
            self.base.tag
        )
    }

    /// Renders the TextArea component for the first time.
    pub fn internal_first_render(&self) -> String {
        self.internal_render()
    }
    pub fn internal_pre_render(&mut self) {}

    pub fn internal_post_render(&mut self) {}
}

/// Appends text to the value of a TextArea component.
pub async fn append_textarea_value(comp: TnComponent<'static>, new_str: &str, sep: Option<&str>) {
    let v;
    {
        let comp = comp.read().await;
        assert!(comp.get_type() == TnComponentType::TextArea);
        let v0 = match comp.value() {
            TnComponentValue::String(s) => s.clone(),
            _ => "".into(),
        };
        v = [v0, new_str.to_string()];
    }
    {
        let mut comp = comp.write().await;
        let sep = sep.unwrap_or("");
        comp.set_value(TnComponentValue::String(v.join(sep)));
    }
}

/// Appends text to the value of a TextArea component.
pub async fn append_textarea_value_with_context(
    context: TnContext,
    tron_id: &str,
    new_str: &str,
    sep: Option<&str>,
) {
    let comp = context.get_component(tron_id).await;
    append_textarea_value(comp, new_str, sep).await;
}

/// Appends text to the value of a TextArea component with a given context.
pub async fn update_and_send_textarea_with_context(
    context: &TnContext,
    tron_id: &str,
    new_str: &str,
) {
    {
        let comp = context.get_component(tron_id).await;
        {
            assert!(comp.read().await.get_type() == TnComponentType::TextArea);
        }
        {
            let mut guard = comp.write().await;
            guard.set_value(TnComponentValue::String(new_str.to_string()));
            guard.set_state(TnComponentState::Ready);
            let sse_tx = context.get_sse_tx().await;
            let msg = TnSseTriggerMsg {
                server_side_trigger_data: TnServerSideTriggerData {
                    target: tron_id.to_string(),
                    new_state: "ready".into(),
                },
            };
            send_sse_msg_to_client(&sse_tx, msg).await;
        }
    }
}

/// Cleans the value of a TextArea component within a given context.
pub async fn clean_textarea_with_context(context: &TnContext, tron_id: &str) {
    update_and_send_textarea_with_context(context, tron_id, "").await;
}

/// Represents a TextArea component with streaming updates.
#[non_exhaustive]
#[derive(ComponentBase)]
pub struct TnStreamTextArea<'a: 'static> {
    base: TnComponentBase<'a>,
}

#[derive(Serialize)]
pub struct SseStreamTextAreaTriggerMsg {
    pub server_side_trigger_data: TnServerSideTriggerData,
    pub stream_textarea_control: String,
    pub payload: String,
}

/// Creates a new instance of TnStreamTextArea.
///
/// # Arguments
///
/// * `idx` - The ID of the component.
/// * `tnid` - The name of the component.
/// * `value` - The initial value of the textarea.
///
/// # Returns
///
/// A new instance of TnStreamTextArea.
impl TnStreamTextAreaBuilder<'static> {
    pub fn init(mut self, idx: TnComponentIndex, tnid: String, value: Vec<String>) -> Self {
        let component_type = TnComponentType::StreamTextArea;
        TnComponentType::register_script(
            component_type.clone(),
            include_str!("../javascript/stream_textarea.html"),
        );
        self.base.init("textarea".into(), idx, tnid, component_type);
        self.base.set_value(TnComponentValue::VecString(value));
        // stream textarea is totally passive!!
        self.base.remove_attribute("hx-trigger".into());
        self.base.remove_attribute("hx-swap".into());
        self.base.remove_attribute("hx-post".into());
        self.base.remove_attribute("hx-target".into());
        self.base.remove_attribute("hx-vals".into());
        self.base.remove_attribute("hx-ext".into());
        self.base.set_attribute("type".into(), "text".into());
        self.base.set_attribute("disabled".into(), "".into());
        self
    }
}

/// Implements the default trait for TnStreamTextArea.
///
/// This sets the default value for TnStreamTextArea as an empty vector.
impl Default for TnStreamTextArea<'static> {
    fn default() -> Self {
        Self {
            base: TnComponentBase {
                value: TnComponentValue::VecString(vec![]),
                ..Default::default()
            },
        }
    }
}

/// Implements internal rendering functions for TnStreamTextArea.
impl<'a: 'static> TnStreamTextArea<'a> {
    /// Implements internal rendering functions for TnStreamTextArea.
    pub fn internal_first_render(&self) -> String {
        self.internal_render()
    }

    /// Renders the stream text area, showing only the last appended string.
    pub fn internal_render(&self) -> String {
        format!(
            r##"<{} {}>{}</{}>"##,
            self.base.tag,
            self.generate_attr_string(),
            match self.value() {
                TnComponentValue::VecString(s) => s.join(""),
                _ => "".to_string(),
            },
            self.base.tag
        )
    }
    pub fn internal_pre_render(&mut self) {}

    pub fn internal_post_render(&mut self) {}
}

/// Appends a new string to the stream text area component but not rendering it.
pub async fn append_stream_textarea(comp: TnComponent<'static>, new_str: &str) {
    let mut comp = comp.write().await;
    assert!(comp.get_type() == TnComponentType::StreamTextArea);
    if let TnComponentValue::VecString(v) = comp.get_mut_value() {
        v.push(new_str.to_string());
    }
}

/// Appends a new string to the stream text area component and sends a server-sent event (SSE) message to the client.
pub async fn append_and_update_stream_textarea_with_context(
    context: &TnContext,
    tron_id: &str,
    new_str: &str,
) {
    tracing::debug!(target:"tron_app", "tron_id: {tron_id}, new_str: {new_str}");
    let component = context.get_component(tron_id).await;
    append_stream_textarea(component, new_str).await;
    update_all_stream_textarea_with_context(context, tron_id).await;
}

/// Appends a new string to the stream text area component and sends a server-sent event (SSE) message to the client.
pub async fn update_all_stream_textarea_with_context(context: &TnContext, tron_id: &str) {
    tracing::debug!(target:"tron_app", "tron_id: {tron_id}");
    {
        let comp = context.get_component(tron_id).await;

        let rest = {
            let mut value = comp.write().await;
            let value = value.get_mut_value();
            if let TnComponentValue::VecString(value) = value {
                let rest = value.join("");
                value.clear();
                rest
            } else {
                "".into()
            }
        };

        let sse_tx = context.get_sse_tx().await;

        let msg = SseStreamTextAreaTriggerMsg {
            server_side_trigger_data: TnServerSideTriggerData {
                target: tron_id.to_string(),
                new_state: "ready".into(),
            },
            stream_textarea_control: "append_text".into(),
            payload: rest,
        };
        send_sse_msg_to_client(&sse_tx, msg).await;
    }
}

/// Cleans the content of the stream text area component with the specified ID and sends a server-sent event (SSE) message to the client.
pub async fn clean_stream_textarea_with_context(context: &TnContext, tron_id: &str) {
    assert!(
        context.get_component(tron_id).await.read().await.get_type()
            == TnComponentType::StreamTextArea
    );

    let component = context.get_component(tron_id).await;
    let mut guard = component.write().await;
    let value = guard.get_mut_value();
    if let TnComponentValue::VecString(value) = value {
        value.clear();
    }

    let sse_tx = context.get_sse_tx().await;

    let msg = SseStreamTextAreaTriggerMsg {
        server_side_trigger_data: TnServerSideTriggerData {
            target: tron_id.to_string(),
            new_state: "ready".into(),
        },
        stream_textarea_control: "update_text".into(),
        payload: "".into(),
    };
    send_sse_msg_to_client(&sse_tx, msg).await;
}

/// Represents a text input component.
#[non_exhaustive]
#[derive(ComponentBase)]
pub struct TnTextInput<'a: 'static> {
    base: TnComponentBase<'a>,
}

/// Creates a new text input component.
///
/// # Arguments
///
/// * `idx` - The unique index of the component.
/// * `tnid` - The name or identifier of the component.
/// * `value` - The initial value of the text input.
///
/// # Returns
///
/// A new `TnTextInput` instance.
impl TnTextInputBuilder<'static> {
    pub fn init(mut self, idx: TnComponentIndex, tnid: String, value: String) -> Self {
        self.base
            .init("input".into(), idx, tnid, TnComponentType::TextInput);
        self.base
            .set_value(TnComponentValue::String(value.to_string()));

        self.base
            .set_attribute("hx-trigger".into(), "change, server_side_trigger".into());
        self.base.set_attribute("type".into(), "text".into());
        self.base.set_attribute(
            "hx-vals".into(),
            r##"js:{event_data:get_input_event(event)}"##.into(),
        ); //over-ride the default as we need the value of the input text
        self.base
            .set_attribute("hx-swap".into(), "outerHTML".into());
        self
    }
}

/// Implements the default trait for `TnTextInput`, providing a default instance.
///
/// The default value for the text input is set to "input".
impl<'a: 'static> Default for TnTextInput<'a> {
    fn default() -> Self {
        Self {
            base: TnComponentBase {
                value: TnComponentValue::String("input".to_string()),
                ..Default::default()
            },
        }
    }
}

impl TnTextInput<'static> {
    /// Renders the internal representation of the text input component.
    pub fn internal_render(&self) -> String {
        format!(
            r##"<{} {} value="{}">"##,
            self.base.tag,
            self.generate_attr_string(),
            match self.value() {
                TnComponentValue::String(s) => tron_utils::html_escape_double_quote(&s.clone()),
                _ => "".to_string(),
            }
        )
    }
    /// Renders the initial representation of the text input component.
    pub fn internal_first_render(&self) -> String {
        self.internal_render()
    }

    pub fn internal_pre_render(&mut self) {}

    pub fn internal_post_render(&mut self) {}
}

/// Cleans the text input component with the given context and Tron ID.
///
/// # Arguments
///
/// * `context` - The context containing the text input component.
/// * `tron_id` - The Tron ID of the text input component to clean.
///
pub async fn clean_textinput_with_context(context: TnContext, tron_id: &str) {
    {
        let comp = context.get_component(tron_id).await;
        {
            assert!(comp.read().await.get_type() == TnComponentType::TextInput);
        }
        {
            let mut guard = comp.write().await;
            guard.set_value(TnComponentValue::String(String::default()));
            guard.set_state(TnComponentState::Ready);
            let sse_tx = context.get_sse_tx().await;
            let msg = TnSseTriggerMsg {
                server_side_trigger_data: TnServerSideTriggerData {
                    target: tron_id.to_string(),
                    new_state: "ready".into(),
                },
            };
            send_sse_msg_to_client(&sse_tx, msg).await;
        }
    }
}
