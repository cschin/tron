use super::*;
use tron_macro::*;

/// Represents a button component in a Tron application.
#[non_exhaustive]
#[derive(ComponentBase)]
pub struct TnButton<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl<'a: 'static> TnButtonBuilder<'a> {
    pub fn init(mut self, tnid: String, value: String) -> Self {
        self.base = TnComponentBase::builder(self.base)
            .init("button".into(), tnid, TnComponentType::Button)
            .set_value(TnComponentValue::String(value))
            .set_attribute("hx-trigger", "click, server_event")
            .build();
        self
    }
}

impl Default for TnButton<'static> {
    /// Creates a default instance of `TnButton`.
    fn default() -> Self {
        Self {
            base: TnComponentBase {
                value: TnComponentValue::String("button".to_string()),
                ..Default::default()
            },
        }
    }
}

#[async_trait]
impl<'a> TnComponentRenderTrait<'a> for TnButton<'a>
where
    'a: 'static,
{
    /// Generates the internal HTML representation of the button component.
    async fn render(&self) -> String {
        format!(
            r##"<{} {}>{}</{}>"##,
            self.base.tag,
            self.generate_attr_string(),
            match self.value() {
                TnComponentValue::String(s) => &s,
                _ => "button",
            },
            self.base.tag
        )
    }

    /// Generates the initial HTML representation of the button component.
    async fn first_render(&self) -> String {
        self.render().await
    }

    async fn pre_render(&mut self) {}

    async fn post_render(&mut self) {}
}
