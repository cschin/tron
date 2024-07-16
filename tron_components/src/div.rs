use super::*;
use tron_macro::*;
use tron_utils::{send_sse_msg_to_client, TnServerSideTriggerData, TnSseTriggerMsg};

/// Represents a TextArea component.
#[derive(ComponentBase)]
pub struct TnDiv<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl TnDiv<'static> {
    /// Creates a new TextArea component with the specified ID, name, and value.
    pub fn new(id: TnComponentIndex, name: String, value: String) -> Self {
        let mut base = TnComponentBase::new("div".into(), id, name, TnComponentType::Div);
        base.set_value(TnComponentValue::String(value));
        base.set_attribute("disabled".into(), "".into());
        base.set_attribute("hx-trigger".into(), "server_side_trigger".into());
        base.set_attribute("type".into(), "container".into());

        Self { base }
    }
}

impl Default for TnDiv<'static> {
    /// Creates a default component with an empty value.
    fn default() -> Self {
        Self {
            base: TnComponentBase {
                value: TnComponentValue::String("".into()),
                ..Default::default()
            },
        }
    }
}

impl TnDiv<'static> {
    /// Renders the Div component.
    pub fn internal_render(&self) -> String {
        format!(
            r##"<{} {}>{}</{}>"##,
            self.base.tag,
            self.generate_attr_string(),
            match self.value() {
                TnComponentValue::String(s) => &s,
                _ => "container",
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
pub async fn append_div_value(comp: TnComponent<'static>, new_str: &str, sep: Option<&str>) {
    let v;
    {
        let comp = comp.read().await;
        assert!(comp.get_type() == TnComponentType::Div);
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
pub async fn append_in_div_with_context(
    context: TnContext,
    tron_id: &str,
    new_str: &str,
    sep: Option<&str>,
) {
    let comp = context.get_component(tron_id).await;
    append_div_value(comp, new_str, sep).await;
}

/// Appends text to the value of a TextArea component with a given context.
pub async fn update_and_send_div_with_context(context: &TnContext, tron_id: &str, new_str: &str) {
    {
        let comp = context.get_component(tron_id).await;
        {
            assert!(comp.read().await.get_type() == TnComponentType::Div);
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
pub async fn clean_div_with_context(context: &TnContext, tron_id: &str) {
    update_and_send_div_with_context(context, tron_id, "").await;
}
