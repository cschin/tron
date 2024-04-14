use super::*;
use tron_macro::*;

#[derive(ComponentBase)]
pub struct TnButton<'a> {
    inner: ComponentBase<'a>,
}

impl<'a> TnButton<'a> {
    pub fn new(id: ComponentId, name: String, value: String) -> Self {
        let mut component_base = ComponentBase::new("button".to_string(), id, name);
        component_base.value = ComponentValue::String(value);
        component_base
            .attributes
            .insert("hx-trigger".into(), "click, server_side_trigger".into());

        Self {
            inner: component_base,
        }
    }
}

impl<'a> Default for TnButton<'a> {
    fn default() -> Self {
        Self {
            inner: ComponentBase {
                value: ComponentValue::String("button".to_string()),
                ..Default::default()
            },
        }
    }
}

impl<'a> TnButton<'a> {
    pub fn internal_render(&self) -> Html<String> {
        Html::from(format!(
            r##"<{} {}>{}</{}>"##,
            self.inner.tag,
            self.generate_attr_string(),
            match self.value() {
                ComponentValue::String(s) => s,
                _ => "button",
            },
            self.inner.tag
        ))
    }
}
