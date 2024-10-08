use super::*;
use tron_macro::*;
use tron_utils::{send_sse_msg_to_client, TnServerEventData, TnSseTriggerMsg};

/// Represents a TextArea component.
#[non_exhaustive]
#[derive(ComponentBase)]
pub struct TnDiv<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl TnDivBuilder<'static> {
    /// Creates a new Div component with the specified ID, name, and value.
    pub fn init(mut self, name: String, value: String) -> Self {
        self.base = TnComponentBase::builder(self.base)
            .init("div".into(), name, TnComponentType::Div)
            .set_value(TnComponentValue::String(value))
            .set_attr("disabled", "")
            .set_attr("hx-trigger", "server_event")
            .set_attr("type", "container")
            .build();

        self
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

#[async_trait]
impl<'a> TnComponentRenderTrait<'a> for TnDiv<'a>
where
    'a: 'static,
{
    /// Renders the Div component.
    async fn render(&self) -> String {
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
    async fn initial_render(&self) -> String {
        self.render().await
    }
    async fn pre_render(&mut self, _ctx: &TnContextBase) {}

    async fn post_render(&mut self, _ctx: &TnContextBase) {}
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

/// Set the value of a TextArea component with a given context and send the update event
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
                server_event_data: TnServerEventData {
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
