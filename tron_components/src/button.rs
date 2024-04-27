use super::*;
use tron_macro::*;

#[derive(ComponentBase)]
pub struct TnButton<'a: 'static> {
    inner: ComponentBase<'a>,
}

impl<'a: 'static> TnButton<'a> {
    pub fn new(id: ComponentId, name: String, value: String) -> Self {
        let mut component_base =
            ComponentBase::new("button".into(), id, name, TnComponentType::Button);
        component_base.set_value(ComponentValue::String(value));
        component_base.set_attribute("hx-trigger".into(), "click, server_side_trigger".into());

        Self {
            inner: component_base,
        }
    }
}

impl<'a: 'static> Default for TnButton<'a> {
    fn default() -> Self {
        Self {
            inner: ComponentBase {
                value: ComponentValue::String("button".to_string()),
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
            self.inner.tag,
            self.generate_attr_string(),
            match self.value() {
                ComponentValue::String(s) => s,
                _ => "button",
            },
            self.inner.tag
        )
    }

    pub fn internal_first_render(&self) -> String {
        self.internal_render()
    }
}
