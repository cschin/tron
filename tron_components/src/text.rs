use super::*;
use tron_macro::*;
use tron_utils::{send_sse_msg_to_client, TnSseTriggerMsg, TnServerSideTriggerData};

/// Represents a TextArea component.
#[derive(ComponentBase)]
pub struct TnTextArea<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl<'a: 'static> TnTextArea<'a> {
    /// Creates a new TextArea component with the specified ID, name, and value.
    pub fn new(id: TnComponentIndex, name: String, value: String) -> Self {
        let mut base = TnComponentBase::new("textarea".into(), id, name, TnComponentType::TextArea);
        base.set_value(TnComponentValue::String(value));
        base.set_attribute("disabled".into(), "".into());
        base.set_attribute("hx-trigger".into(), "server_side_trigger".into());
        base.set_attribute("type".into(), "text".into());

        Self { base }
    }
}

impl<'a: 'static> Default for TnTextArea<'a> {
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

impl<'a: 'static> TnTextArea<'a> {
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
    context: TnContext,
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
pub async fn clean_textarea_with_context(context: TnContext, tron_id: &str) {
    update_and_send_textarea_with_context(context, tron_id, "").await;
}


/// Represents a TextArea component with streaming updates.
#[derive(ComponentBase)]
pub struct TnStreamTextArea<'a: 'static> {
    base: TnComponentBase<'a>,
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
impl<'a: 'static> TnStreamTextArea<'a> {
    pub fn new(idx: TnComponentIndex, tnid: String, value: Vec<String>) -> Self {
        let mut base =
            TnComponentBase::new("textarea".into(), idx, tnid, TnComponentType::StreamTextArea);
        base.set_value(TnComponentValue::VecString(value));

        base.set_attribute("hx-trigger".into(), "server_side_trigger".into());
        base.set_attribute("type".into(), "text".into());
        //base.set_attribute("disabled".into(), "".into());
        base.set_attribute(
            "hx-swap".into(),
            "beforeend scroll:bottom focus-scroll:true ".into(),
        );

        Self { base }
    }
}

/// Implements the default trait for TnStreamTextArea.
///
/// This sets the default value for TnStreamTextArea as an empty vector.
impl<'a: 'static> Default for TnStreamTextArea<'a> {
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

    /// Renders the stream text area, showing only the last appended string.
    pub fn internal_render(&self) -> String {
        let empty = "".to_string();
        match self.value() {
            TnComponentValue::VecString(s) => s.last().unwrap_or(&empty).clone(),
            _ => "".into(),
        }
    }
}

/// Appends a new string to the stream text area component.
pub async fn append_stream_textarea(comp: TnComponent<'static>, new_str: &str) {
    let mut comp = comp.write().await;
    assert!(comp.get_type() == TnComponentType::StreamTextArea);
    if let TnComponentValue::VecString(v) = comp.get_mut_value() {
        v.push(new_str.to_string());
    }
}

/// Appends a new string to the stream text area component and sends a server-sent event (SSE) message to the client.
pub async fn append_and_send_stream_textarea_with_context(
    context: TnContext,
    tron_id: &str,
    new_str: &str,
) {
    tracing::debug!(target:"tron_app", "tron_id: {tron_id}, new_str: {new_str}");
    {
        let comp = context.get_component(tron_id).await;
        append_stream_textarea(comp, new_str).await;
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

/// Cleans the content of the stream text area component with the specified ID and sends a server-sent event (SSE) message to the client.
pub async fn clean_stream_textarea_with_context(context: TnContext, tron_id: &str) {
    let sse_tx = context.get_sse_tx().await;
    {
        // remove the transcript in the chatbox component, and sent the hx-reswap to innerHTML
        // once the server side trigger for an update, the content will be empty
        // the hx-reswap will be removed when there is new text in append_chatbox_value()
        assert!(
            context.get_component(tron_id).await.read().await.get_type()
                == TnComponentType::StreamTextArea
        );
        context
            .set_value_for_component(tron_id, TnComponentValue::VecString(vec![]))
            .await;
        let comp = context.get_component(tron_id).await;
        {
            let mut guard = comp.write().await;
            guard.set_state(TnComponentState::Ready);
            guard.set_header("hx-reswap".into(), ("innerHTML".into(), true));

            let msg = TnSseTriggerMsg {
                server_side_trigger_data: TnServerSideTriggerData {
                    target: tron_id.into(),
                    new_state: "ready".into(),
                },
            };

            send_sse_msg_to_client(&sse_tx, msg).await;
        }
    }
}

/// Represents a text input component.
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
impl<'a: 'static> TnTextInput<'a> {
    pub fn new(idx: TnComponentIndex, tnid: String, value: String) -> Self {
        let mut base = TnComponentBase::new("input".into(), idx, tnid, TnComponentType::TextInput);
        base.set_value(TnComponentValue::String(value.to_string()));

        base.set_attribute("hx-trigger".into(), "change, server_side_trigger".into());
        base.set_attribute("type".into(), "text".into());
        base.set_attribute(
            "hx-vals".into(),
            r##"js:{event_data:get_input_event(event)}"##.into(),
        ); //over-ride the default as we need the value of the input text
        base.set_attribute("hx-swap".into(), "outerHTML".into());

        Self { base }
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

impl<'a: 'static> TnTextInput<'a> {
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
