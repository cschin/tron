use super::*;
use tron_macro::*;
use tron_utils::{send_sse_msg_to_client, SseTriggerMsg, TriggerData};

#[derive(ComponentBase)]
pub struct TnTextArea<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl<'a: 'static> TnTextArea<'a> {
    pub fn new(id: TnComponentId, name: String, value: String) -> Self {
        let mut base =
            TnComponentBase::new("textarea".into(), id, name, TnComponentType::TextArea);
        base.set_value(TnComponentValue::String(value));
        base.set_attribute("contenteditable".into(), "true".into());

        base.set_attribute("hx-trigger".into(), "server_side_trigger".into());
        base.set_attribute("type".into(), "text".into());

        Self {
            base,
        }
    }
}

impl<'a: 'static> Default for TnTextArea<'a> {
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

    pub fn internal_first_render(&self) -> String {
        self.internal_render()
    }
}

pub async fn append_textarea_value(
    comp: TnComponent<'static>,
    new_str: &str,
    sep: Option<&str>,
) {
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

#[derive(ComponentBase)]
pub struct TnStreamTextArea<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl<'a: 'static> TnStreamTextArea<'a> {
    pub fn new(id: TnComponentId, name: String, value: Vec<String>) -> Self {
        let mut base =
            TnComponentBase::new("textarea".into(), id, name, TnComponentType::StreamTextArea);
        base.set_value(TnComponentValue::VecString(value));
        base.set_attribute("contenteditable".into(), "false".into());

        base.set_attribute("hx-trigger".into(), "server_side_trigger".into());
        base.set_attribute("type".into(), "text".into());
        base.set_attribute(
            "hx-swap".into(),
            "beforeend scroll:bottom focus-scroll:true ".into(),
        );

        Self {
            base,
        }
    }
}

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

impl<'a: 'static> TnStreamTextArea<'a> {
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

    pub fn internal_render(&self) -> String {
        let empty = "".to_string();
        match self.value() {
            TnComponentValue::VecString(s) => s.last().unwrap_or(&empty).clone(),
            _ => "".into(),
        }
    }
}

pub async fn append_stream_textarea_value(
    comp: TnComponent<'static>,
    new_str: &str,
) {
    let mut comp = comp.write().await;
    assert!(comp.get_type() == TnComponentType::StreamTextArea);
    if let TnComponentValue::VecString(v) = comp.get_mut_value() {
        v.push(new_str.to_string());
    }
}

pub async fn append_and_send_stream_textarea_with_context(
    context: TnContext,
    tron_id: &str,
    new_str: &str,
) {
    tracing::info!(target:"tron_app", "tron_id; {tron_id}, new_str: {new_str}");
    {
        let comp = context.get_component(tron_id).await;
        append_stream_textarea_value(comp, new_str).await;
        let sse_tx = context.get_sse_tx_with_context().await;
        let msg = SseTriggerMsg {
            server_side_trigger: TriggerData {
                target: "status".into(),
                new_state: "ready".into(),
            },
        };
        send_sse_msg_to_client(&sse_tx, msg).await;
    }
}

#[derive(ComponentBase)]
pub struct TnTextInput<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl<'a: 'static> TnTextInput<'a> {
    pub fn new(id: TnComponentId, name: String, value: String) -> Self {
        let mut base =
            TnComponentBase::new("input".into(), id, name, TnComponentType::TextInput);
        base.set_value(TnComponentValue::String(value.to_string()));
        base.set_attribute("contenteditable".into(), "true".into());

        base.set_attribute("hx-trigger".into(), "change, server_side_trigger".into());
        base.set_attribute("type".into(), "text".into());
        base.set_attribute(
            "hx-vals".into(),
            r##"js:{event_data:get_input_event(event)}"##.into(),
        ); //over-ride the default as we need the value of the input text
        base.set_attribute("hx-swap".into(), "none".into());

        Self {
            base,
        }
    }
}

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
    pub fn internal_render(&self) -> String {
        format!(
            r##"<{} {} value="{}">"##,
            self.base.tag,
            self.generate_attr_string(),
            match self.value() {
                TnComponentValue::String(s) => tron_utils::html_escape_double_quote(&s.clone()),
                _ => "".to_string()
            }
        )
    }

    pub fn internal_first_render(&self) -> String {
        self.internal_render()
    }
}
