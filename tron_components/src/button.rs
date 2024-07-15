use super::*;
use tron_macro::*;

/// Represents a button component in a Tron application.
#[derive(ComponentBase)]
pub struct TnButton<'a: 'static> {
    base: TnComponentBase<'a>,
}

/// Creates a new button component with the specified idx, tnid, and value.
impl TnButton<'static> {
    pub fn new(idx: TnComponentIndex, tnid: String, value: String) -> Self {
        let mut base = TnComponentBase::new("button".into(), idx, tnid, TnComponentType::Button);
        base.set_value(TnComponentValue::String(value));
        base.set_attribute("hx-trigger".into(), "click, server_side_trigger".into());

        Self { base }
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

impl TnButton<'static>
{
    /// Generates the internal HTML representation of the button component.
    pub fn internal_render(&self) -> String {
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
    pub fn internal_first_render(&self) -> String {
        self.internal_render()
    }

    pub fn internal_pre_render(&mut self) {}

    pub fn internal_post_render(&mut self) {}
}
