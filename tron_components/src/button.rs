use super::*;
use tron_macro::*;

#[derive(ComponentBase)]
pub struct TnButton<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl<'a: 'static> TnButton<'a> {
    pub fn new(id: TnComponentId, name: String, value: String) -> Self {
        let mut base = TnComponentBase::new("button".into(), id, name, TnComponentType::Button);
        base.set_value(TnComponentValue::String(value));
        base.set_attribute("hx-trigger".into(), "click, server_side_trigger".into());

        Self { base }
    }
}

impl<'a: 'static> Default for TnButton<'a> {
    fn default() -> Self {
        Self {
            base: TnComponentBase {
                value: TnComponentValue::String("button".to_string()),
                ..Default::default()
            },
        }
    }
}

impl<'a: 'static> TnButton<'a>
where
    'a: 'static,
{
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

    pub fn internal_first_render(&self) -> String {
        self.internal_render()
    }
}
