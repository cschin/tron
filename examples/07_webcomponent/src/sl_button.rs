use axum::async_trait;
use std::collections::HashMap;
use tron_app::tron_components::*;
use tron_app::tron_macro::*;

/// Represents a button component in a Tron application.
#[non_exhaustive]
#[derive(ComponentBase)]
pub struct SLButton<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl<'a: 'static> SLButtonBuilder<'a> {
    pub fn init(mut self, tnid: String, value: String) -> Self {
        let component_type = TnComponentType::UserDefined("sl-button".into());
        self.base = TnComponentBase::builder(self.base)
            .init("sl-button".into(), tnid, component_type)
            .set_value(TnComponentValue::String(value))
            .set_attr("hx-trigger", "click, server_event")
            .build();
        self
    }
}

#[async_trait]
impl<'a> TnComponentRenderTrait<'a> for SLButton<'a>
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
    async fn initial_render(&self) -> String {
        self.render().await
    }

    async fn pre_render(&mut self) {}

    async fn post_render(&mut self) {}
}
